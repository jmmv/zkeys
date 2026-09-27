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

//! Integration tests for the `keep-alive` command.

use crate::common::mocks::{KeepAliveResponse, MockService};
use crate::common::zkeys;
use axum::http::StatusCode;
use std::fs;
use tempfile::NamedTempFile;
use uuid::{Uuid, uuid};

const KEY_ID_1: Uuid = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
const KEY_ID_2: Uuid = uuid!("0de89eda-f2b5-4cc9-a90c-82dbf96f6983");

/// Writes a configuration file for one key.
fn make_config(
    service_url: &str,
    key_id: Uuid,
    password: &str,
    refresh_period: Option<&str>,
) -> NamedTempFile {
    let refresh_period = refresh_period
        .map(|period| format!("default_key_refresh_period = {period:?}\n"))
        .unwrap_or_default();

    let config = NamedTempFile::new().unwrap();
    fs::write(
        config.path(),
        format!(
            r#"
service_url = "{service_url}"
{refresh_period}

[keys.test]
id = "{key_id}"
remote_password = "{password}"
local_secret = "local-secret-"
"#
        ),
    )
    .unwrap();
    config
}

/// Writes a configuration file for multiple keys and returns it.
fn make_multi_config(service_url: &str, keys: &[(&str, Uuid, &str)]) -> NamedTempFile {
    let config = NamedTempFile::new().unwrap();
    let mut contents = format!("service_url = {service_url:?}\n");
    for (name, id, password) in keys {
        contents += &format!(
            r#"
[keys.{name}]
id = "{id}"
remote_password = "{password}"
local_secret = "local-secret-"
"#
        );
    }
    fs::write(config.path(), contents).unwrap();
    config
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_refresh_period_changes() {
    let password = "remote-password";
    let mut service = MockService::start(StatusCode::OK, "").await;
    for response in [
        KeepAliveResponse { body: "", refresh_period: Some("0"), status: StatusCode::OK },
        KeepAliveResponse { body: "", refresh_period: Some("1"), status: StatusCode::OK },
        KeepAliveResponse {
            body: "Entity not found",
            refresh_period: None,
            status: StatusCode::NOT_FOUND,
        },
    ] {
        service.add_keep_alive_response(response).await;
    }
    let config = make_config(&service.url, KEY_ID_1, password, None);

    let output = zkeys()
        .env("RUST_LOG", "zkeys=info")
        .args(["keep-alive", "--config-file"])
        .arg(config.path())
        .arg("test")
        .output()
        .expect("Failed to execute subprocess");
    let stderr = String::from_utf8(output.stderr).expect("Stderr is not valid UTF-8");

    assert_eq!(Some(0), output.status.code());
    assert_eq!(b"", output.stdout.as_slice());
    assert!(stderr.contains("Sending keepalive for key test"));
    assert!(stderr.contains("Next keepalive for key test is due in 0ns"));
    assert!(stderr.contains("Next keepalive for key test is due in 1s"));
    assert!(stderr.contains("Keepalive for key test failed: key was deleted"));
    for _ in 0..3 {
        service.check_request(KEY_ID_1, password).await;
    }
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_retries_failures_with_previous_period() {
    let password = "remote-password";
    let mut service = MockService::start(StatusCode::OK, "").await;
    for response in [
        KeepAliveResponse { body: "", refresh_period: None, status: StatusCode::OK },
        KeepAliveResponse {
            body: "Access denied: Invalid key credentials",
            refresh_period: None,
            status: StatusCode::FORBIDDEN,
        },
        KeepAliveResponse {
            body: "Service temporarily unavailable",
            refresh_period: None,
            status: StatusCode::INTERNAL_SERVER_ERROR,
        },
        KeepAliveResponse { body: "", refresh_period: Some("invalid"), status: StatusCode::OK },
        KeepAliveResponse {
            body: "Entity not found",
            refresh_period: None,
            status: StatusCode::NOT_FOUND,
        },
    ] {
        service.add_keep_alive_response(response).await;
    }
    let config = make_config(&service.url, KEY_ID_1, password, Some("0s"));

    let output = zkeys()
        .env("RUST_LOG", "zkeys=info")
        .args(["keep-alive", "--config-file"])
        .arg(config.path())
        .arg("test")
        .output()
        .expect("Failed to execute subprocess");
    let stderr = String::from_utf8(output.stderr).expect("Stderr is not valid UTF-8");

    assert_eq!(Some(0), output.status.code());
    assert!(stderr.contains("HTTP 403 Forbidden"));
    assert!(stderr.contains("HTTP 500 Internal Server Error"));
    assert!(stderr.contains("Invalid key refresh period header \"invalid\""));
    for _ in 0..5 {
        service.check_request(KEY_ID_1, password).await;
    }
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_all_keys() {
    let mut service = MockService::start(StatusCode::OK, "").await;
    for response in [
        KeepAliveResponse {
            body: "Entity not found",
            refresh_period: None,
            status: StatusCode::NOT_FOUND,
        },
        KeepAliveResponse {
            body: "Entity not found",
            refresh_period: None,
            status: StatusCode::NOT_FOUND,
        },
    ] {
        service.add_keep_alive_response(response).await;
    }
    let config = make_multi_config(
        &service.url,
        &[("first", KEY_ID_1, "password-1"), ("second", KEY_ID_2, "password-2")],
    );

    zkeys()
        .env("RUST_LOG", "off")
        .args(["keep-alive", "--config-file"])
        .arg(config.path())
        .assert()
        .code(0)
        .stdout("")
        .stderr("");

    let mut requests = vec![service.next_request().await, service.next_request().await];
    requests.sort();
    let mut expected =
        vec![(KEY_ID_1, "password-1".to_owned()), (KEY_ID_2, "password-2".to_owned())];
    expected.sort();
    assert_eq!(expected, requests);
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_selected_keys() {
    let mut service = MockService::start(StatusCode::OK, "").await;
    for response in [KeepAliveResponse {
        body: "Entity not found",
        refresh_period: None,
        status: StatusCode::NOT_FOUND,
    }] {
        service.add_keep_alive_response(response).await;
    }
    let config = make_multi_config(
        &service.url,
        &[("first", KEY_ID_1, "password-1"), ("second", KEY_ID_2, "password-2")],
    );

    zkeys()
        .env("RUST_LOG", "off")
        .args(["keep-alive", "--config-file"])
        .arg(config.path())
        .arg("second")
        .assert()
        .code(0)
        .stdout("")
        .stderr("");

    service.check_request(KEY_ID_2, "password-2").await;
    service.check_no_requests().await;
    service.shutdown().await;
}

#[test]
fn test_validates_all_names_first() {
    let config = make_config("http://127.0.0.1:1/not-an-api-root", KEY_ID_1, "password", None);

    zkeys()
        .env("RUST_LOG", "off")
        .args(["keep-alive", "--config-file"])
        .arg(config.path())
        .args(["test", "unknown"])
        .assert()
        .code(1)
        .stdout("")
        .stderr("zkeys: No key named unknown is configured\n");
}

#[test]
fn test_invalid_default_period() {
    let config = make_config("http://127.0.0.1:1", KEY_ID_1, "password", Some("invalid"));

    let output = zkeys()
        .env("RUST_LOG", "off")
        .args(["keep-alive", "--config-file"])
        .arg(config.path())
        .output()
        .expect("Failed to execute subprocess");
    let stderr = String::from_utf8(output.stderr).expect("Stderr is not valid UTF-8");

    assert_eq!(Some(1), output.status.code());
    assert_eq!(b"", output.stdout.as_slice());
    assert!(stderr.starts_with(&format!(
        "zkeys: Failed to load configuration file {}: ",
        config.path().display()
    )));
    assert!(stderr.contains("expected number at 0"));
}

#[test]
fn test_keep_alive_help() {
    zkeys()
        .args(["help", "keep-alive"])
        .assert()
        .code(0)
        .stdout(
            r#"Usage: zkeys keep-alive [options] [name1 .. nameN]

Options:
    --config-file FILE  path to the configuration file (default:
                        /non-existent/prefix/etc/zkeys.toml)

Arguments:
    [name1 .. nameN]    names of the keys to keep alive (default: all)

zkeys home page: https://zkeys.jmmv.dev/
"#,
        )
        .stderr("");
}
