use crate::{
    model::{DATA, STATUS},
    sync,
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    env,
    process::{Command, Output},
};

fn git(arguments: &[&str]) -> Result<Output> {
    Command::new("git")
        .args(arguments)
        .output()
        .context("Cannot run git")
}

fn checked(arguments: &[&str]) -> Result<String> {
    let output = git(arguments)?;
    ensure!(
        output.status.success(),
        "git {} failed: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub fn update(allow_removals: bool) -> Result<()> {
    ensure!(
        env::var("GITHUB_ACTIONS").as_deref() == Ok("true"),
        "update is reserved for a clean GitHub Actions checkout; use sync locally"
    );
    ensure!(
        checked(&["status", "--porcelain"])?.is_empty(),
        "Refusing to update a dirty checkout"
    );
    checked(&["config", "user.name", "github-actions[bot]"])?;
    checked(&[
        "config",
        "user.email",
        "41898282+github-actions[bot]@users.noreply.github.com",
    ])?;
    for attempt in 1..=3 {
        checked(&["fetch", "origin", "main"])?;
        checked(&["reset", "--hard", "origin/main"])?;
        let result = sync::sync(allow_removals);
        checked(&["add", DATA, STATUS])?;
        let staged = git(&["diff", "--cached", "--quiet"])?;
        match staged.status.code() {
            Some(0) => {
                result?;
                return Ok(());
            }
            Some(1) => {}
            _ => bail!("Cannot inspect staged dictionary changes"),
        }
        let message = match &result {
            Ok(changes) if changes.added + changes.edited + changes.removed > 0 => format!(
                "data: sync Harvard dictionary (+{} / ~{} / -{})",
                changes.added, changes.edited, changes.removed
            ),
            _ => "chore: record daily dictionary sync heartbeat".to_owned(),
        };
        checked(&["commit", "-m", &message])?;
        let push = git(&["push", "origin", "HEAD:main"])?;
        if push.status.success() {
            result?;
            return Ok(());
        }
        checked(&["fetch", "origin", "main"])?;
        let parent = checked(&["rev-parse", "HEAD^"])?;
        let remote = checked(&["rev-parse", "origin/main"])?;
        ensure!(
            parent != remote && attempt < 3,
            "Cannot push dictionary update: {}",
            String::from_utf8_lossy(&push.stderr)
        );
        eprintln!(
            "main advanced during synchronization; retrying against the latest head ({attempt}/3)"
        );
    }
    bail!("Dictionary commit retries exhausted")
}
