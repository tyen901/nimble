#![allow(warnings)]          // Disables all warnings for the entire file
#![allow(dead_code)]         // For unused code
#![allow(unused_variables)]  // For unused variables
#![allow(unused_imports)]    // For unused imports

use std::path::PathBuf;
use std::error::Error;
use std::fmt;
use clap::{Parser, Subcommand};

pub mod commands;
pub mod md5_digest;
pub mod mod_cache;
pub mod pbo;
pub mod repository;
pub mod srf;
pub mod gui;

#[derive(Debug)]
pub enum NimbleError {
    PathNotFound(PathBuf),
    NetworkError(String),
    LaunchError(String),
    Other(String),
}

impl fmt::Display for NimbleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PathNotFound(path) => write!(f, "Path not found: {}", path.display()),
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::LaunchError(msg) => write!(f, "Launch error: {}", msg),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for NimbleError {}

impl From<mod_cache::Error> for NimbleError {
    fn from(error: mod_cache::Error) -> Self {
        NimbleError::Other(error.to_string())
    }
}

impl From<commands::sync::Error> for NimbleError {
    fn from(error: commands::sync::Error) -> Self {
        NimbleError::Other(error.to_string())
    }
}

impl From<commands::launch::Error> for NimbleError {
    fn from(error: commands::launch::Error) -> Self {
        NimbleError::Other(error.to_string())
    }
}

impl From<commands::diff::Error> for NimbleError {
    fn from(error: commands::diff::Error) -> Self {
        NimbleError::Other(error.to_string())
    }
}

#[derive(Subcommand)]
pub enum Commands {
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

        #[clap(short, long)]
        output: Option<PathBuf>,
    },
    Launch {
        #[clap(short, long)]
        path: PathBuf,
    },
}

#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,
}

pub fn run(args: Args) -> Result<(), NimbleError> {
    let mut agent = ureq::AgentBuilder::new()
        .user_agent("nimble (like Swifty)/0.1")
        .build();

    match args.command {
        Commands::Sync {
            repo_url,
            path,
            dry_run,
        } => {
            commands::sync::sync(&mut agent, &repo_url, &path, dry_run)?;
        }
        Commands::GenSrf { path, output } => {
            commands::gen_srf::gen_srf(&path, output.as_deref(), None)?;
        }
        Commands::Launch { path } => {
            commands::launch::launch(&path, None)?;
        }
    }
    Ok(())
}
