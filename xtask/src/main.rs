use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use xtask::{plan, publish};

#[derive(Parser)]
#[command(about = "Validate and publish the workspace's crates.io releases")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check published package contents, version bumps, and dependency order.
    Plan {
        #[arg(long)]
        manifest_path: Option<PathBuf>,
    },
    /// Revalidate the complete plan, then run cargo publish in dependency order.
    Publish {
        #[arg(long)]
        manifest_path: Option<PathBuf>,
        /// Run cargo publish --dry-run instead of uploading.
        #[arg(long)]
        dry_run: bool,
        /// Maximum seconds to wait for each uploaded version to reach the index.
        #[arg(long, default_value_t = 120, value_parser = clap::value_parser!(u64).range(1..))]
        wait_timeout: u64,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Plan { manifest_path } => plan::run(plan::Arg { manifest_path }),
        Command::Publish {
            manifest_path,
            dry_run,
            wait_timeout,
        } => publish::run(publish::Arg {
            manifest_path,
            dry_run,
            wait_timeout: std::time::Duration::from_secs(wait_timeout),
        }),
    }
}
