use anyhow::{Context, Result, ensure};
use chrono::DateTime;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};

pub const DATA: &str = "data/dictionary.json";
pub const STATUS: &str = "data/sync-status.json";
pub const BASE_URL: &str = "https://www.health.harvard.edu/";
pub const SECTIONS: [(&str, &str); 4] = [
    ("a-through-c", "ABC"),
    ("d-through-i", "DEFGHI"),
    ("j-through-p", "JKLMNOP"),
    ("q-through-z", "QRSTUVWXYZ"),
];

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub updated_at: String,
    pub groups: Vec<Group>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub letter: char,
    pub source: String,
    pub terms: Vec<Term>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Term {
    pub term: String,
    pub definitions: Vec<String>,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
pub struct Changes {
    pub added: usize,
    pub edited: usize,
    pub removed: usize,
}

#[derive(Deserialize, Serialize)]
pub struct SyncStatus {
    pub last_attempt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success: Option<String>,
    pub entry_count: usize,
    #[serde(flatten)]
    pub outcome: SyncOutcome,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "result", rename_all = "lowercase")]
pub enum SyncOutcome {
    Changed { changes: Changes },
    Unchanged { changes: Changes },
    Failed { error: String },
}

impl Snapshot {
    pub fn load() -> Result<Self> {
        let snapshot: Self = read_json(DATA)?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<()> {
        DateTime::parse_from_rfc3339(&self.updated_at)
            .context("Invalid content synchronization date")?;
        ensure!(
            self.groups.iter().map(|group| group.letter).eq('A'..='Z'),
            "Dictionary must contain exactly the ordered letter groups A–Z"
        );
        for group in &self.groups {
            let slug = SECTIONS
                .iter()
                .find(|(_, letters)| letters.contains(group.letter))
                .unwrap()
                .0;
            ensure!(
                group.source == format!("{BASE_URL}{slug}#{}-terms", group.letter),
                "Invalid source for letter {}",
                group.letter
            );
            ensure!(
                !group.terms.is_empty(),
                "Missing terms for letter {}",
                group.letter
            );
            let mut names = HashSet::new();
            for term in &group.terms {
                ensure!(
                    !term.term.trim().is_empty() && names.insert(&term.term),
                    "Empty or duplicate term in letter {}",
                    group.letter
                );
                let definitions: HashSet<_> = term.definitions.iter().collect();
                ensure!(
                    !definitions.is_empty()
                        && definitions.len() == term.definitions.len()
                        && definitions
                            .iter()
                            .all(|definition| !definition.trim().is_empty()),
                    "Empty or duplicate definition for {}",
                    term.term
                );
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.groups.iter().map(|group| group.terms.len()).sum()
    }

    pub fn changes_from(&self, previous: &Self) -> Changes {
        let before = previous.index();
        let after = self.index();
        Changes {
            added: after.keys().filter(|key| !before.contains_key(key)).count(),
            removed: before.keys().filter(|key| !after.contains_key(key)).count(),
            edited: after
                .iter()
                .filter(|(key, value)| before.get(key).is_some_and(|old| old != *value))
                .count(),
        }
    }

    fn index(&self) -> BTreeMap<(char, &str), (&str, &Term)> {
        self.groups
            .iter()
            .flat_map(|group| {
                group.terms.iter().map(move |term| {
                    (
                        (group.letter, term.term.as_str()),
                        (group.source.as_str(), term),
                    )
                })
            })
            .collect()
    }
}

pub fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    serde_json::from_slice(
        &fs::read(path).with_context(|| format!("Cannot read {}", path.display()))?,
    )
    .with_context(|| format!("Invalid JSON in {}", path.display()))
}

pub fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    let path = path.as_ref();
    let temporary = path.with_extension("tmp");
    fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
    fs::write(
        &temporary,
        format!("{}\n", serde_json::to_string_pretty(value)?),
    )?;
    fs::rename(&temporary, path).with_context(|| format!("Cannot replace {}", path.display()))
}
