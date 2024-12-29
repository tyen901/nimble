use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;
mod md5_digest;
mod mod_cache;
mod pbo;
mod repository;
mod srf;
mod ui;
mod config;

#[derive(Subcommand)]
enum Commands {
    Sync {
        #[clap(short, long)]
        repo_url: String,

        #[clap(short, long)]
        path: PathBuf,

        #[clap(short, long)]
        dry_run: bool,
    },
    GenSrf {
        #[clap(short, long)]
        path: PathBuf,
    },
    Launch {
        #[clap(short, long)]
        path: PathBuf,
    },
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

fn main() -> Result<(), eframe::Error> {
    ui::run_ui()
}
