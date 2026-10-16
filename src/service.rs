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

//! REST client for the zkeys service.

use crate::config::RetryPolicy;
use reqwest::StatusCode;
use std::future::Future;
use std::io;
use std::time::Duration;
use thiserror::Error;
use url::Url;
use uuid::Uuid;

const REFRESH_PERIOD_HEADER: &str = "X-Key-Refresh-Period-Seconds";
const RETRY_INITIAL_PERIOD_HEADER: &str = "X-Key-Retry-Initial-Period-Seconds";
const RETRY_MAX_PERIOD_HEADER: &str = "X-Key-Retry-Max-Period-Seconds";

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

    /// The key was refreshed, with recommendations for future requests.
    Refreshed {
        refresh_period: Recommendation<Duration>,
        retry_policy: Recommendation<RetryPolicy>,
    },
}

/// Recommendation from a successful keep-alive request.
pub(crate) enum Recommendation<T> {
    /// The service provided an invalid recommendation.
    Invalid(io::Error),

    /// The service did not provide a recommendation.
    Missing,

    /// The service provided a valid recommendation.
    Value(T),
}

/// Error returned by a keep-alive request.
#[derive(Debug, Error)]
pub(crate) enum KeepAliveError {
    /// A failure that should be retried at the normal refresh period.
    #[error(transparent)]
    NonTransient(io::Error),

    /// A failure that should be retried sooner than the normal refresh period.
    #[error(transparent)]
    Transient(io::Error),
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
    ) -> impl Future<Output = Result<KeepAliveResult, KeepAliveError>> + Send;
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

    /// Parses a duration in seconds from `name`, if present.
    fn parse_period_header(
        headers: &reqwest::header::HeaderMap,
        name: &str,
    ) -> Recommendation<Duration> {
        let Some(value) = headers.get(name) else {
            return Recommendation::Missing;
        };
        let value = match value.to_str() {
            Ok(value) => value,
            Err(error) => {
                return Recommendation::Invalid(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid {name} header: {error}"),
                ));
            }
        };
        match value.parse::<u64>() {
            Ok(seconds) => Recommendation::Value(Duration::from_secs(seconds)),
            Err(error) => Recommendation::Invalid(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid {name} header {value:?}: {error}"),
            )),
        }
    }

    /// Parses and validates the optional retry policy headers.
    fn parse_retry_policy(headers: &reqwest::header::HeaderMap) -> Recommendation<RetryPolicy> {
        let initial_period = Self::parse_period_header(headers, RETRY_INITIAL_PERIOD_HEADER);
        let max_period = Self::parse_period_header(headers, RETRY_MAX_PERIOD_HEADER);
        let (initial_period, max_period) = match (initial_period, max_period) {
            (Recommendation::Invalid(error), _) | (_, Recommendation::Invalid(error)) => {
                return Recommendation::Invalid(error);
            }
            (Recommendation::Missing, Recommendation::Missing) => {
                return Recommendation::Missing;
            }
            (Recommendation::Value(initial_period), Recommendation::Value(max_period)) => {
                (initial_period, max_period)
            }
            _ => {
                return Recommendation::Invalid(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Service returned an incomplete key retry policy",
                ));
            }
        };
        match RetryPolicy::new(initial_period, max_period) {
            Ok(policy) => Recommendation::Value(policy),
            Err(error) => Recommendation::Invalid(error),
        }
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

    async fn keep_alive(
        &self,
        key_id: Uuid,
        password: &str,
    ) -> Result<KeepAliveResult, KeepAliveError> {
        let response = self
            .client
            .get(self.make_url(&format!("api/v1/keys/{}/keepalive", key_id)))
            .query(&[("password", password)])
            .send()
            .await
            .map_err(reqwest_error_to_io_error)
            .map_err(KeepAliveError::Transient)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(KeepAliveResult::Deleted);
        }

        if response.status() != StatusCode::OK {
            let status = response.status();
            let text = response
                .text()
                .await
                .map_err(reqwest_error_to_io_error)
                .map_err(KeepAliveError::Transient)?;
            let error = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Service returned HTTP {}: {}", status, text),
            );
            return if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                Err(KeepAliveError::Transient(error))
            } else {
                Err(KeepAliveError::NonTransient(error))
            };
        }

        let refresh_period = Self::parse_period_header(response.headers(), REFRESH_PERIOD_HEADER);
        let retry_policy = Self::parse_retry_policy(response.headers());

        Ok(KeepAliveResult::Refreshed { refresh_period, retry_policy })
    }
}

#[cfg(test)]
pub(crate) mod testutils {
    use super::{KeepAliveError, KeepAliveResult, KeySecret, Service};
    use std::collections::VecDeque;
    use std::io;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    /// Mapping of a `get_key` request to the mock response.
    type GetKeySecretMock = ((Uuid, String), io::Result<KeySecret>);
    type KeepAliveMock = ((Uuid, String), Result<KeepAliveResult, KeepAliveError>);

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
            result: Result<KeepAliveResult, KeepAliveError>,
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

        async fn keep_alive(
            &self,
            key_id: Uuid,
            password: &str,
        ) -> Result<KeepAliveResult, KeepAliveError> {
            let mut mocks = self.state.keep_alive.lock().unwrap();
            let position = mocks
                .iter()
                .position(|mock| mock.0.0 == key_id && mock.0.1 == password)
                .expect("No mock request available for key");
            mocks.remove(position).unwrap().1
        }
    }
}
