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

//! REST client for the zkeys service.

use reqwest::StatusCode;
use std::future::Future;
use std::io;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

/// Converts a `reqwest::Error` to an `io::Error`.
fn reqwest_error_to_io_error(mut e: reqwest::Error) -> io::Error {
    if let Some(url) = e.url_mut() {
        url.set_query(None);
        url.set_fragment(None);
    }
    io::Error::other(format!("{}", e))
}

/// Contains the secret returned by the service.
pub(crate) struct KeySecret {
    /// Key material returned by the service.
    pub(crate) secret: String,
}

/// Result of refreshing a key.
pub(crate) enum KeepAliveResult {
    /// The key no longer exists in the service.
    Deleted,

    /// The key was refreshed, with an optional recommended refresh period.
    Refreshed(Option<Duration>),
}

/// Interface to the zkeys service.
pub(crate) trait Service {
    /// Retrieves a key secret identified by `key_id` using `password`.
    async fn get_key_secret(&self, key_id: Uuid, password: &str) -> io::Result<KeySecret>;

    /// Refreshes the last-access timestamp of a key.
    fn keep_alive(
        &self,
        key_id: Uuid,
        password: &str,
    ) -> impl Future<Output = io::Result<KeepAliveResult>> + Send;
}

/// HTTP client for the zkeys service.
#[derive(Clone)]
pub(crate) struct HttpService {
    /// Service API root URL.
    base_url: Url,

    /// Client used to make HTTP requests.
    client: reqwest::Client,
}

impl HttpService {
    /// Creates a client for a service at `url`.
    pub fn new(url: Url) -> io::Result<Self> {
        if !(url.path().is_empty() || url.path() == "/") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid service API address {url}: cannot contain a path"),
            ));
        }

        Ok(Self { base_url: url, client: reqwest::Client::default() })
    }

    /// Generates a service URL with the given `path`.
    fn make_url(&self, path: &str) -> Url {
        assert!(path.starts_with("api/"));
        let mut url = self.base_url.clone();
        assert!(url.path().is_empty() || url.path() == "/");
        url.set_path(path);
        url
    }
}

impl Service for HttpService {
    async fn get_key_secret(&self, key_id: Uuid, password: &str) -> io::Result<KeySecret> {
        let response = self
            .client
            .get(self.make_url(&format!("api/v1/keys/{}/secret", key_id)))
            .query(&[("password", password)])
            .send()
            .await
            .map_err(reqwest_error_to_io_error)?;

        if response.status() != StatusCode::OK {
            let status = response.status();
            let text = response.text().await.map_err(reqwest_error_to_io_error)?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Service returned HTTP {}: {}", status, text),
            ));
        }

        let secret = response.text().await.map_err(reqwest_error_to_io_error)?;

        Ok(KeySecret { secret })
    }

    async fn keep_alive(&self, key_id: Uuid, password: &str) -> io::Result<KeepAliveResult> {
        let response = self
            .client
            .get(self.make_url(&format!("api/v1/keys/{}/keepalive", key_id)))
            .query(&[("password", password)])
            .send()
            .await
            .map_err(reqwest_error_to_io_error)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(KeepAliveResult::Deleted);
        }

        if response.status() != StatusCode::OK {
            let status = response.status();
            let text = response.text().await.map_err(reqwest_error_to_io_error)?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Service returned HTTP {}: {}", status, text),
            ));
        }

        let refresh_period = response
            .headers()
            .get("X-Key-Refresh-Period-Seconds")
            .map(|value| -> io::Result<Duration> {
                let value = value.to_str().map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid key refresh period header: {error}"),
                    )
                })?;
                let seconds = value.parse::<u64>().map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid key refresh period header {value:?}: {error}"),
                    )
                })?;
                Ok(Duration::from_secs(seconds))
            })
            .transpose()?;

        Ok(KeepAliveResult::Refreshed(refresh_period))
    }
}

#[cfg(test)]
pub(crate) mod testutils {
    use super::{KeepAliveResult, KeySecret, Service};
    use std::collections::VecDeque;
    use std::io;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    /// Mapping of a `get_key` request to the mock response.
    type GetKeySecretMock = ((Uuid, String), io::Result<KeySecret>);
    type KeepAliveMock = ((Uuid, String), io::Result<KeepAliveResult>);

    /// Shared state for a mock service.
    #[derive(Default)]
    struct MockServiceState {
        get_key_secret: Mutex<VecDeque<GetKeySecretMock>>,
        keep_alive: Mutex<VecDeque<KeepAliveMock>>,
    }

    impl Drop for MockServiceState {
        fn drop(&mut self) {
            assert!(
                self.get_key_secret.get_mut().unwrap().is_empty(),
                "Mock requests not fully consumed"
            );
            assert!(
                self.keep_alive.get_mut().unwrap().is_empty(),
                "Mock requests not fully consumed"
            );
        }
    }

    /// Service implementation that returns predefined results for expected requests.
    #[derive(Clone, Default)]
    pub(crate) struct MockService {
        state: Arc<MockServiceState>,
    }

    impl MockService {
        /// Records an expected key-secret request and the result to return.
        pub(crate) fn add_get_key_secret(
            &mut self,
            key_id: Uuid,
            password: &str,
            result: io::Result<KeySecret>,
        ) {
            self.state
                .get_key_secret
                .lock()
                .unwrap()
                .push_back(((key_id, password.to_owned()), result));
        }

        /// Records an expected keep-alive request and the result to return.
        pub(crate) fn add_keep_alive(
            &mut self,
            key_id: Uuid,
            password: &str,
            result: io::Result<KeepAliveResult>,
        ) {
            self.state
                .keep_alive
                .lock()
                .unwrap()
                .push_back(((key_id, password.to_owned()), result));
        }
    }

    impl Service for MockService {
        async fn get_key_secret(&self, key_id: Uuid, password: &str) -> io::Result<KeySecret> {
            let mock = self
                .state
                .get_key_secret
                .lock()
                .unwrap()
                .pop_front()
                .expect("No mock requests available");
            assert_eq!(mock.0.0, key_id);
            assert_eq!(mock.0.1, password);
            mock.1
        }

        async fn keep_alive(&self, key_id: Uuid, password: &str) -> io::Result<KeepAliveResult> {
            let mut mocks = self.state.keep_alive.lock().unwrap();
            let position = mocks
                .iter()
                .position(|mock| mock.0.0 == key_id && mock.0.1 == password)
                .expect("No mock request available for key");
            mocks.remove(position).unwrap().1
        }
    }
}
