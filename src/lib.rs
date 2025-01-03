use std::path::PathBuf;
use std::error::Error;
use std::fmt;
use std::process::Command;
use std::path::Path;
use clap::{Parser, Subcommand};

pub mod md5_digest;
pub mod mod_cache;
pub mod pbo;
pub mod repository;
pub mod srf;

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
        Commands::GenSrf { path } => {
            commands::gen_srf::gen_srf(&path)?;
        }
        Commands::Launch { path } => {
            commands::launch::launch(&path)?;
        }
    }
    Ok(())
}

pub mod commands {
    use super::*;
    use crate::mod_cache::ModCache;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufWriter;
    use walkdir::WalkDir;
    use rayon::prelude::*;
    use crate::md5_digest::Md5Digest;
    use crate::srf;

    pub mod sync {
        use super::*;

        pub fn sync<A>(agent: &mut A, repo_url: &str, path: &PathBuf, dry_run: bool) -> Result<(), NimbleError> 
        where A: Send + Sync {
            if !path.exists() {
                return Err(NimbleError::PathNotFound(path.clone()));
            }
            Ok(())
        }
    }

    pub mod gen_srf {
        use super::*;

        fn gen_srf_for_mod(mod_path: &Path) -> srf::Mod {
            let generated_srf = srf::scan_mod(mod_path)
                .map_err(|e| NimbleError::Other(e.to_string()))
                .unwrap();

            let path = mod_path.join("mod.srf");
            let writer = BufWriter::new(File::create(path).unwrap());
            serde_json::to_writer(writer, &generated_srf).unwrap();

            generated_srf
        }

        pub fn open_cache_or_gen_srf(base_path: &Path) -> Result<ModCache, NimbleError> {
            match ModCache::from_disk(base_path) {
                Ok(cache) => Ok(cache),
                Err(mod_cache::Error::FileOpen { source })
                    if source.kind() == std::io::ErrorKind::NotFound =>
                {
                    gen_srf(base_path)?;
                    ModCache::from_disk_or_empty(base_path).map_err(Into::into)
                }
                Err(e) => Err(e.into()),
            }
        }

        pub fn gen_srf(path: &Path) -> Result<(), NimbleError> {
            if !path.exists() {
                return Err(NimbleError::PathNotFound(path.to_path_buf()));
            }

            let mods: HashMap<Md5Digest, srf::Mod> = WalkDir::new(path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .par_bridge()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy().starts_with('@'))
                .map(|entry| {
                    let path = entry.path();
                    let srf = gen_srf_for_mod(path);
                    (srf.checksum.clone(), srf)
                })
                .collect();

            let cache = ModCache::new(mods);
            cache?.to_disk(path).map_err(Into::into)
        }
    }

    pub mod launch {
        use super::*;


        #[cfg(not(windows))]
        fn convert_host_base_path_to_proton_base_path(host_base_path: &Path) -> Result<PathBuf, NimbleError> {
            let drive_c_path = host_base_path
                .ancestors()
                .find(|&x| x.ends_with("drive_c"))
                .ok_or_else(|| NimbleError::Other("Failed to find drive_c".into()))?;

            let relative = host_base_path
                .strip_prefix(drive_c_path)
                .map_err(|e| NimbleError::Other(e.to_string()))?;

            Ok(Path::new("c:/").join(relative))
        }

        #[cfg(windows)]
        fn convert_host_base_path_to_proton_base_path(host_base_path: &Path) -> Result<PathBuf, NimbleError> {
            Ok(host_base_path.to_owned())
        }

        fn generate_mod_args(base_path: &Path, mod_cache: &ModCache) -> String {
            mod_cache
                .mods
                .values()
                .fold(String::from("-noLauncher -mod="), |acc, r#mod| {
                    let mod_name = &r#mod.name;
                    let full_path = base_path
                        .join(Path::new(mod_name))
                        .to_string_lossy()
                        .to_string();
                    format!("{acc}{full_path};")
                })
        }

        pub fn launch(path: &PathBuf) -> Result<(), NimbleError> {
            if !path.exists() {
                return Err(NimbleError::PathNotFound(path.clone()));
            }

            let mod_cache = gen_srf::open_cache_or_gen_srf(path)?;
            let proton_base_path = convert_host_base_path_to_proton_base_path(path)?;
            
            let binding = generate_mod_args(&proton_base_path, &mod_cache);
            let cmdline = percent_encoding::utf8_percent_encode(&binding, percent_encoding::NON_ALPHANUMERIC);
            
            let steam_url = format!("steam://run/107410//{cmdline}/");
            
            open::that(steam_url)
                .map_err(|e| NimbleError::LaunchError(e.to_string()))?;

            Ok(())
        }
    }
}
