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

//! `luks-stage-keys` command implementation.

use crate::Config;
use crate::get_key::get_key_internal;
use crate::service::{HttpService, Service};
use std::fs::DirBuilder;
use std::io::{self, Write};
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;
use tempfile::NamedTempFile;

/// Stages one LUKS key in `output_dir`.
async fn stage_one<S: Service>(
    config: &Config,
    volume: &str,
    output_dir: &Path,
    service: &S,
) -> io::Result<()> {
    let mapping = &config.luks[volume];
    let key = get_key_internal(&config.keys, &mapping.key, service).await?;

    let mut temporary = NamedTempFile::new_in(output_dir)?;
    temporary.write_all(key.as_bytes())?;
    temporary.flush()?;
    temporary.persist(output_dir.join(format!("{volume}.key"))).map_err(|error| error.error)?;
    Ok(())
}

/// Stages all configured LUKS keys using the given service.
async fn luks_stage_keys_internal<S: Service>(
    config: Config,
    output_dir: &Path,
    service: &S,
) -> io::Result<()> {
    if config.luks.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "No LUKS volumes are configured"));
    }

    let mut builder = DirBuilder::new();
    builder.recursive(true).mode(0o700);
    builder.create(output_dir)?;

    let mut volumes = config.luks.keys().map(String::as_str).collect::<Vec<_>>();
    volumes.sort_unstable();

    let mut failures = vec![];
    for volume in volumes {
        if let Err(error) = stage_one(&config, volume, output_dir, service).await {
            failures.push(format!("{volume}: {error}"));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "Failed to stage one or more LUKS keys: {}",
            failures.join("; ")
        )))
    }
}

/// Retrieves and stages all configured LUKS keys in `output_dir`.
pub async fn luks_stage_keys(config: Config, output_dir: &Path) -> io::Result<()> {
    let service = HttpService::new(config.service_url.clone())?;
    luks_stage_keys_internal(config, output_dir, &service).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Key, LuksVolume};
    use crate::service::KeySecret;
    use crate::service::testutils::MockService;
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;
    use tempfile::TempDir;
    use url::Url;
    use uuid::{Uuid, uuid};

    const KEY_ID_1: Uuid = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
    const KEY_ID_2: Uuid = uuid!("0de89eda-f2b5-4cc9-a90c-82dbf96f6983");

    /// Returns a configuration with two LUKS volumes.
    fn make_config() -> Config {
        let keys = [
            (
                "first".to_owned(),
                Key {
                    id: KEY_ID_1,
                    local_secret: "local-1-".to_owned(),
                    remote_password: "password-1".to_owned(),
                },
            ),
            (
                "second".to_owned(),
                Key {
                    id: KEY_ID_2,
                    local_secret: "local-2-".to_owned(),
                    remote_password: "password-2".to_owned(),
                },
            ),
        ]
        .into_iter()
        .collect();

        let luks = [
            ("alpha".to_owned(), LuksVolume { key: "first".to_owned() }),
            ("beta".to_owned(), LuksVolume { key: "second".to_owned() }),
        ]
        .into_iter()
        .collect();

        Config {
            default_key_refresh_period: Duration::from_secs(300),
            keys,
            luks,
            service_url: Url::parse("http://unused.example/").unwrap(),
            zfs: HashMap::default(),
        }
    }

    #[tokio::test]
    async fn test_empty_configuration() {
        let mut config = make_config();
        config.luks.clear();
        let output = TempDir::new().unwrap();
        let service = MockService::default();

        let error = luks_stage_keys_internal(config, output.path(), &service).await.unwrap_err();

        assert_eq!(io::ErrorKind::NotFound, error.kind());
        assert_eq!("No LUKS volumes are configured", error.to_string());
    }

    #[tokio::test]
    async fn test_partial_failure() {
        let config = make_config();
        let output = TempDir::new().unwrap();
        let mut service = MockService::default();
        service.add_get_key_secret(KEY_ID_1, "password-1", Err(io::Error::other("Injected")));
        service.add_get_key_secret(
            KEY_ID_2,
            "password-2",
            Ok(KeySecret { secret: "remote-2".to_owned() }),
        );

        let error = luks_stage_keys_internal(config, output.path(), &service).await.unwrap_err();

        assert_eq!("Failed to stage one or more LUKS keys: alpha: Injected", error.to_string());
        assert!(!output.path().join("alpha.key").exists());
        assert_eq!(
            b"local-2-remote-2",
            fs::read(output.path().join("beta.key")).unwrap().as_slice()
        );
    }

    #[tokio::test]
    async fn test_stages_keys_with_private_permissions() {
        let config = make_config();
        let parent = TempDir::new().unwrap();
        let output = parent.path().join("new");
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

        luks_stage_keys_internal(config, &output, &service).await.unwrap();

        assert_eq!(0o700, fs::metadata(&output).unwrap().permissions().mode() & 0o777);
        for (volume, key) in
            [("alpha", b"local-1-remote-1".as_slice()), ("beta", b"local-2-remote-2".as_slice())]
        {
            let path = output.join(format!("{volume}.key"));
            assert_eq!(key, fs::read(&path).unwrap());
            assert_eq!(0o600, fs::metadata(path).unwrap().permissions().mode() & 0o777);
        }
    }
}
