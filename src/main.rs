mod build;
mod http;
mod model;
mod sync;
mod update;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{env, fs::OpenOptions, io::Write, path::PathBuf};

#[derive(Parser)]
#[command(version, about)]
struct Arguments {
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Build,
    Sync {
        #[arg(long)]
        allow_removals: bool,
    },
    Update {
        #[arg(long)]
        allow_removals: bool,
    },
    PublishNeeded {
        url: String,
    },
}

fn run() -> Result<()> {
    let arguments = Arguments::parse();
    env::set_current_dir(arguments.root).context("Cannot open project directory")?;
    match arguments.command {
        Command::Build => build::build()?,
        Command::Sync { allow_removals } => {
            sync::sync(allow_removals)?;
        }
        Command::Update { allow_removals } => update::update(allow_removals)?,
        Command::PublishNeeded { url } => {
            let local: build::Manifest = model::read_json("_site/version.json")?;
            let published = http::get(&url, "application/json")
                .and_then(|body| Ok(serde_json::from_str::<build::Manifest>(&body)?));
            let needed = match published {
                Ok(manifest) => manifest.fingerprint != local.fingerprint,
                Err(error) => {
                    eprintln!("Publication manifest unavailable; will publish: {error:#}");
                    true
                }
            };
            println!("publish={needed}");
            if let Some(path) = env::var_os("GITHUB_OUTPUT") {
                writeln!(
                    OpenOptions::new().append(true).create(true).open(path)?,
                    "publish={needed}"
                )?;
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
