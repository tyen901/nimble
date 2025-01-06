use crate::md5_digest::Md5Digest;
use serde::{Deserialize, Deserializer, Serialize};
use snafu::prelude::*;
use std::{fmt::Display, net::IpAddr, str::FromStr};
use ureq::Agent;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Error while requesting repository data: {}", source))]
    Http {
        url: String,

        #[snafu(source(from(ureq::Error, Box::new)))]
        source: Box<ureq::Error>,
    },
    #[snafu(display("Error while deserializing: {}", source))]
    Deserialization { source: std::io::Error },
}

pub fn deserialize_number_from_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt<T> {
        String(String),
        Number(T),
    }

    match StringOrInt::<T>::deserialize(deserializer)? {
        StringOrInt::String(s) => s.parse::<T>().map_err(serde::de::Error::custom),
        StringOrInt::Number(i) => Ok(i),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")] // this particular file is camelcase for reasons
pub struct Mod {
    pub mod_name: String,
    #[serde(rename = "checkSum")] // why
    pub checksum: Md5Digest,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")] // this particular file is camelcase for reasons
pub struct BasicAuth {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub name: String,
    pub address: IpAddr,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub password: String,
    pub battle_eye: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")] // this particular file is camelcase for reasons
pub struct Repository {
    pub repo_name: String,
    pub checksum: String,
    pub required_mods: Vec<Mod>,
    pub optional_mods: Vec<Mod>,
    pub client_parameters: String,
    pub repo_basic_authentication: Option<BasicAuth>,
    pub version: String,
    pub servers: Vec<Server>,
}

impl Repository {
    pub fn new(url: &str, agent: &mut ureq::Agent) -> Result<Self, Error> {
        let repo_json_url = format!("{}/repo.json", url.trim_end_matches('/'));
        get_repository_info(agent, &repo_json_url)
    }

    pub fn validate_connection(agent: &mut Agent, repo_url: &str) -> Result<(), String> {
        let repo_json_url = format!("{}/repo.json", repo_url.trim_end_matches('/'));
        
        match agent.get(&repo_json_url).call() {
            Ok(response) => {
                if response.status() != 200 {
                    return Err(format!("Repository returned status: {}", response.status()));
                }
                Ok(())
            },
            Err(e) => Err(format!("Failed to connect to repository: {}", e)),
        }
    }
}

pub fn get_repository_info(agent: &mut ureq::Agent, url: &str) -> Result<Repository, Error> {
    agent
        .get(url)
        .call()
        .context(HttpSnafu { url: url.to_string() })?
        .into_json()
        .context(DeserializationSnafu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    impl Repository {
        fn create_test_repository() -> Self {
            Repository {
                repo_name: "Test Repository".to_string(),
                checksum: "DUMMY_HASH".to_string(),
                required_mods: vec![
                    Mod {
                        mod_name: "@test_mod1".to_string(),
                        checksum: Md5Digest::default(),
                        enabled: true,
                    },
                    Mod {
                        mod_name: "@test_mod2".to_string(),
                        checksum: Md5Digest::default(),
                        enabled: true,
                    },
                ],
                optional_mods: vec![],
                client_parameters: "-noPause -noSplash -skipIntro".to_string(),
                repo_basic_authentication: None,
                version: "1.0.0".to_string(),
                servers: vec![
                    Server {
                        name: "Test Server".to_string(),
                        address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                        port: 2302,
                        password: "password".to_string(),
                        battle_eye: true,
                    },
                ],
            }
        }
    }

    #[test]
    fn test_repository_serialization() {
        let repo = Repository::create_test_repository();
        
        // Serialize to JSON
        let json = serde_json::to_string_pretty(&repo).unwrap();
        
        // Deserialize back to Repository
        let deserialized: Repository = serde_json::from_str(&json).unwrap();
        
        // Verify fields
        assert_eq!(deserialized.repo_name, "Test Repository");
        assert_eq!(deserialized.version, "1.0.0");
        assert_eq!(deserialized.required_mods.len(), 2);
        assert_eq!(deserialized.required_mods[0].mod_name, "@test_mod1");
        assert_eq!(deserialized.required_mods[1].mod_name, "@test_mod2");
        assert_eq!(deserialized.servers.len(), 1);
        assert_eq!(deserialized.servers[0].name, "Test Server");
        assert_eq!(deserialized.servers[0].port, 2302);
    }

    #[test]
    fn test_repository_file_format() {
        let repo = Repository::create_test_repository();
        let json = serde_json::to_string_pretty(&repo).unwrap();
        
        // Write to file
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().join("repo.json");
        std::fs::write(&repo_path, json).unwrap();
        
        // Read and parse file
        let content = std::fs::read_to_string(&repo_path).unwrap();
        let parsed: Repository = serde_json::from_str(&content).unwrap();
        
        // Verify structure matches example_repo.json format
        assert!(parsed.client_parameters.contains("-noPause"));
        assert!(parsed.required_mods.iter().all(|m| m.mod_name.starts_with('@')));
        assert!(parsed.servers.iter().all(|s| s.port >= 1024 && s.port <= 65535));
    }
}
