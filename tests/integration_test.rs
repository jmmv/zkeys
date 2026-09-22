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

//! Integration tests for the command-line client.

use assert_cmd::Command;
use std::fs;
use tempfile::NamedTempFile;
use uuid::{Uuid, uuid};

mod mocks;

use axum::http::StatusCode;
use mocks::MockService;

/// Constructs a `Command` to execute zkeys.
fn zkeys() -> Command {
    let mut command = Command::cargo_bin("zkeys").unwrap();
    command.env("ZKEYS_TEST_PREFIX", "/non-existent/prefix");
    command
}

/// Writes a configuration file for one key and returns it.
fn make_config(
    service_url: &str,
    key_id: Uuid,
    password: &str,
    local_secret: &str,
) -> NamedTempFile {
    let config = NamedTempFile::new().unwrap();
    fs::write(
        config.path(),
        format!(
            r#"service_url = "{service_url}"

[keys.test]
id = "{key_id}"
remote_password = "{password}"
local_secret = "{local_secret}"
"#
        ),
    )
    .unwrap();
    config
}

/// Checks the output from a service error response.
async fn check_service_error(
    status: StatusCode,
    body: &str,
    password: &str,
    expected_stderr: &str,
) {
    let key_id = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
    let mut service = MockService::start(status, body).await;
    let config = make_config(&service.url, key_id, password, "local-secret-");

    zkeys()
        .args(["get-key", "--config-file"])
        .arg(config.path())
        .arg("test")
        .assert()
        .code(1)
        .stdout("")
        .stderr(expected_stderr.to_owned());

    service.check_request(key_id, password).await;
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_key() {
    let key_id = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
    let password = "remote-password";
    let local_secret = "local-secret-";
    let secret = "service-secret";
    let mut service = MockService::start(StatusCode::OK, secret).await;
    let config = make_config(&service.url, key_id, password, local_secret);

    zkeys()
        .args(["get-key", "--config-file"])
        .arg(config.path())
        .arg("test")
        .assert()
        .code(0)
        .stdout(format!("{local_secret}{secret}\n"))
        .stderr("");

    service.check_request(key_id, password).await;
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_key_invalid_service_key() {
    check_service_error(
        StatusCode::NOT_FOUND,
        "Entity not found",
        "remote-password",
        "zkeys: Service returned HTTP 404 Not Found: Entity not found\n",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_key_locked() {
    check_service_error(
        StatusCode::FORBIDDEN,
        "Access denied: Key locked",
        "remote-password",
        "zkeys: Service returned HTTP 403 Forbidden: Access denied: Key locked\n",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_key_server_error() {
    check_service_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Service temporarily unavailable",
        "remote-password",
        "zkeys: Service returned HTTP 500 Internal Server Error: Service temporarily unavailable\n",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_key_service_error() {
    check_service_error(
        StatusCode::BAD_GATEWAY,
        "The upstream service is unavailable",
        "remote-password",
        "zkeys: Service returned HTTP 502 Bad Gateway: The upstream service is unavailable\n",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_key_unreachable_service() {
    let key_id = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
    let password = "remote-password";
    let service_url = MockService::unreachable_url().await;
    let config = make_config(&service_url, key_id, password, "local-secret-");
    let output = zkeys()
        .args(["get-key", "--config-file"])
        .arg(config.path())
        .arg("test")
        .output()
        .expect("Failed to execute subprocess");
    let stdout = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("Stderr is not valid UTF-8");

    assert_eq!(Some(1), output.status.code());
    assert_eq!("", stdout);
    assert!(stderr.starts_with("zkeys: error sending request for url ("));
    assert!(stderr.contains(&format!("{service_url}/api/v1/keys/{key_id}/secret")));
    assert!(!stderr.contains(password));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_key_wrong_password() {
    check_service_error(
        StatusCode::FORBIDDEN,
        "Access denied: Failed to decrypt secret",
        "wrong-password",
        "zkeys: Service returned HTTP 403 Forbidden: Access denied: Failed to decrypt secret\n",
    )
    .await;
}

#[test]
fn test_help() {
    zkeys()
        .arg("help")
        .assert()
        .code(0)
        .stdout(
            r#"Usage: zkeys command [arg...]

Commands:
    get-key             retrieve a key
    help                show command-line usage information
    version             show version information

zkeys home page: https://zkeys.jmmv.dev/
"#,
        )
        .stderr("");
}

#[test]
fn test_get_key_help() {
    zkeys()
        .args(["help", "get-key"])
        .assert()
        .code(0)
        .stdout(
            r#"Usage: zkeys get-key [options] name

Options:
    --config-file FILE  path to the configuration file (default:
                        /non-existent/prefix/etc/zkeys.toml)

Arguments:
    name                name of the key to retrieve

zkeys home page: https://zkeys.jmmv.dev/
"#,
        )
        .stderr("");
}

#[test]
fn test_config_file_is_not_a_global_option() {
    zkeys()
        .args(["--config-file", "/no/such/zkeys.toml", "version"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(
            "Usage error: Unrecognized option: 'config-file'\nType `zkeys help` or `man 8 zkeys` for more information\n",
        );
}

#[test]
fn test_get_key_missing_name() {
    zkeys().arg("get-key").assert().code(2).stdout("").stderr(
        r#"Usage error: Required argument `name` not provided
Type `zkeys help` or `man 8 zkeys` for more information
"#,
    );
}

#[test]
fn test_get_key_missing_config() {
    zkeys()
        .args(["get-key", "--config-file", "/no/such/zkeys.toml", "test"])
        .assert()
        .code(1)
        .stdout("")
        .stderr("zkeys: Failed to load configuration file /no/such/zkeys.toml: No such file or directory (os error 2)\n");
}

#[test]
fn test_get_key_invalid_config() {
    let config = NamedTempFile::new().unwrap();
    fs::write(config.path(), "this is not = valid TOML =").unwrap();

    let output = zkeys()
        .args(["get-key", "--config-file"])
        .arg(config.path())
        .arg("test")
        .output()
        .expect("Failed to execute subprocess");
    let stderr = String::from_utf8(output.stderr).expect("Stderr is not valid UTF-8");

    assert_eq!(Some(1), output.status.code());
    assert!(stderr.starts_with(&format!(
        "zkeys: Failed to load configuration file {}: ",
        config.path().display()
    )));
    assert!(stderr.contains("TOML parse error"));
}

#[test]
fn test_get_key_invalid_service_url() {
    let key_id = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
    let service_url = "http://127.0.0.1:1/not-an-api-root";
    let config = make_config(service_url, key_id, "password", "local-secret-");

    zkeys()
        .args(["get-key", "--config-file"])
        .arg(config.path())
        .arg("test")
        .assert()
        .code(1)
        .stdout("")
        .stderr(format!(
            "zkeys: Invalid service API address {service_url}: cannot contain a path\n"
        ));
}

#[test]
fn test_get_key_unknown_name() {
    let key_id = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");
    let config = make_config("http://127.0.0.1:1", key_id, "password", "local-secret-");

    zkeys()
        .args(["get-key", "--config-file"])
        .arg(config.path())
        .arg("unknown")
        .assert()
        .code(1)
        .stdout("")
        .stderr("zkeys: No key named unknown is configured\n");
}
