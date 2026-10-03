import hashlib
import json
import re
import shutil
import string
import unicodedata
from collections import Counter
from datetime import datetime
from html import escape
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "_site"


def anchor(entry):
    text = unicodedata.normalize("NFKD", entry["term"]).encode("ascii", "ignore").decode().lower()
    slug = re.sub(r"[^a-z0-9]+", "-", text).strip("-")
    digest = hashlib.sha256(f"{entry['letter']}:{entry['term']}".encode()).hexdigest()[:8]
    return f"term-{slug}-{digest}"


def build():
    snapshot = json.loads((ROOT / "data" / "dictionary.json").read_text(encoding="utf-8"))
    entries = snapshot["entries"]
    counts = Counter(entry["letter"] for entry in entries)
    updated = datetime.fromisoformat(snapshot["updated_at"].replace("Z", "+00:00"))
    date = f"{updated:%B} {updated.day}, {updated.year}"
    navigation = "".join(
        f'<a href="#{letter}" aria-label="{letter}, {counts[letter]} terms"><span>{letter}</span><span class="letter-count">{counts[letter]}</span></a>'
        for letter in string.ascii_uppercase
    )
    sections = []
    for letter in string.ascii_uppercase:
        terms = []
        for entry in entries:
            if entry["letter"] != letter:
                continue
            definitions = escape(entry["definitions"][0]) if len(entry["definitions"]) == 1 else "".join(f"<p>{escape(definition)}</p>" for definition in entry["definitions"])
            identifier = anchor(entry)
            terms.append(f'<div class="entry" id="{identifier}"><dt><a href="#{identifier}">{escape(entry["term"])}</a></dt><dd>{definitions}</dd></div>')
        source = next(entry["source"] for entry in entries if entry["letter"] == letter)
        sections.append(
            f'<section class="letter-section" id="{letter}" aria-labelledby="heading-{letter}">'
            f'<div class="section-heading"><h2 id="heading-{letter}">{letter}<span class="sr-only"> terms</span></h2>'
            f'<div class="section-meta"><span class="section-count">{counts[letter]} terms</span><a href="{escape(source)}" class="source-link">Harvard source <span aria-hidden="true">↗</span></a></div></div>'
            f'<dl>{"".join(terms)}</dl></section>'
        )
    template = (ROOT / "site" / "index.html").read_text(encoding="utf-8")
    replacements = {
        "{{navigation}}": navigation,
        "{{sections}}": "".join(sections),
        "{{count}}": f"{len(entries):,}",
        "{{date}}": date,
        "{{iso_date}}": updated.date().isoformat(),
    }
    for placeholder, value in replacements.items():
        template = template.replace(placeholder, value)
    OUTPUT.mkdir(exist_ok=True)
    (OUTPUT / "index.html").write_text(template, encoding="utf-8")
    for filename in ("style.css", "search.js", "favicon.svg"):
        shutil.copyfile(ROOT / "site" / filename, OUTPUT / filename)
    (OUTPUT / ".nojekyll").touch()
    print(f"Built {len(entries):,} terms → {OUTPUT}")


if __name__ == "__main__":
    build()
