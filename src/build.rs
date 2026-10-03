use crate::model::{self, Snapshot};
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt::Write, fs};
use unicode_normalization::UnicodeNormalization;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub fingerprint: String,
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn anchor(letter: char, term: &str) -> String {
    let ascii: String = term
        .nfkd()
        .filter(char::is_ascii)
        .flat_map(char::to_lowercase)
        .collect();
    let slug = ascii
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let digest = format!("{:x}", Sha256::digest(format!("{letter}:{term}")));
    format!("term-{slug}-{}", &digest[..8])
}

fn render(snapshot: &Snapshot) -> Result<String> {
    let mut navigation = String::new();
    let mut sections = String::new();
    for group in &snapshot.groups {
        let letter = group.letter;
        let count = group.terms.len();
        write!(
            navigation,
            "<a href=\"#{letter}\" aria-label=\"{letter}, {count} terms\"><span>{letter}</span><span class=\"letter-count\">{count}</span></a>"
        )?;
        write!(
            sections,
            "<section class=\"letter-section\" id=\"{letter}\" aria-labelledby=\"heading-{letter}\"><div class=\"section-heading\"><h2 id=\"heading-{letter}\">{letter}<span class=\"sr-only\"> terms</span></h2><div class=\"section-meta\"><span class=\"section-count\">{count} terms</span><a href=\"{}\" class=\"source-link\">Harvard source <span aria-hidden=\"true\">↗</span></a></div></div><dl>",
            escape(&group.source)
        )?;
        for term in &group.terms {
            let identifier = anchor(letter, &term.term);
            write!(
                sections,
                "<div class=\"entry\" id=\"{identifier}\"><dt><a href=\"#{identifier}\">{}</a></dt><dd>",
                escape(&term.term)
            )?;
            for definition in &term.definitions {
                if term.definitions.len() > 1 {
                    sections.push_str("<p>");
                }
                sections.push_str(&escape(definition));
                if term.definitions.len() > 1 {
                    sections.push_str("</p>");
                }
            }
            sections.push_str("</dd></div>");
        }
        sections.push_str("</dl></section>");
    }
    let updated = DateTime::parse_from_rfc3339(&snapshot.updated_at)?;
    let date = format!(
        "{} {}, {}",
        updated.format("%B"),
        updated.day(),
        updated.year()
    );
    let digits = snapshot.len().to_string();
    let count: String = digits
        .chars()
        .enumerate()
        .flat_map(|(index, digit)| {
            let separator = index > 0 && (digits.len() - index).is_multiple_of(3);
            separator.then_some(',').into_iter().chain([digit])
        })
        .collect();
    let values = [
        ("navigation", navigation),
        ("sections", sections),
        ("count", count),
        ("date", date),
        ("iso_date", updated.format("%Y-%m-%d").to_string()),
    ];
    let template = fs::read_to_string("site/index.html").context("Cannot read site template")?;
    let mut html = String::with_capacity(template.len() + values[1].1.len());
    let mut rest = template.as_str();
    while let Some(start) = rest.find("{{") {
        html.push_str(&rest[..start]);
        let end = rest[start..]
            .find("}}")
            .context("Unclosed template placeholder")?
            + start;
        let key = &rest[start + 2..end];
        let value = values
            .iter()
            .find(|(name, _)| *name == key)
            .with_context(|| format!("Unknown template placeholder: {key}"))?;
        html.push_str(&value.1);
        rest = &rest[end + 2..];
    }
    html.push_str(rest);
    Ok(html)
}

pub fn build() -> Result<()> {
    let snapshot = Snapshot::load()?;
    let html = render(&snapshot)?;
    let mut files = vec![("index.html", html.into_bytes())];
    for name in ["style.css", "search.js", "favicon.svg"] {
        files.push((name, fs::read(format!("site/{name}"))?));
    }
    files.push((".nojekyll", Vec::new()));
    let mut hash = Sha256::new();
    for (name, bytes) in &files {
        hash.update(name.as_bytes());
        hash.update([0]);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    fs::create_dir_all("_site")?;
    for (name, bytes) in files {
        fs::write(format!("_site/{name}"), bytes)?;
    }
    model::write_json(
        "_site/version.json",
        &Manifest {
            fingerprint: format!("{:x}", hash.finalize()),
        },
    )?;
    println!("Built {} terms → _site", snapshot.len());
    Ok(())
}
