# Health Terms

A complete, single-page medical dictionary with definitions from Harvard Health Publishing. Browse all 26 letters, search terms and definitions, or read the entire dictionary without JavaScript.

**[Open the dictionary](https://knlnks.github.io/harvard-med-dict-of-health-terms/)** · [Original Harvard dictionary](https://www.health.harvard.edu/medical-dictionary-of-health-terms)

The website uses vanilla HTML, CSS, and a small search script. There are no external fonts, runtime data requests, or frontend dependencies. Python builds the HTML from a committed dictionary snapshot. Beautiful Soup is used only when importing Harvard's four source pages.

## Local development

Python 3.13 or later is recommended. Building the committed snapshot requires only the Python standard library:

```sh
python3 scripts/build.py
python3 -m http.server 8000 --directory _site
```

Open [localhost:8000](http://localhost:8000). Edit `site/index.html`, `site/style.css`, or `site/search.js`, rebuild, and refresh. Relative asset URLs work under GitHub Pages project paths.

To refresh the dictionary:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
.venv/bin/python scripts/sync.py
python3 scripts/build.py
```

## Daily synchronization

The GitHub Actions workflow runs every day at **09:17 UTC** (02:17 Pacific daylight time, 01:17 Pacific standard time). GitHub can delay scheduled runs. You can also run **Sync dictionary and publish** manually from the Actions tab.

Each synchronization fetches these four pages:

- [A–C](https://www.health.harvard.edu/a-through-c)
- [D–I](https://www.health.harvard.edu/d-through-i)
- [J–P](https://www.health.harvard.edu/j-through-p)
- [Q–Z](https://www.health.harvard.edu/q-through-z)

The importer preserves source letter groupings, splits multiple labels within one paragraph, and collects distinct definitions for repeated terms. Additions, edits, and removals are reflected in `data/dictionary.json`. That snapshot changes only when content changes; its timestamp records synchronization, not medical review.

All four pages and all 26 nonempty letter groups must parse before replacing the snapshot. A count decrease greater than 10% overall or within a source section also stops the import. HTTP requests have a 25-second timeout and up to three attempts for temporary errors. Failed imports preserve the previous snapshot and published site, record the failure in `data/sync-status.json`, and fail the workflow visibly.

Every synchronization attempt commits a heartbeat with its outcome, even when the dictionary is unchanged. This maintains repository activity so GitHub's [60-day scheduled-workflow inactivity limit](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/disable-and-enable-workflows) does not stop daily updates. Failed runs that cannot start the importer or push a commit still require attention in Actions.

The site rebuilds and deploys after dictionary changes or pushes affecting the website, builder, or workflow. Unchanged heartbeats do not redeploy. Workflow runs are serialized, and bot commits do not recursively trigger another run.

If Harvard legitimately removes a large number of terms, inspect the source and the failed run first. Then enable **allow_removals** during a manual workflow run, or use:

```sh
.venv/bin/python scripts/sync.py --allow-removals
```

This bypasses only the count-decrease safeguard; completeness and parsing validation remain enabled.

## Hosting and validation

The public repository publishes to GitHub Pages through GitHub Actions. In repository **Settings → Pages**, the publishing source must be **GitHub Actions**. The workflow needs permission to write contents for synchronization commits. It uses separate, limited Pages deployment permissions.

Validation is manual; the repository contains no test files or code comments. Check 320px, 390px, tablet, and desktop layouts; keyboard navigation; letter and term anchors; case-insensitive multiword search; no-result and clear states; and JavaScript-disabled browsing. Use Chrome DevTools for network, layout, accessibility, and performance inspection. Check dictionary counts against the original source after importer changes.

## Attribution

Definitions © Harvard Health Publishing. This is an independent reference and is not affiliated with Harvard University. The crimson color is drawn from Harvard Health's palette. The small `h.` mark is this project's own mark.

This dictionary is for understanding health terminology and is not a substitute for medical advice from a qualified clinician.
