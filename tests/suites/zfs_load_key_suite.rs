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

//! Integration tests for the `zfs-load-key` command.

use crate::common::mocks::MockService;
use crate::common::zkeys;
use axum::http::StatusCode;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::{NamedTempFile, TempDir};
use uuid::{Uuid, uuid};

const KEY_ID: Uuid = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");

/// Files used by a mock ZFS executable.
struct MockZfs {
    _directory: TempDir,
    key: PathBuf,
    log: PathBuf,
}

/// Creates a mock ZFS executable and its state files.
fn make_mock_zfs() -> MockZfs {
    let directory = TempDir::new().unwrap();
    let executable = directory.path().join("zfs");
    let key = directory.path().join("key");
    let log = directory.path().join("log");
    fs::write(
        &executable,
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >>"${MOCK_ZFS_LOG}"
case "$1" in
    get)
        printf '%s\n' "${MOCK_ZFS_KEY_STATUS}"
        ;;
    load-key)
        cat >"${MOCK_ZFS_KEY}"
        if [ -n "${MOCK_ZFS_LOAD_ERROR:-}" ]; then
            echo "${MOCK_ZFS_LOAD_ERROR}" >&2
            exit 1
        fi
        ;;
    *)
        exit 2
        ;;
esac
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).unwrap();
    MockZfs { _directory: directory, key, log }
}

/// Writes a configuration for a ZFS dataset and returns it.
fn make_config(service_url: &str, dataset: Option<&str>) -> NamedTempFile {
    let config = NamedTempFile::new().unwrap();
    let zfs = dataset.map_or_else(String::new, |dataset| {
        format!(
            r#"
[zfs."{dataset}"]
key = "test"
"#
        )
    });
    fs::write(
        config.path(),
        format!(
            r#"service_url = "{service_url}"

[keys.test]
id = "{KEY_ID}"
remote_password = "remote-password"
local_secret = "local-secret-"
{zfs}"#
        ),
    )
    .unwrap();
    config
}

/// Adds the environment needed to use a mock ZFS executable.
fn configure_mock(command: &mut assert_cmd::Command, zfs: &MockZfs, status: &str) {
    let path = env::join_paths(
        std::iter::once(zfs._directory.path().to_path_buf())
            .chain(env::split_paths(&env::var_os("PATH").unwrap())),
    )
    .unwrap();
    command
        .env("PATH", path)
        .env("MOCK_ZFS_KEY", &zfs.key)
        .env("MOCK_ZFS_KEY_STATUS", status)
        .env("MOCK_ZFS_LOG", &zfs.log);
}

/// Returns the contents of `path`.
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_already_loaded() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url, Some("pool/test"));
    let zfs = make_mock_zfs();
    let mut command = zkeys();
    configure_mock(&mut command, &zfs, "available");

    let output = command
        .args(["zfs-load-key", "--config-file"])
        .arg(config.path())
        .arg("pool/test")
        .output()
        .unwrap();

    assert_eq!(Some(0), output.status.code());
    assert_eq!(b"", output.stdout.as_slice());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!("zkeys: WARN: Key already loaded for pool/test\n", stderr);

    assert_eq!("get -H -o value keystatus pool/test\n", read(&zfs.log));
    assert!(!zfs.key.exists());
    service.check_no_requests().await;
    service.shutdown().await;
}

#[test]
fn test_all_with_no_mappings() {
    let config = make_config("http://unused.example/", None);

    zkeys()
        .args(["zfs-load-key", "--config-file"])
        .arg(config.path())
        .arg("--all")
        .assert()
        .code(1)
        .stdout("")
        .stderr("zkeys: No ZFS datasets are configured\n");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_loads_key_without_newline() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url, Some("pool/test"));
    let zfs = make_mock_zfs();
    let mut command = zkeys();
    configure_mock(&mut command, &zfs, "unavailable");

    let output = command
        .args(["zfs-load-key", "--config-file"])
        .arg(config.path())
        .arg("pool/test")
        .output()
        .unwrap();

    assert_eq!(Some(0), output.status.code());
    assert_eq!(b"", output.stdout.as_slice());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!("zkeys: INFO: Loading key for pool/test\n", stderr);

    assert_eq!(
        "get -H -o value keystatus pool/test
load-key -L prompt pool/test
",
        read(&zfs.log)
    );
    assert_eq!(b"local-secret-service-secret", fs::read(&zfs.key).unwrap().as_slice());
    service.check_request(KEY_ID, "remote-password").await;
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_load_failure_does_not_expose_key() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url, Some("pool/test"));
    let zfs = make_mock_zfs();
    let mut command = zkeys();
    configure_mock(&mut command, &zfs, "unavailable");
    command.env("MOCK_ZFS_LOAD_ERROR", "Injected load failure");

    let output = command
        .args(["zfs-load-key", "--config-file"])
        .arg(config.path())
        .arg("pool/test")
        .output()
        .unwrap();

    assert_eq!(Some(1), output.status.code());
    assert_eq!(b"", output.stdout.as_slice());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        "zkeys: INFO: Loading key for pool/test
zkeys: zfs load-key for pool/test failed: Injected load failure
",
        stderr
    );
    assert!(!stderr.contains("local-secret-service-secret"));
    service.check_request(KEY_ID, "remote-password").await;
    service.shutdown().await;
}

#[test]
fn test_rejects_all_and_dataset() {
    zkeys().args(["zfs-load-key", "--all", "pool/test"]).assert().code(2).stdout("").stderr(
        "Usage error: A dataset and --all cannot be specified together
Type `zkeys help` or `man 8 zkeys` for more information
",
    );
}

#[test]
fn test_requires_selection() {
    zkeys().arg("zfs-load-key").assert().code(2).stdout("").stderr(
        "Usage error: Either a dataset or --all must be specified
Type `zkeys help` or `man 8 zkeys` for more information
",
    );
}

#[test]
fn test_zfs_load_key_help() {
    zkeys()
        .args(["help", "zfs-load-key"])
        .assert()
        .code(0)
        .stdout(
            r#"Usage: zkeys zfs-load-key [options] [dataset]

Options:
    -a, --all           load keys for all configured ZFS datasets
        --config-file FILE
                        path to the configuration file (default:
                        /non-existent/prefix/etc/zkeys.toml)

Arguments:
    [dataset]           ZFS dataset whose key to load

zkeys home page: https://zkeys.jmmv.dev/
"#,
        )
        .stderr("");
}
