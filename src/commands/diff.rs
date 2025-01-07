use crate::{mod_cache::ModCache, repository, srf};
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

#[derive(Debug)]
pub struct DownloadCommand {
    pub file: String,
    pub begin: u64,
    pub end: u64,
}

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("io error: {}", source))]
    Io { source: std::io::Error },
    #[snafu(display("Error while requesting repository data: {}", source))]
    Http {
        url: String,
        #[snafu(source(from(ureq::Error, Box::new)))]
        source: Box<ureq::Error>,
    },
    #[snafu(display("SRF deserialization failure: {}", source))]
    SrfDeserialization { source: serde_json::Error },
    #[snafu(display("Legacy SRF deserialization failure: {}", source))]
    LegacySrfDeserialization { source: srf::Error },
    #[snafu(display("Failed to generate SRF: {}", source))]
    SrfGeneration { source: srf::Error },
}

pub fn diff_repo<'a>(
    mod_cache: &ModCache,
    remote_repo: &'a repository::Repository,
) -> Vec<&'a repository::Mod> {
    let mut downloads = Vec::new();

    // Include both required_mods and optional_mods
    for r#mod in remote_repo.required_mods.iter().chain(remote_repo.optional_mods.iter()) {
        if !mod_cache.mods.contains_key(&r#mod.checksum) {
            downloads.push(r#mod);
        }
    }

    downloads
}

pub fn diff_mod(
    agent: &ureq::Agent,
    repo_base_path: &str,
    local_base_path: &Path,
    remote_mod: &repository::Mod,
) -> Result<Vec<DownloadCommand>, Error> {
    let remote_srf_url = format!("{}{}/mod.srf", repo_base_path, remote_mod.mod_name);
    let mut remote_srf = agent
        .get(&remote_srf_url)
        .call()
        .context(HttpSnafu {
            url: remote_srf_url,
        })?
        .into_reader();

    let mut buf = String::new();
    let _len = remote_srf.read_to_string(&mut buf).context(IoSnafu)?;

    let bomless = buf.trim_start_matches('\u{feff}');
    let remote_is_legacy = srf::is_legacy_srf(&mut Cursor::new(bomless)).context(IoSnafu)?;

    let remote_srf: srf::Mod = if remote_is_legacy {
        srf::deserialize_legacy_srf(&mut BufReader::new(Cursor::new(bomless)))
            .context(LegacySrfDeserializationSnafu)?
    } else {
        serde_json::from_str(bomless).context(SrfDeserializationSnafu)?
    };

    let local_path = local_base_path.join(Path::new(&format!("{}/", remote_mod.mod_name)));
    let srf_path = local_path.join(Path::new("mod.srf"));

    let local_srf = {
        if local_path.exists() {
            let file = File::open(srf_path);

            match file {
                Ok(file) => {
                    let mut reader = BufReader::new(file);

                    if srf::is_legacy_srf(&mut reader).context(IoSnafu)? {
                        srf::deserialize_legacy_srf(&mut reader)
                            .context(LegacySrfDeserializationSnafu)?
                    } else {
                        serde_json::from_reader(&mut reader).context(SrfDeserializationSnafu)?
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    srf::scan_mod(&local_path).context(SrfGenerationSnafu)?
                }
                Err(e) => return Err(Error::Io { source: e }),
            }
        } else {
            srf::Mod::generate_invalid(&remote_srf)
        }
    };

    if local_srf.checksum == remote_srf.checksum {
        return Ok(vec![]);
    }

    let mut local_files = HashMap::new();
    let mut remote_files = HashMap::new();

    for file in &local_srf.files {
        local_files.insert(&file.path, file);
    }

    for file in &remote_srf.files {
        remote_files.insert(&file.path, file);
    }

    let mut download_list = Vec::new();

    for (path, file) in remote_files.drain() {
        let local_file = local_files.remove(path);

        if let Some(local_file) = local_file {
            if file.checksum != local_file.checksum {
                download_list.push(DownloadCommand {
                    file: format!("{}/{}", remote_srf.name, path),
                    begin: 0,
                    end: file.length,
                });
            }
        } else {
            download_list.push(DownloadCommand {
                file: format!("{}/{}", remote_srf.name, path),
                begin: 0,
                end: file.length,
            });
        }
    }

    remove_leftover_files(local_base_path, &remote_srf, local_files.into_values())
        .context(IoSnafu)?;

    Ok(download_list)
}

fn remove_leftover_files<'a>(
    local_base_path: &Path,
    r#mod: &srf::Mod,
    files: impl Iterator<Item = &'a srf::File>,
) -> Result<(), std::io::Error> {
    for file in files {
        let path = file
            .path
            .to_path(local_base_path.join(Path::new(&r#mod.name)));

        println!("removing leftover file {}", &path.display());

        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                eprintln!("Warning: Failed to remove file {}: {}", path.display(), e);
            }
        }
    }

    Ok(())
}
