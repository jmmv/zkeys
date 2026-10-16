// ZKeys
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
use std::fs::File;
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path};
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

/// Returns the default initial delay between retries.
fn default_key_retry_initial_period() -> Duration {
    Duration::from_secs(5)
}

/// Returns the default maximum delay between retries.
fn default_key_retry_max_period() -> Duration {
    Duration::from_secs(60)
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

/// Describes the key associated with a LUKS volume.
#[derive(Clone, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct LuksVolume {
    /// Name of the key used to unlock the volume.
    pub key: String,
}

#[derive(Deserialize)]
struct UnvalidatedRetryPolicy {
    #[serde(
        default = "default_key_retry_initial_period",
        deserialize_with = "deserialize_duration",
        rename = "default_key_retry_initial_period"
    )]
    initial_period: Duration,

    #[serde(
        default = "default_key_retry_max_period",
        deserialize_with = "deserialize_duration",
        rename = "default_key_retry_max_period"
    )]
    max_period: Duration,
}

/// Policy for retrying transient keep-alive failures.
#[derive(Clone, Copy, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
#[serde(try_from = "UnvalidatedRetryPolicy")]
pub struct RetryPolicy {
    /// Initial delay between retries.
    initial_period: Duration,

    /// Maximum delay between retries.
    max_period: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(default_key_retry_initial_period(), default_key_retry_max_period()).unwrap()
    }
}

impl RetryPolicy {
    /// Creates a retry policy with the given initial and maximum delays.
    pub fn new(initial_period: Duration, max_period: Duration) -> io::Result<Self> {
        if initial_period.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Initial retry period must be greater than zero",
            ));
        }
        if max_period.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Maximum retry period must be greater than zero",
            ));
        }
        if initial_period > max_period {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Initial retry period cannot exceed maximum retry period",
            ));
        }
        Ok(Self { initial_period, max_period })
    }

    /// Returns the initial delay between retries.
    pub fn initial_period(&self) -> Duration {
        self.initial_period
    }

    /// Returns the maximum delay between retries.
    pub fn max_period(&self) -> Duration {
        self.max_period
    }
}

impl TryFrom<UnvalidatedRetryPolicy> for RetryPolicy {
    type Error = io::Error;

    fn try_from(policy: UnvalidatedRetryPolicy) -> io::Result<Self> {
        Self::new(policy.initial_period, policy.max_period)
    }
}

/// Describes the key associated with a ZFS dataset.
#[derive(Clone, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct ZfsDataset {
    /// Name of the key used to encrypt and unlock the dataset.
    pub key: String,
}

/// Describes the zkeys client configuration.
#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct Config {
    /// Interval to use when the service does not provide one.
    #[serde(default = "default_key_refresh_period", deserialize_with = "deserialize_duration")]
    pub default_key_refresh_period: Duration,

    /// Retry policy to use when the service does not provide one.
    #[serde(flatten)]
    pub default_key_retry_policy: RetryPolicy,

    /// Base URL of the zkeys service.
    #[serde(default = "default_service_url")]
    pub service_url: Url,

    /// Keys indexed by their local names.
    #[serde(default)]
    pub keys: HashMap<String, Key>,

    /// LUKS volumes indexed by their mapper names.
    #[serde(default)]
    pub luks: HashMap<String, LuksVolume>,

    /// ZFS datasets indexed by their names.
    #[serde(default)]
    pub zfs: HashMap<String, ZfsDataset>,
}

impl Config {
    /// Parses a configuration file in `content`.
    fn parse_from_str(content: &str) -> io::Result<Self> {
        let config: Self = toml::from_str(content)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        for (volume, luks) in &config.luks {
            // LUKS mapper names are supposed to be unique path components.  Make sure they are
            // so that we don't end up trying to create subdirectories and the like later on.
            let mut components = Path::new(volume).components();
            if !matches!(components.next(), Some(Component::Normal(_)))
                || components.next().is_some()
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid LUKS volume name {volume:?}"),
                ));
            }

            if !config.keys.contains_key(&luks.key) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("LUKS volume {volume} references undefined key {}", luks.key),
                ));
            }
        }

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
        let mut file = File::open(path.as_ref())?;

        if file.metadata()?.permissions().mode() & 0o066 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "configuration file is accessible by group or others",
            ));
        }

        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Self::parse_from_str(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    /// Changes the permissions of already-open `file` to `mode`.
    fn set_file_mode(file: &File, mode: u32) -> io::Result<()> {
        let mut permissions = file.metadata()?.permissions();
        permissions.set_mode(mode);
        file.set_permissions(permissions)
    }

    #[test]
    fn test_parse_from_str_defaults() {
        let config = Config::parse_from_str("").unwrap();

        assert_eq!(
            Config {
                service_url: Url::parse("https://zkeys.jmmv.dev/").unwrap(),
                keys: HashMap::default(),
                default_key_refresh_period: default_key_refresh_period(),
                default_key_retry_policy: RetryPolicy::default(),
                luks: HashMap::default(),
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

        let mut luks = HashMap::default();
        luks.insert("root".to_owned(), LuksVolume { key: "first".to_owned() });

        let mut zfs = HashMap::default();
        zfs.insert("pool/first".to_owned(), ZfsDataset { key: "first".to_owned() });
        zfs.insert("second".to_owned(), ZfsDataset { key: "second".to_owned() });

        let config = Config::parse_from_str(&format!(
            r#"
service_url = "https://example.com/api/"
default_key_refresh_period = "2d"
default_key_retry_initial_period = "10s"
default_key_retry_max_period = "2m"

[keys.first]
id = "{KEY_ID_1}"
remote_password = "remote-password-1"
local_secret = "local-secret-1"

[keys.second]
id = "{KEY_ID_2}"
remote_password = "remote-password-2"
local_secret = "local-secret-2"

[luks.root]
key = "first"

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
                default_key_retry_policy: RetryPolicy::new(
                    Duration::from_secs(10),
                    Duration::from_secs(120),
                )
                .unwrap(),
                keys,
                luks,
                zfs,
            },
            config
        );
    }

    #[test]
    fn test_parse_from_str_rejects_invalid_retry_periods() {
        for (configuration, message) in [
            (
                "default_key_retry_initial_period = \"0s\"",
                "Initial retry period must be greater than zero",
            ),
            (
                "default_key_retry_max_period = \"0s\"",
                "Maximum retry period must be greater than zero",
            ),
            (
                "default_key_retry_initial_period = \"2m\"\ndefault_key_retry_max_period = \"1m\"",
                "Initial retry period cannot exceed maximum retry period",
            ),
        ] {
            let error = Config::parse_from_str(configuration).unwrap_err();
            assert_eq!(io::ErrorKind::InvalidData, error.kind());
            assert!(error.to_string().contains(message));
        }
    }

    #[test]
    fn test_parse_from_str_rejects_invalid_luks_volume_name() {
        for volume in ["", ".", "..", "dir/name", "/name"] {
            let error = Config::parse_from_str(&format!(
                r#"
[keys.test]
id = "00000001-0001-0001-0001-000000000001"
remote_password = "password"
local_secret = "secret"

[luks.{volume:?}]
key = "test"
"#
            ))
            .unwrap_err();

            assert_eq!(io::ErrorKind::InvalidData, error.kind());
            assert_eq!(format!("Invalid LUKS volume name {volume:?}"), error.to_string());
        }
    }

    #[test]
    fn test_parse_from_str_rejects_undefined_luks_key() {
        let error = Config::parse_from_str(
            r#"
[luks.root]
key = "missing"
"#,
        )
        .unwrap_err();

        assert_eq!(io::ErrorKind::InvalidData, error.kind());
        assert_eq!("LUKS volume root references undefined key missing", error.to_string());
    }

    #[test]
    fn test_parse_from_str_rejects_invalid_zfs_entry() {
        let error = Config::parse_from_str(
            r#"
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
        fs::write(file.path(), r#"service_url = "https://example.com/api/""#).unwrap();

        let config = Config::parse(file.path()).unwrap();
        assert_eq!(Url::parse("https://example.com/api/").unwrap(), config.service_url);
    }

    #[test]
    fn test_parse_rejects_group_or_other_access() {
        for mode in [0o640, 0o620, 0o604, 0o602] {
            let file = NamedTempFile::new().unwrap();
            fs::write(file.path(), "").unwrap();
            set_file_mode(file.as_file(), mode).unwrap();

            let error = Config::parse(file.path()).unwrap_err();

            assert_eq!(io::ErrorKind::PermissionDenied, error.kind());
            assert_eq!("configuration file is accessible by group or others", error.to_string());
        }
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
