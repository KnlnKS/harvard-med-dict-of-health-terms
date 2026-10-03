use crate::{
    http,
    model::{
        self, BASE_URL, Changes, DATA, Group, SECTIONS, STATUS, Snapshot, SyncOutcome, SyncStatus,
        Term,
    },
};
use anyhow::{Context, Result, bail, ensure};
use chrono::{SecondsFormat, Utc};
use scraper::{ElementRef, Html, Selector};
use std::{collections::BTreeMap, path::Path, thread};

fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn definitions(paragraph: ElementRef<'_>) -> Result<Vec<Term>> {
    let mut terms: Vec<Term> = Vec::new();
    let mut definition = String::new();
    for child in paragraph.children() {
        let element = ElementRef::wrap(child);
        let text = match element {
            Some(element) => element.text().collect::<String>(),
            None => child
                .value()
                .as_text()
                .map(|text| text.to_string())
                .unwrap_or_default(),
        };
        let label = clean(&text);
        if element.is_some_and(|element| element.value().name() == "strong") && !label.is_empty() {
            let name = label
                .strip_suffix(':')
                .context("Unexpected bold text in dictionary paragraph")?
                .trim();
            ensure!(!name.is_empty(), "Empty term label");
            if let Some(term) = terms.last_mut() {
                term.definitions.push(clean(&definition));
            }
            definition.clear();
            terms.push(Term {
                term: name.to_owned(),
                definitions: Vec::new(),
            });
        } else {
            ensure!(
                !terms.is_empty() || label.is_empty(),
                "Unrecognized text before a term label"
            );
            definition.push_str(&text);
        }
    }
    if let Some(term) = terms.last_mut() {
        term.definitions.push(clean(&definition));
    }
    ensure!(
        terms.iter().all(|term| !term.definitions[0].is_empty()),
        "Missing definition"
    );
    ensure!(
        !terms.is_empty() || clean(&paragraph.text().collect::<String>()).is_empty(),
        "Unrecognized dictionary paragraph"
    );
    Ok(terms)
}

fn parse(markup: &str, slug: &str, letters: &str) -> Result<Vec<Group>> {
    let document = Html::parse_document(markup);
    let selector = Selector::parse(".content-repository-content").unwrap();
    let content = document
        .select(&selector)
        .next()
        .context("Missing dictionary content")?;
    let mut groups: Vec<Group> = Vec::new();
    for element in content.child_elements() {
        match element.value().name() {
            "h2" => {
                let text = clean(&element.text().collect::<String>());
                let letter = text.chars().next().context("Empty letter heading")?;
                ensure!(
                    text.len() == 1 && letters.contains(letter),
                    "Unexpected letter heading: {text}"
                );
                groups.push(Group {
                    letter,
                    source: format!("{BASE_URL}{slug}#{letter}-terms"),
                    terms: Vec::new(),
                });
            }
            "p" if !groups.is_empty() => {
                if !clean(&element.text().collect::<String>())
                    .starts_with("Browse dictionary by letter:")
                {
                    groups
                        .last_mut()
                        .unwrap()
                        .terms
                        .extend(definitions(element)?);
                }
            }
            "a" | "hr" => {}
            _ if groups.is_empty() => {}
            name => bail!("Unexpected dictionary element: {name}"),
        }
    }
    ensure!(
        groups.iter().map(|group| group.letter).collect::<String>() == letters,
        "Incomplete letter groups in {slug}"
    );
    for group in &mut groups {
        let mut terms: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for term in std::mem::take(&mut group.terms) {
            let definitions = terms.entry(term.term).or_default();
            for definition in term.definitions {
                if !definitions.contains(&definition) {
                    definitions.push(definition);
                }
            }
        }
        group.terms = terms
            .into_iter()
            .map(|(term, definitions)| Term { term, definitions })
            .collect();
        group
            .terms
            .sort_by_cached_key(|term| (term.term.to_lowercase(), term.term.clone()));
    }
    Ok(groups)
}

fn fetch_groups() -> Result<Vec<Group>> {
    thread::scope(|scope| {
        let requests: Vec<_> = SECTIONS
            .iter()
            .map(|&(slug, letters)| {
                scope.spawn(move || {
                    parse(
                        &http::get(&format!("{BASE_URL}{slug}"), "text/html")?,
                        slug,
                        letters,
                    )
                    .with_context(|| format!("Cannot import {slug}"))
                })
            })
            .collect();
        let mut groups = Vec::new();
        for request in requests {
            groups.extend(
                request
                    .join()
                    .map_err(|_| anyhow::anyhow!("Dictionary fetch worker failed"))??,
            );
        }
        Ok(groups)
    })
}

fn validate_removals(snapshot: &Snapshot, previous: &Snapshot, allow_removals: bool) -> Result<()> {
    if allow_removals {
        return Ok(());
    }
    for (label, letters) in
        std::iter::once(("dictionary", "ABCDEFGHIJKLMNOPQRSTUVWXYZ")).chain(SECTIONS)
    {
        let count = |snapshot: &Snapshot| {
            snapshot
                .groups
                .iter()
                .filter(|group| letters.contains(group.letter))
                .map(|group| group.terms.len())
                .sum::<usize>()
        };
        let before = count(previous);
        let after = count(snapshot);
        ensure!(
            after * 10 >= before * 9,
            "Entry count dropped by more than 10% in {label}: {before} → {after}. Verify the source before using --allow-removals"
        );
    }
    Ok(())
}

pub fn sync(allow_removals: bool) -> Result<Changes> {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let last_success = if Path::new(STATUS).exists() {
        model::read_json::<SyncStatus>(STATUS)?.last_success
    } else {
        None
    };
    let mut status = SyncStatus {
        last_attempt: now.clone(),
        last_success,
        entry_count: 0,
        outcome: SyncOutcome::Failed {
            error: "Synchronization did not finish".to_owned(),
        },
    };
    let result = (|| -> Result<Changes> {
        let previous = Snapshot::load()?;
        status.entry_count = previous.len();
        let snapshot = Snapshot {
            updated_at: now.clone(),
            groups: fetch_groups()?,
        };
        snapshot.validate()?;
        validate_removals(&snapshot, &previous, allow_removals)?;
        let changes = snapshot.changes_from(&previous);
        let changed = snapshot.groups != previous.groups;
        if changed {
            model::write_json(DATA, &snapshot)?;
        }
        println!(
            "{}: {} terms (+{} / ~{} / -{})",
            if changed { "changed" } else { "unchanged" },
            snapshot.len(),
            changes.added,
            changes.edited,
            changes.removed
        );
        status.last_success = Some(now);
        status.entry_count = snapshot.len();
        status.outcome = if changed {
            SyncOutcome::Changed { changes }
        } else {
            SyncOutcome::Unchanged { changes }
        };
        Ok(changes)
    })();
    if let Err(error) = &result {
        status.outcome = SyncOutcome::Failed {
            error: format!("{error:#}"),
        };
    }
    model::write_json(STATUS, &status)?;
    result
}
