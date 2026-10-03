import argparse
import json
import os
import re
import string
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

from bs4 import BeautifulSoup


ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data" / "dictionary.json"
STATUS = ROOT / "data" / "sync-status.json"
BASE_URL = "https://www.health.harvard.edu/"
SECTIONS = {
    "a-through-c": "ABC",
    "d-through-i": "DEFGHI",
    "j-through-p": "JKLMNOP",
    "q-through-z": "QRSTUVWXYZ",
}


def clean(text):
    return re.sub(r"\s+", " ", text).strip()


def fetch(url):
    request = Request(url, headers={"User-Agent": "HarvardHealthDictionary/1.0 (+https://github.com/KnlnKS/harvard-med-dict-of-health-terms)"})
    for attempt in range(3):
        try:
            with urlopen(request, timeout=25) as response:
                if response.status != 200 or "text/html" not in response.headers.get("Content-Type", ""):
                    raise ValueError(f"Unexpected response from {url}")
                return response.read().decode("utf-8")
        except HTTPError as error:
            if error.code not in (408, 429, 500, 502, 503, 504) or attempt == 2:
                raise
        except (URLError, TimeoutError):
            if attempt == 2:
                raise
        time.sleep(2 ** attempt)


def parse(markup, slug, letters):
    content = BeautifulSoup(markup, "html.parser").select_one(".content-repository-content")
    if content is None:
        raise ValueError(f"Missing dictionary content in {slug}")
    entries = []
    headings = []
    letter = None
    for node in content.find_all(recursive=False):
        if node.name == "h2":
            letter = clean(node.get_text())
            if letter not in letters or len(letter) != 1:
                raise ValueError(f"Unexpected letter heading in {slug}: {letter}")
            headings.append(letter)
        elif node.name == "p" and letter:
            if node.get_text(strip=True).startswith("Browse dictionary by letter:"):
                continue
            term = None
            fragments = []

            def finish():
                definition = clean("".join(fragments))
                if not term or not definition:
                    raise ValueError(f"Missing term or definition in {slug}, letter {letter}")
                entries.append({"letter": letter, "term": term, "definition": definition, "source": f"{BASE_URL}{slug}#{letter}-terms"})

            for child in node.children:
                text = child.get_text() if hasattr(child, "get_text") else str(child)
                if getattr(child, "name", None) == "strong" and clean(text):
                    label = clean(text)
                    if not label.endswith(":"):
                        raise ValueError(f"Unexpected term label in {slug}: {label}")
                    if term is not None:
                        finish()
                    term = label[:-1].strip()
                    fragments = []
                else:
                    if term is None and clean(text):
                        raise ValueError(f"Unrecognized dictionary paragraph in {slug}")
                    fragments.append(text)
            if term is not None:
                finish()
            elif clean(node.get_text()):
                raise ValueError(f"Unrecognized dictionary paragraph in {slug}")
        elif letter and node.name not in ("a", "hr"):
            raise ValueError(f"Unexpected dictionary element in {slug}: {node.name}")
    if "".join(headings) != letters or set(entry["letter"] for entry in entries) != set(letters):
        raise ValueError(f"Incomplete letter groups in {slug}")
    return entries


def combine(entries):
    grouped = {}
    for entry in entries:
        key = (entry["letter"], entry["term"])
        if key not in grouped:
            grouped[key] = {"letter": entry["letter"], "term": entry["term"], "definitions": [], "source": entry["source"]}
        if entry["definition"] not in grouped[key]["definitions"]:
            grouped[key]["definitions"].append(entry["definition"])
    return sorted(grouped.values(), key=lambda entry: (entry["letter"], entry["term"].casefold(), entry["term"]))


def validate(entries, previous, allow_removals=False):
    if {entry["letter"] for entry in entries} != set(string.ascii_uppercase):
        raise ValueError("The dictionary must contain every letter A–Z")
    if previous and not allow_removals:
        for label, letters in [("dictionary", string.ascii_uppercase), *SECTIONS.items()]:
            before = sum(entry["letter"] in letters for entry in previous)
            after = sum(entry["letter"] in letters for entry in entries)
            if after < before * 0.9:
                raise ValueError(f"Entry count dropped by more than 10% in {label}: {before} → {after}. Verify the source before using --allow-removals.")


def changes(previous, entries):
    before = {(entry["letter"], entry["term"]): entry for entry in previous}
    after = {(entry["letter"], entry["term"]): entry for entry in entries}
    return {
        "added": len(after.keys() - before.keys()),
        "edited": sum(before[key] != after[key] for key in before.keys() & after.keys()),
        "removed": len(before.keys() - after.keys()),
    }


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def sync(allow_removals=False):
    now = datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
    previous = json.loads(DATA.read_text(encoding="utf-8")) if DATA.exists() else {"entries": []}
    status = json.loads(STATUS.read_text(encoding="utf-8")) if STATUS.exists() else {}
    status["last_attempt"] = now
    changed = False
    summary = {"added": 0, "edited": 0, "removed": 0}
    try:
        entries = combine(entry for slug, letters in SECTIONS.items() for entry in parse(fetch(BASE_URL + slug), slug, letters))
        validate(entries, previous["entries"], allow_removals)
        summary = changes(previous["entries"], entries)
        changed = previous["entries"] != entries
        if changed:
            write_json(DATA, {"updated_at": now, "entries": entries})
        status.update({"last_success": now, "result": "changed" if changed else "unchanged", "entry_count": len(entries), "changes": summary})
        status.pop("error", None)
        print(f"{status['result']}: {len(entries)} terms (+{summary['added']} / ~{summary['edited']} / -{summary['removed']})")
    except Exception as error:
        status.update({"result": "failed", "error": str(error)})
        raise
    finally:
        write_json(STATUS, status)
        if output := os.environ.get("GITHUB_OUTPUT"):
            with open(output, "a", encoding="utf-8") as handle:
                handle.write(f"changed={str(changed).lower()}\n")
                for key, count in summary.items():
                    handle.write(f"{key}={count}\n")
    return changed


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--allow-removals", action="store_true")
    arguments = parser.parse_args()
    try:
        sync(arguments.allow_removals)
    except Exception as error:
        print(f"Synchronization failed: {error}", file=sys.stderr)
        sys.exit(1)
