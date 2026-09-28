// zkeys
// Copyright 2025 Julio Merino.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
// * Redistributions of source code must retain the above copyright
//   notice, this list of conditions and the following disclaimer.
// * Redistributions in binary form must reproduce the above copyright
//   notice, this list of conditions and the following disclaimer in the
//   documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! Configuration file support.

use serde::{Deserialize, Deserializer, de};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

/// Returns the default URL of the zkeys service.
fn default_service_url() -> Url {
    Url::parse("https://zkeys.jmmv.dev/").unwrap()
}

/// Returns the default interval to use when the service does not provide one.
fn default_key_refresh_period() -> Duration {
    Duration::from_secs(300)
}

/// Deserializes a human-readable duration.
fn deserialize_duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    let raw = String::deserialize(deserializer)?;
    humantime::parse_duration(&raw).map_err(de::Error::custom)
}

/// Describes a key that can be retrieved from the service.
#[derive(Clone, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct Key {
    /// Unique identifier used by the service.
    pub id: Uuid,

    /// Password sent to the service to retrieve the key.
    pub remote_password: String,

    /// Secret prepended to the value returned by the service.
    pub local_secret: String,
}

/// Describes the key associated with a ZFS dataset.
#[derive(Clone, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct ZfsDataset {
    /// Name of the key used to unlock the dataset.
    pub key: String,
}

/// Describes the zkeys client configuration.
#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct Config {
    /// Interval to use when the service does not provide one.
    #[serde(default = "default_key_refresh_period", deserialize_with = "deserialize_duration")]
    pub default_key_refresh_period: Duration,

    /// Base URL of the zkeys service.
    #[serde(default = "default_service_url")]
    pub service_url: Url,

    /// Keys indexed by their local names.
    pub keys: HashMap<String, Key>,

    /// ZFS datasets indexed by their names.
    #[serde(default)]
    pub zfs: HashMap<String, ZfsDataset>,
}

impl Config {
    /// Parses a configuration file in `content`.
    fn parse_from_str(content: &str) -> io::Result<Self> {
        let config: Self = toml::from_str(content)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        for (dataset, zfs) in &config.zfs {
            if !config.keys.contains_key(&zfs.key) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("ZFS dataset {dataset} references undefined key {}", zfs.key),
                ));
            }
        }

        Ok(config)
    }

    /// Parses a configuration file at `path`.
    pub fn parse<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path.as_ref())?;
        Self::parse_from_str(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_from_str_defaults() {
        let config = Config::parse_from_str("[keys]").unwrap();

        assert_eq!(
            Config {
                service_url: Url::parse("https://zkeys.jmmv.dev/").unwrap(),
                keys: HashMap::default(),
                default_key_refresh_period: default_key_refresh_period(),
                zfs: HashMap::default(),
            },
            config
        );
    }

    #[test]
    fn test_parse_from_str_explicit() {
        const KEY_ID_1: &str = "00000001-0001-0001-0001-000000000001";
        const KEY_ID_2: &str = "00000002-0002-0002-0002-000000000002";

        let mut keys = HashMap::default();
        keys.insert(
            "first".to_owned(),
            Key {
                id: Uuid::parse_str(KEY_ID_1).unwrap(),
                remote_password: "remote-password-1".to_owned(),
                local_secret: "local-secret-1".to_owned(),
            },
        );
        keys.insert(
            "second".to_owned(),
            Key {
                id: Uuid::parse_str(KEY_ID_2).unwrap(),
                remote_password: "remote-password-2".to_owned(),
                local_secret: "local-secret-2".to_owned(),
            },
        );

        let mut zfs = HashMap::default();
        zfs.insert("pool/first".to_owned(), ZfsDataset { key: "first".to_owned() });
        zfs.insert("second".to_owned(), ZfsDataset { key: "second".to_owned() });

        let config = Config::parse_from_str(&format!(
            r#"
service_url = "https://example.com/api/"
default_key_refresh_period = "2d"

[keys.first]
id = "{KEY_ID_1}"
remote_password = "remote-password-1"
local_secret = "local-secret-1"

[keys.second]
id = "{KEY_ID_2}"
remote_password = "remote-password-2"
local_secret = "local-secret-2"

[zfs."pool/first"]
key = "first"

[zfs.second]
key = "second"
"#
        ))
        .unwrap();

        assert_eq!(
            Config {
                service_url: Url::parse("https://example.com/api/").unwrap(),
                default_key_refresh_period: Duration::from_secs(2 * 24 * 60 * 60),
                keys,
                zfs,
            },
            config
        );
    }

    #[test]
    fn test_parse_from_str_rejects_invalid_zfs_entry() {
        let error = Config::parse_from_str(
            r#"
[keys]

[zfs.dataset]
key = ["not", "a", "string"]
"#,
        )
        .unwrap_err();

        assert_eq!(io::ErrorKind::InvalidData, error.kind());
        assert!(error.to_string().contains("invalid type"));
    }

    #[test]
    fn test_parse_from_str_rejects_undefined_zfs_key() {
        let error = Config::parse_from_str(
            r#"
[keys]

[zfs."pool/dataset"]
key = "missing"
"#,
        )
        .unwrap_err();

        assert_eq!(io::ErrorKind::InvalidData, error.kind());
        assert_eq!("ZFS dataset pool/dataset references undefined key missing", error.to_string());
    }

    #[test]
    fn test_parse_reads_configuration_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
service_url = "https://example.com/api/"

[keys]
"#,
        )
        .unwrap();

        let config = Config::parse(file.path()).unwrap();
        assert_eq!(Url::parse("https://example.com/api/").unwrap(), config.service_url);
    }

    #[test]
    fn test_parse_reports_missing_file() {
        let file = NamedTempFile::new().unwrap();
        fs::remove_file(file.path()).unwrap();

        let error = Config::parse(file.path()).err().unwrap();

        assert_eq!(io::ErrorKind::NotFound, error.kind());
    }

    #[test]
    fn test_parse_reports_malformed_configuration() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "not valid TOML").unwrap();

        let error = Config::parse(file.path()).err().unwrap();

        assert_eq!(io::ErrorKind::InvalidData, error.kind());
    }
}
