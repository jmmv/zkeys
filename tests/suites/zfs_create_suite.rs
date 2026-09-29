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

//! Integration tests for the `zfs-create` command.

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
printf '<%s>' "$@" >"${MOCK_ZFS_LOG}"
printf '\n' >>"${MOCK_ZFS_LOG}"
cat >"${MOCK_ZFS_KEY}"
if [ -n "${MOCK_ZFS_ERROR:-}" ]; then
    printf '%s\n' "${MOCK_ZFS_ERROR}" >&2
    exit 42
fi
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).unwrap();
    MockZfs { _directory: directory, key, log }
}

/// Writes a configuration for one key and returns it.
fn make_config(service_url: &str) -> NamedTempFile {
    let config = NamedTempFile::new().unwrap();
    fs::write(
        config.path(),
        format!(
            r#"service_url = "{service_url}"

[keys.test]
id = "{KEY_ID}"
remote_password = "remote-password"
local_secret = "local-secret-"

[zfs."pool/test"]
key = "test"
"#
        ),
    )
    .unwrap();
    config
}

/// Adds the environment needed to use a mock ZFS executable.
fn configure_mock(command: &mut assert_cmd::Command, zfs: &MockZfs) {
    let path = env::join_paths(
        std::iter::once(zfs._directory.path().to_path_buf())
            .chain(env::split_paths(&env::var_os("PATH").unwrap())),
    )
    .unwrap();
    command.env("PATH", path).env("MOCK_ZFS_KEY", &zfs.key).env("MOCK_ZFS_LOG", &zfs.log);
}

/// Returns the contents of `path`.
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

/// Returns the full-key warning logged by a successful command.
fn key_warning(secret: &str) -> String {
    format!(
        "zkeys: WARN: THIS IS YOUR FULL KEY:\n\
zkeys: WARN: \n\
zkeys: WARN:     {secret}\n\
zkeys: WARN: \n\
zkeys: WARN: BACK IT UP NOW.\n"
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_create() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url);
    let zfs = make_mock_zfs();
    let mut command = zkeys();
    configure_mock(&mut command, &zfs);

    command
        .args(["zfs-create", "--config-file"])
        .arg(config.path())
        .args(["--", "-p", "-o", "compression=zstd", "pool/test"])
        .assert()
        .code(0)
        .stdout("")
        .stderr(format!(
            "zkeys: INFO: Loading key test
zkeys: INFO: Running: zfs create -o encryption=on -o keyformat=passphrase -o \
keylocation=prompt -p -o compression=zstd pool/test\n{}",
            key_warning("local-secret-service-secret")
        ));

    assert_eq!(
        "<create><-o><encryption=on><-o><keyformat=passphrase><-o>\
<keylocation=prompt><-p><-o><compression=zstd><pool/test>\n",
        read(&zfs.log)
    );
    assert_eq!("local-secret-service-secret", read(&zfs.key));
    service.check_request(KEY_ID, "remote-password").await;
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_failure_does_not_print_key_warning() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url);
    let zfs = make_mock_zfs();
    let mut command = zkeys();
    configure_mock(&mut command, &zfs);

    command
        .env("MOCK_ZFS_ERROR", "Injected failure")
        .args(["zfs-create", "--config-file"])
        .arg(config.path())
        .arg("pool/test")
        .assert()
        .code(1)
        .stdout("")
        .stderr(
            "zkeys: INFO: Loading key test
zkeys: INFO: Running: zfs create -o encryption=on -o keyformat=passphrase \
-o keylocation=prompt pool/test\nInjected failure\nzkeys: zfs create failed: exit status: 42\n",
        );

    service.check_request(KEY_ID, "remote-password").await;
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_retrieval_failure_does_not_execute_zfs() {
    let mut service = MockService::start(StatusCode::FORBIDDEN, "Access denied: Key locked").await;
    let config = make_config(&service.url);
    let zfs = make_mock_zfs();
    let mut command = zkeys();
    configure_mock(&mut command, &zfs);

    command
        .args(["zfs-create", "--config-file"])
        .arg(config.path())
        .arg("pool/test")
        .assert()
        .code(1)
        .stdout("")
        .stderr(
            "zkeys: INFO: Loading key test
zkeys: Service returned HTTP 403 Forbidden: Access denied: Key locked\n",
        );

    assert!(!zfs.log.exists());
    assert!(!zfs.key.exists());
    service.check_request(KEY_ID, "remote-password").await;
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unconfigured_dataset_does_not_execute_zfs() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url);
    let zfs = make_mock_zfs();
    let mut command = zkeys();
    configure_mock(&mut command, &zfs);

    command
        .args(["zfs-create", "--config-file"])
        .arg(config.path())
        .arg("pool/unknown")
        .assert()
        .code(1)
        .stdout("")
        .stderr("zkeys: No ZFS dataset named pool/unknown is configured\n");

    assert!(!zfs.log.exists());
    assert!(!zfs.key.exists());
    service.check_no_requests().await;
    service.shutdown().await;
}

#[test]
fn test_zfs_create_missing_arguments() {
    zkeys().arg("zfs-create").assert().code(2).stdout("").stderr(
        r#"Usage error: Trailing argument `zfs-arg` requires at least 1 value
Type `zkeys help` or `man 8 zkeys` for more information
"#,
    );
}

#[test]
fn test_zfs_create_help() {
    zkeys()
        .args(["help", "zfs-create"])
        .assert()
        .code(0)
        .stdout(
            r#"Usage: zkeys zfs-create [options] zfs-arg1 [.. zfs-argN]

Options:
    --quiet             do not print the full key after success
    --config-file FILE  path to the configuration file (default:
                        /non-existent/prefix/etc/zkeys.toml)

Arguments:
    zfs-arg1 [.. zfs-argN]
                        arguments to pass to zfs

The final zfs-arg is the configured ZFS dataset name.
Pass `--` before zfs-arg arguments that begin with a hyphen.

zkeys home page: https://zkeys.jmmv.dev/
"#,
        )
        .stderr("");
}
