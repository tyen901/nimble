use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;
mod md5_digest;
mod mod_cache;
mod pbo;
mod repository;
mod srf;

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

fn main() {
    let args = Args::parse();

    let mut agent = ureq::AgentBuilder::new()
        .user_agent("nimble (like Swifty)/0.1")
        .build();

    match args.command {
        Commands::Sync {
            repo_url,
            path,
            dry_run,
        } => {
            commands::sync::sync(&mut agent, &repo_url, &path, dry_run).unwrap();
        }
        Commands::GenSrf { path } => {
            commands::gen_srf::gen_srf(&path);
        }
        Commands::Launch { path } => {
            commands::launch::launch(&path).unwrap();
        }
    }
}
