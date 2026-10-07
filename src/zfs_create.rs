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

//! `zfs-create` command implementation.

use crate::Config;
use crate::get_key::get_key_internal;
use crate::service::{HttpService, Service};
use crate::zfs::{CommandZfs, Zfs, print_key_warning};
use log::info;
use std::io;

/// Retrieves the configured key and creates the ZFS dataset.
async fn zfs_create_internal<S: Service, Z: Zfs>(
    config: &Config,
    args: &[String],
    service: &S,
    zfs: &Z,
) -> io::Result<String> {
    let dataset = args.last().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "No ZFS dataset was specified")
    })?;
    let mapping = config.zfs.get(dataset).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("No ZFS dataset named {dataset} is configured"),
        )
    })?;
    info!("Loading key {}", mapping.key);
    let key = get_key_internal(&config.keys, &mapping.key, service).await?;
    zfs.create(&key, args).await?;
    Ok(key)
}

/// Creates an encrypted dataset using its configured key.
pub async fn zfs_create(config: Config, quiet: bool, args: &[String]) -> io::Result<()> {
    let service = HttpService::new(config.service_url.clone())?;
    let key = zfs_create_internal(&config, args, &service, &CommandZfs).await?;
    if !quiet {
        print_key_warning(&key);
    }
    Ok(())
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

    /// Returns a configuration containing one key named `test`.
    fn make_config() -> Config {
        let mut keys = HashMap::new();
        keys.insert(
            "test".to_owned(),
            Key {
                id: KEY_ID_1,
                local_secret: "local-".to_owned(),
                remote_password: "password".to_owned(),
            },
        );
        let mut zfs = HashMap::new();
        zfs.insert("pool/test".to_owned(), ZfsDataset { key: "test".to_owned() });
        Config {
            default_key_refresh_period: Duration::from_secs(300),
            keys,
            service_url: Url::parse("http://unused.example/").unwrap(),
            luks: HashMap::default(),
            zfs,
        }
    }

    /// Returns a mock service that provides the key from `make_config`.
    fn make_service() -> MockService {
        let mut service = MockService::default();
        service.add_get_key_secret(
            KEY_ID_1,
            "password",
            Ok(KeySecret { secret: "remote".to_owned() }),
        );
        service
    }

    #[tokio::test]
    async fn test_create_uses_last_argument_as_dataset() {
        let mut config = make_config();
        config.keys.insert(
            "other".to_owned(),
            Key {
                id: KEY_ID_2,
                local_secret: "other-local-".to_owned(),
                remote_password: "other-password".to_owned(),
            },
        );
        config.zfs.insert("pool/other".to_owned(), ZfsDataset { key: "other".to_owned() });
        let service = make_service();
        let zfs = MockZfs::default();
        zfs.add_create("local-remote", &["pool/other", "pool/test"], Ok(()));
        let args = ["pool/other".to_owned(), "pool/test".to_owned()];

        let key = zfs_create_internal(&config, &args, &service, &zfs).await.unwrap();

        assert_eq!("local-remote", key);
    }

    #[tokio::test]
    async fn test_missing_arguments() {
        let config = make_config();
        let service = MockService::default();
        let zfs = MockZfs::default();

        let error = zfs_create_internal(&config, &[], &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::InvalidInput, error.kind());
        assert_eq!("No ZFS dataset was specified", error.to_string());
    }

    #[tokio::test]
    async fn test_service_error_does_not_invoke_zfs() {
        let config = make_config();
        let mut service = MockService::default();
        service.add_get_key_secret(KEY_ID_1, "password", Err(io::Error::other("Injected")));
        let zfs = MockZfs::default();
        let args = ["pool/test".to_owned()];

        let error = zfs_create_internal(&config, &args, &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::Other, error.kind());
        assert_eq!("Injected", error.to_string());
    }

    #[tokio::test]
    async fn test_unknown_dataset() {
        let config = make_config();
        let service = MockService::default();
        let zfs = MockZfs::default();
        let args = ["pool/unknown".to_owned()];

        let error = zfs_create_internal(&config, &args, &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::NotFound, error.kind());
        assert_eq!("No ZFS dataset named pool/unknown is configured", error.to_string());
    }

    #[tokio::test]
    async fn test_zfs_error() {
        let config = make_config();
        let service = make_service();
        let zfs = MockZfs::default();
        zfs.add_create(
            "local-remote",
            &["-o", "user:test=pool/other", "pool/test"],
            Err(io::Error::other("Injected")),
        );
        let args = ["-o".to_owned(), "user:test=pool/other".to_owned(), "pool/test".to_owned()];

        let error = zfs_create_internal(&config, &args, &service, &zfs).await.unwrap_err();

        assert_eq!(io::ErrorKind::Other, error.kind());
        assert_eq!("Injected", error.to_string());
    }
}
