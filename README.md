# Health Terms

A complete, single-page medical dictionary with definitions from Harvard Health Publishing. Browse all 26 letters, search terms and definitions, or read the entire dictionary without JavaScript.

**[Open the dictionary](https://knlnks.github.io/harvard-med-dict-of-health-terms/)** · [Original Harvard dictionary](https://www.health.harvard.edu/a-through-c)

The website uses vanilla HTML, CSS, and a small search script. There are no external fonts, runtime data requests, or frontend dependencies. A Rust CLI imports Harvard's four source pages and builds static HTML from the committed snapshot. The daily workflow downloads a pinned, checksum-verified Linux binary from this repository's [releases](https://github.com/KnlnKS/harvard-med-dict-of-health-terms/releases); it installs no build tools or language dependencies.

## Local development

Use Rust 1.88 or later:

```sh
cargo build --release --locked
target/release/health-dictionary build
```

Open `_site/index.html` in your browser, or serve `_site` with a static HTTP server. Edit `site/index.html`, `site/style.css`, or `site/search.js`, rebuild, and refresh. Relative asset URLs work under GitHub Pages project paths.

To refresh the dictionary:

```sh
target/release/health-dictionary sync
target/release/health-dictionary build
```

The CLI runs from the repository directory. Use `--root /path/to/repository` to select another checkout. `sync` writes the snapshot and heartbeat locally; `update` is reserved for clean GitHub Actions checkouts and also commits and pushes them.

## Daily synchronization

The GitHub Actions workflow runs every day at **09:17 UTC** (02:17 Pacific daylight time, 01:17 Pacific standard time). GitHub can delay scheduled runs. You can also run **Sync dictionary and publish** manually from the Actions tab.

Each synchronization fetches these four pages concurrently:

- [A–C](https://www.health.harvard.edu/a-through-c)
- [D–I](https://www.health.harvard.edu/d-through-i)
- [J–P](https://www.health.harvard.edu/j-through-p)
- [Q–Z](https://www.health.harvard.edu/q-through-z)

The importer preserves source letter groupings, splits multiple labels within one paragraph, and collects distinct definitions for repeated terms. Each letter owns its source URL and terms in `data/dictionary.json`. Additions, edits, and removals replace that snapshot only when content changes; its timestamp records synchronization, not medical review.

All four pages and all 26 nonempty letter groups must parse before replacing the snapshot. A count decrease greater than 10% overall or within a source section also stops the import. HTTP requests have a 25-second timeout and up to three attempts for temporary errors. Failed imports preserve the previous snapshot and published site, record the failure in `data/sync-status.json`, and fail the workflow visibly.

Every synchronization attempt commits a heartbeat with its outcome, even when the dictionary is unchanged. This maintains repository activity so GitHub's [60-day scheduled-workflow inactivity limit](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/disable-and-enable-workflows) does not stop daily updates. Failure records retain the last successful synchronization date and contain no stale change counts. Failures that prevent the CLI from starting or pushing a commit still require attention in Actions.

Workflow runs are serialized. The updater starts from the latest `main`; if another push arrives during synchronization, it retries from the new head up to three times and imports again. It preserves intervening commits and never force-pushes. Bot heartbeat commits do not recursively trigger another run.

If Harvard legitimately removes a large number of terms, inspect the source and the failed run first. Then enable **allow_removals** during a manual workflow run, or use:

```sh
target/release/health-dictionary sync --allow-removals
```

This bypasses only the count-decrease safeguard; completeness and parsing validation remain enabled.

## Publication and recovery

Each eligible workflow run builds the page and hashes the exact HTML and assets. It compares the resulting `_site/version.json` fingerprint with the manifest at the published site. Only differing, missing, or unavailable manifests trigger a Pages deployment. An unchanged heartbeat alone therefore causes no deployment.

The published manifest advances only with a successful deployment. If deployment fails after dictionary changes were committed, the next daily or manual run retries publication even when Harvard's content is unchanged. Website and publish-workflow pushes also use this comparison. The small native build runs to calculate the fingerprint; unchanged output is neither uploaded nor deployed.

To inspect the publication decision locally after building:

```sh
target/release/health-dictionary publish-needed https://knlnks.github.io/harvard-med-dict-of-health-terms/version.json
```

## CLI releases

`Cargo.lock` pins Rust dependencies. To publish a CLI update, bump the version in `Cargo.toml`, refresh the lockfile, build locally, and run formatting and lint checks:

```sh
cargo build --release --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
```

Commit and push the CLI changes, then tag that commit with its Cargo version, such as `v1.0.1`, and push the tag. **Release dictionary CLI** compiles an optimized, stripped Linux x86-64 binary on Ubuntu 22.04 and publishes `health-dictionary-linux-x86_64.tar.gz` and `SHA256SUMS`. Compilation happens once per release, rather than during daily synchronization. The archive contains one executable; it needs no Python or Rust installation on the runner.

Wait for the release to succeed before updating `TOOL_VERSION` in `.github/workflows/publish.yml`. Commit and push that pin change to activate the binary. Keep existing release assets immutable; publish a new version for corrections. The runner verifies the downloaded archive against its release checksum before extraction.

## Hosting and validation

The public repository publishes through GitHub's official Pages artifact workflow. In repository **Settings → Pages**, the publishing source must be **GitHub Actions**. Synchronization and release jobs have contents-write permission; the separate deployment job has only Pages-write and identity-token permissions.

Validation is manual; the repository contains no test files or code comments. Check 320px, 390px, tablet, and desktop layouts; keyboard navigation; letter and term anchors; case-insensitive multiword search; no-result and clear states; and JavaScript-disabled browsing. Use Chrome DevTools for network, layout, accessibility, and performance inspection.

After importer or builder changes, reconcile all four source sections, repeated terms, numeric-leading terms, and paragraphs containing multiple entries. In temporary directories, verify unchanged imports, failed requests, removal safeguards and their explicit override, publication retry after an unchanged import, and preservation of a concurrent push. Compare generated HTML and term anchors before migrating the data format.

## Attribution

Definitions © Harvard Health Publishing. This is an independent reference and is not affiliated with Harvard University. The crimson color is drawn from Harvard Health's palette. The small `h.` mark is this project's own mark.

This dictionary is for understanding health terminology and is not a substitute for medical advice from a qualified clinician.
