// zkeys
// Copyright 2026 Julio Merino.
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

//! `zfs-load-key` command implementation.

use crate::Config;
use crate::get_key::get_key_internal;
use crate::service::{HttpService, Service};
use crate::zfs::{CommandZfs, KeyStatus, Zfs};
use log::warn;
use std::io;

/// Loads the key for one configured ZFS dataset.
async fn load_one<S: Service, Z: Zfs>(
    config: &Config,
    dataset: &str,
    service: &S,
    zfs: &Z,
) -> io::Result<()> {
    let mapping = config.zfs.get(dataset).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("No ZFS dataset named {dataset} is configured"),
        )
    })?;

    match zfs.key_status(dataset).await? {
        KeyStatus::Available => {
            warn!("Key already loaded for {dataset}");
            Ok(())
        }
        KeyStatus::Unavailable => {
            let key = get_key_internal(&config.keys, &mapping.key, service).await?;
            zfs.load_key(dataset, &key).await
        }
    }
}

/// Loads one or all configured ZFS keys using the given backends.
async fn zfs_load_key_internal<S: Service, Z: Zfs>(
    config: Config,
    dataset: Option<&str>,
    service: &S,
    zfs: &Z,
) -> io::Result<()> {
    if let Some(dataset) = dataset {
        return load_one(&config, dataset, service, zfs).await;
    }

    if config.zfs.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "No ZFS datasets are configured"));
    }

    let mut datasets = config.zfs.keys().map(String::as_str).collect::<Vec<_>>();
    datasets.sort_unstable();

    let mut failures = vec![];
    for dataset in datasets {
        if let Err(error) = load_one(&config, dataset, service, zfs).await {
            failures.push(format!("{dataset}: {error}"));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "Failed to load one or more ZFS keys: {}",
            failures.join("; ")
        )))
    }
}

/// Loads the key for `dataset`, or all configured ZFS keys when it is `None`.
pub async fn zfs_load_key(config: Config, dataset: Option<&str>) -> io::Result<()> {
    let service = HttpService::new(config.service_url.clone())?;
    zfs_load_key_internal(config, dataset, &service, &CommandZfs).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Key, ZfsDataset};
    use crate::service::KeySecret;
    use crate::service::testutils::MockService;
    use crate::zfs::testutils::MockZfs;
    use std::collections::HashMap;
    use std::time::Duration;
    use url::Url;
    use uuid::{Uuid, uuid};

    const KEY_ID_1: Uuid = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
    const KEY_ID_2: Uuid = uuid!("0de89eda-f2b5-4cc9-a90c-82dbf96f6983");

    /// Returns a configuration with the requested dataset-to-key mappings.
    fn make_config(mappings: &[(&str, &str, Uuid, &str, &str)]) -> Config {
        let mut keys = HashMap::new();
        let mut zfs = HashMap::new();
        for (dataset, name, id, password, local_secret) in mappings {
            keys.insert(
                (*name).to_owned(),
                Key {
                    id: *id,
                    local_secret: (*local_secret).to_owned(),
                    remote_password: (*password).to_owned(),
                },
            );
            zfs.insert((*dataset).to_owned(), ZfsDataset { key: (*name).to_owned() });
        }
        Config {
            default_key_refresh_period: Duration::from_secs(300),
            keys,
            service_url: Url::parse("http://unused.example/").unwrap(),
            zfs,
        }
    }

    #[tokio::test]
    async fn test_all_continues_after_failure_in_sorted_order() {
        let config = make_config(&[
            ("zpool/second", "second", KEY_ID_2, "password-2", "local-2-"),
            ("apool/first", "first", KEY_ID_1, "password-1", "local-1-"),
        ]);
        let mut service = MockService::default();
        service.add_get_key_secret(
            KEY_ID_1,
            "password-1",
            Ok(KeySecret { secret: "remote-1".to_owned() }),
        );
        service.add_get_key_secret(
            KEY_ID_2,
            "password-2",
            Ok(KeySecret { secret: "remote-2".to_owned() }),
        );
        let zfs = MockZfs::default();
        zfs.add_key_status("apool/first", Ok(KeyStatus::Unavailable));
        zfs.add_load_key("apool/first", "local-1-remote-1", Err(io::Error::other("Injected")));
        zfs.add_key_status("zpool/second", Ok(KeyStatus::Unavailable));
        zfs.add_load_key("zpool/second", "local-2-remote-2", Ok(()));

        let error = zfs_load_key_internal(config, None, &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::Other, error.kind());
        assert_eq!("Failed to load one or more ZFS keys: apool/first: Injected", error.to_string());
    }

    #[tokio::test]
    async fn test_already_loaded() {
        let config = make_config(&[("pool/test", "test", KEY_ID_1, "password", "local-")]);
        let service = MockService::default();
        let zfs = MockZfs::default();
        zfs.add_key_status("pool/test", Ok(KeyStatus::Available));

        zfs_load_key_internal(config, Some("pool/test"), &service, &zfs).await.unwrap();
    }

    #[tokio::test]
    async fn test_empty_all() {
        let config = make_config(&[]);
        let service = MockService::default();
        let zfs = MockZfs::default();

        let error = zfs_load_key_internal(config, None, &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::NotFound, error.kind());
        assert_eq!("No ZFS datasets are configured", error.to_string());
    }

    #[tokio::test]
    async fn test_loads_key() {
        let config = make_config(&[("pool/test", "test", KEY_ID_1, "password", "local-")]);
        let mut service = MockService::default();
        service.add_get_key_secret(
            KEY_ID_1,
            "password",
            Ok(KeySecret { secret: "remote".to_owned() }),
        );
        let zfs = MockZfs::default();
        zfs.add_key_status("pool/test", Ok(KeyStatus::Unavailable));
        zfs.add_load_key("pool/test", "local-remote", Ok(()));

        zfs_load_key_internal(config, Some("pool/test"), &service, &zfs).await.unwrap();
    }

    #[tokio::test]
    async fn test_service_error() {
        let config = make_config(&[("pool/test", "test", KEY_ID_1, "password", "local-")]);
        let mut service = MockService::default();
        service.add_get_key_secret(KEY_ID_1, "password", Err(io::Error::other("Injected")));
        let zfs = MockZfs::default();
        zfs.add_key_status("pool/test", Ok(KeyStatus::Unavailable));

        let error =
            zfs_load_key_internal(config, Some("pool/test"), &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::Other, error.kind());
        assert_eq!("Injected", error.to_string());
    }

    #[tokio::test]
    async fn test_unknown_dataset() {
        let config = make_config(&[("pool/test", "test", KEY_ID_1, "password", "local-")]);
        let service = MockService::default();
        let zfs = MockZfs::default();

        let error =
            zfs_load_key_internal(config, Some("pool/unknown"), &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::NotFound, error.kind());
        assert_eq!("No ZFS dataset named pool/unknown is configured", error.to_string());
    }

    #[tokio::test]
    async fn test_zfs_error() {
        let config = make_config(&[("pool/test", "test", KEY_ID_1, "password", "local-")]);
        let service = MockService::default();
        let zfs = MockZfs::default();
        zfs.add_key_status("pool/test", Err(io::Error::other("Injected")));

        let error =
            zfs_load_key_internal(config, Some("pool/test"), &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::Other, error.kind());
        assert_eq!("Injected", error.to_string());
    }
}
