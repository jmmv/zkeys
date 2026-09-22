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

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use url::Url;
use uuid::Uuid;

/// Returns the default URL of the zkeys service.
fn default_service_url() -> Url {
    Url::parse("https://zkeys.jmmv.dev/").unwrap()
}

/// Describes a key that can be retrieved from the service.
#[derive(Deserialize)]
pub struct Key {
    /// Unique identifier used by the service.
    pub id: Uuid,

    /// Password sent to the service to retrieve the key.
    pub remote_password: String,

    /// Secret prepended to the value returned by the service.
    pub local_secret: String,
}

/// Describes the zkeys client configuration.
#[derive(Deserialize)]
pub struct Config {
    /// Base URL of the zkeys service.
    #[serde(default = "default_service_url")]
    pub service_url: Url,

    /// Keys indexed by their local names.
    pub keys: HashMap<String, Key>,
}

impl Config {
    /// Parses a configuration file at `path`.
    pub fn parse<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path.as_ref())?;
        toml::from_str(&content).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}
