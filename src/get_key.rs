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

//! `get-key` command implementation.

use crate::Config;
use crate::config::Key;
use crate::service::{HttpService, Service};
use std::collections::HashMap;
use std::io;

/// Retrieves `name` from `service` and returns the full key material.
pub(crate) async fn get_key_internal<S: Service>(
    keys: &HashMap<String, Key>,
    name: &str,
    service: &S,
) -> io::Result<String> {
    let key = keys.get(name).ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, format!("No key named {name} is configured"))
    })?;

    let result = service.get_key_secret(key.id, &key.remote_password).await?;
    Ok(format!("{}{}", key.local_secret, result.secret))
}

/// Retrieves `name` from the configured service and writes it to standard output.
pub async fn get_key(config: Config, name: &str) -> io::Result<()> {
    let service = HttpService::new(config.service_url.clone())?;
    let secret = get_key_internal(&config.keys, name, &service).await?;
    println!("{secret}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Key;
    use crate::service::KeySecret;
    use crate::service::testutils::MockService;
    use std::collections::HashMap;
    use uuid::{Uuid, uuid};

    const KEY_ID: Uuid = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");

    /// Returns a configuration containing one key named `test`.
    fn make_keys() -> HashMap<String, Key> {
        let mut keys = HashMap::new();
        keys.insert(
            "test".to_owned(),
            Key {
                id: KEY_ID,
                local_secret: "local-secret-".to_owned(),
                remote_password: "remote-password".to_owned(),
            },
        );
        keys
    }

    #[tokio::test]
    async fn test_ok() {
        let keys = make_keys();
        let mut service = MockService::default();
        service.add_get_key_secret(
            KEY_ID,
            "remote-password",
            Ok(KeySecret { secret: "service-secret".to_owned() }),
        );

        assert_eq!(
            "local-secret-service-secret",
            get_key_internal(&keys, "test", &service).await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_service_error() {
        let keys = make_keys();
        let mut service = MockService::default();
        service.add_get_key_secret(
            KEY_ID,
            "remote-password",
            Err(io::Error::other("Injected error")),
        );

        let error = get_key_internal(&keys, "test", &service).await.unwrap_err();

        assert_eq!(io::ErrorKind::Other, error.kind());
        assert_eq!("Injected error", error.to_string());
    }

    #[tokio::test]
    async fn test_unknown_name() {
        let keys = make_keys();
        let service = MockService::default();

        let error = get_key_internal(&keys, "unknown", &service).await.unwrap_err();

        assert_eq!(io::ErrorKind::NotFound, error.kind());
        assert_eq!("No key named unknown is configured", error.to_string());
    }
}
