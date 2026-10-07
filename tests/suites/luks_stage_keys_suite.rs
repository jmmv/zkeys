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

//! Integration tests for the `luks-stage-keys` command.

use crate::common::mocks::MockService;
use crate::common::zkeys;
use axum::http::StatusCode;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::{NamedTempFile, TempDir};
use uuid::{Uuid, uuid};

const KEY_ID: Uuid = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");

/// Writes a configuration with an optional LUKS mapping.
fn make_config(service_url: &str, with_luks: bool) -> NamedTempFile {
    let luks = if with_luks { "[luks.root]\nkey = \"test\"\n" } else { "" };

    let config = NamedTempFile::new().unwrap();
    fs::write(
        config.path(),
        format!(
            r#"service_url = "{service_url}"

[keys.test]
id = "{KEY_ID}"
remote_password = "remote-password"
local_secret = "local-secret-"

{luks}"#
        ),
    )
    .unwrap();
    config
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_stages_key() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url, true);
    let output = TempDir::new().unwrap();

    zkeys()
        .args(["luks-stage-keys", "--config-file"])
        .arg(config.path())
        .args(["--output-dir"])
        .arg(output.path())
        .assert()
        .code(0)
        .stdout("")
        .stderr("");

    let key = output.path().join("root.key");
    assert_eq!(b"local-secret-service-secret", fs::read(&key).unwrap().as_slice());
    assert_eq!(0o600, fs::metadata(key).unwrap().permissions().mode() & 0o777);
    service.check_request(KEY_ID, "remote-password").await;
    service.shutdown().await;
}

#[test]
fn test_empty_configuration() {
    let config = make_config("http://unused.example/", false);
    let output = TempDir::new().unwrap();

    zkeys()
        .args(["luks-stage-keys", "--config-file"])
        .arg(config.path())
        .args(["--output-dir"])
        .arg(output.path())
        .assert()
        .code(1)
        .stdout("")
        .stderr("zkeys: No LUKS volumes are configured\n");
}

#[test]
fn test_luks_stage_keys_help() {
    zkeys()
        .args(["help", "luks-stage-keys"])
        .assert()
        .code(0)
        .stdout(
            r#"Usage: zkeys luks-stage-keys [options]

Options:
    --output-dir DIR    directory in which to stage LUKS keys (default:
                        /run/cryptsetup-keys.d)
    --config-file FILE  path to the configuration file (default:
                        /non-existent/prefix/etc/zkeys.toml)

zkeys home page: https://zkeys.jmmv.dev/
"#,
        )
        .stderr("");
}
