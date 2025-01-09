use crate::md5_digest::Md5Digest;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("failed to create cache file: {}", source))]
    FileCreation { source: std::io::Error },
    #[snafu(display("failed to open cache file: {}", source))]
    FileOpen { source: std::io::Error },
    #[snafu(display("serde failed to serialize: {}", source))]
    Serialization { source: serde_json::Error },
    #[snafu(display("serde failed to deserialize: {}", source))]
    Deserialization { source: serde_json::Error },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mod {
    pub name: String,
}

impl From<crate::srf::Mod> for Mod {
    fn from(value: crate::srf::Mod) -> Self {
        Mod { name: value.name }
    }
}

type SrfMod = crate::srf::Mod;

#[derive(Serialize, Deserialize)]
pub struct ModCache {
    version: u32,
    pub mods: HashMap<Md5Digest, Mod>,
    pub repository: Option<crate::repository::Repository>,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
    /// Last sync timestamp to track when the cache was updated from remote
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}

impl ModCache {
    pub fn new(mods: HashMap<Md5Digest, SrfMod>) -> Result<Self, Error> {
        Ok(Self {
            version: 1,
            mods: mods.into_iter().map(|(k, v)| (k, v.into())).collect(),
            repository: None,
            last_updated: None,
            last_sync: None,
        })
    }

    pub fn new_empty() -> Result<Self, Error> {
        Ok(Self {
            version: 1,
            mods: HashMap::new(),
            repository: None,
            last_updated: None,
            last_sync: None,
        })
    }

    pub fn from_disk(repo_path: &Path) -> Result<Self, Error> {
        let path = repo_path.join("nimble-cache.json");
        let open_result = File::open(path);
        match open_result {
            Ok(file) => {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader).context(DeserializationSnafu)
            }
            Err(e) => Err(Error::FileOpen { source: e }),
        }
    }

    pub fn from_disk_or_empty(repo_path: &Path) -> Result<Self, Error> {
        match Self::from_disk(repo_path) {
            Ok(cache) => Ok(cache),
            Err(Error::FileOpen { source }) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(Self::new_empty()?)
            }
            Err(e) => Err(e),
        }
    }

    pub fn to_disk(&self, repo_path: &Path) -> Result<(), Error> {
        let path = repo_path.join("nimble-cache.json");
        let file = File::create(path).context(FileCreationSnafu)?;
        let writer = BufWriter::new(file);

        serde_json::to_writer(writer, &self).context(SerializationSnafu)?;

        Ok(())
    }

    pub fn remove(&mut self, checksum: &Md5Digest) {
        self.mods.remove(checksum);
    }

    pub fn insert(&mut self, r#mod: crate::srf::Mod) {
        self.mods.insert(r#mod.checksum.clone(), r#mod.into());
    }

    // Add methods for repository caching
    pub fn update_repository(&mut self, repo: crate::repository::Repository) {
        self.repository = Some(repo);
        self.last_updated = Some(chrono::Utc::now());
    }

    pub fn get_repository(&self) -> Option<&crate::repository::Repository> {
        self.repository.as_ref()
    }

    pub fn is_cache_fresh(&self, max_age_hours: i64) -> bool {
        self.last_updated
            .map(|time| {
                let age = chrono::Utc::now() - time;
                age.num_hours() < max_age_hours
            })
            .unwrap_or(false)
    }

    pub fn update_from_remote(&mut self, repo: crate::repository::Repository, base_path: &Path) -> Result<(), Error> {
        self.repository = Some(repo);
        self.last_sync = Some(chrono::Utc::now());
        self.last_updated = Some(chrono::Utc::now());
        self.to_disk(base_path)?;
        Ok(())
    }

    pub fn is_synced(&self) -> bool {
        self.last_sync.is_some()
    }

    pub fn sync_age(&self) -> Option<chrono::Duration> {
        self.last_sync.map(|time| chrono::Utc::now() - time)
    }
}
