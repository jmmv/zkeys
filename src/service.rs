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
use std::io;
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

/// Interface to the zkeys service.
pub(crate) trait Service {
    /// Retrieves a key secret identified by `key_id` using `password`.
    async fn get_key_secret(&self, key_id: Uuid, password: &str) -> io::Result<KeySecret>;
}

/// HTTP client for the zkeys service.
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
}

#[cfg(test)]
pub(crate) mod testutils {
    use super::{KeySecret, Service};
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io;
    use uuid::Uuid;

    /// Mapping of a `get_key` request to the mock response.
    type GetKeySecretMock = ((Uuid, String), io::Result<KeySecret>);

    /// Service implementation that returns predefined results for expected requests.
    #[derive(Default)]
    pub(crate) struct MockService {
        get_key_secret: RefCell<VecDeque<GetKeySecretMock>>,
    }

    impl MockService {
        /// Records an expected key-secret request and the result to return.
        pub(crate) fn add_get_key_secret(
            &mut self,
            key_id: Uuid,
            password: &str,
            result: io::Result<KeySecret>,
        ) {
            self.get_key_secret.get_mut().push_back(((key_id, password.to_owned()), result));
        }
    }

    impl Drop for MockService {
        fn drop(&mut self) {
            assert!(self.get_key_secret.borrow().is_empty(), "Mock requests not fully consumed");
        }
    }

    impl Service for MockService {
        async fn get_key_secret(&self, key_id: Uuid, password: &str) -> io::Result<KeySecret> {
            let mock =
                self.get_key_secret.borrow_mut().pop_front().expect("No mock requests available");
            assert_eq!(mock.0.0, key_id);
            assert_eq!(mock.0.1, password);
            mock.1
        }
    }
}
