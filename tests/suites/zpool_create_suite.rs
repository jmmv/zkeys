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

//! Integration tests for the `zpool-create` command.

use crate::common::mocks::MockService;
use crate::common::zkeys;
use assert_cmd::Command;
use axum::http::StatusCode;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::{NamedTempFile, TempDir};
use uuid::{Uuid, uuid};

const KEY_ID: Uuid = uuid!("6de5a8c5-3549-4b4a-b6d2-daca1ec29012");

/// Creates a mock zpool executable that records arguments and standard input.
fn make_mock_zpool() -> TempDir {
    let tempdir = TempDir::new().unwrap();
    let executable = tempdir.path().join("zpool");
    fs::write(
        &executable,
        r#"#!/bin/sh
set -eu
printf '<%s>' "$@" >"${MOCK_ZPOOL_LOG}"
printf '\n' >>"${MOCK_ZPOOL_LOG}"
cat >"${MOCK_ZPOOL_KEY}"
if [ -n "${MOCK_ZPOOL_ERROR:-}" ]; then
    printf '%s\n' "${MOCK_ZPOOL_ERROR}" >&2
    exit 42
fi
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).unwrap();
    tempdir
}

/// Constructs a `Command` to execute zkeys with a zpool mock.
fn zkeys_with_zpool() -> (TempDir, Command) {
    let tempdir = make_mock_zpool();

    let path = env::join_paths(
        std::iter::once(tempdir.path().to_path_buf())
            .chain(env::split_paths(&env::var_os("PATH").unwrap())),
    )
    .unwrap();

    let mut command = zkeys();
    command
        .env("PATH", path)
        .env("MOCK_ZPOOL_KEY", tempdir.path().join("key"))
        .env("MOCK_ZPOOL_LOG", tempdir.path().join("log"));

    (tempdir, command)
}

/// Writes a configuration with a key for the pool's root file system.
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

[zfs.pool]
key = "test"
"#
        ),
    )
    .unwrap();
    config
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_create() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url);
    let (tempdir, mut command) = zkeys_with_zpool();

    command
        .args(["zpool-create", "--quiet", "--config-file"])
        .arg(config.path())
        .args(["--", "-f", "-o", "ashift=12", "-O", "compression=zstd", "pool", "mirror", "disk1", "disk2"])
        .assert()
        .code(0)
        .stdout("")
        .stderr(
            "zkeys: INFO: Loading key test\nzkeys: INFO: Running: zpool create -O encryption=on -O keyformat=passphrase -O keylocation=prompt -f -o ashift=12 -O compression=zstd pool mirror disk1 disk2\n",
        );

    assert_eq!(
        "<create><-O><encryption=on><-O><keyformat=passphrase><-O><keylocation=prompt><-f><-o><ashift=12><-O><compression=zstd><pool><mirror><disk1><disk2>\n",
        fs::read_to_string(tempdir.path().join("log")).unwrap()
    );
    assert_eq!(
        "local-secret-service-secret",
        fs::read_to_string(tempdir.path().join("key")).unwrap()
    );
    service.check_request(KEY_ID, "remote-password").await;
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unconfigured_pool_does_not_execute_zpool() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url);
    let (tempdir, mut command) = zkeys_with_zpool();

    command
        .args(["zpool-create", "--config-file"])
        .arg(config.path())
        .args(["unknown", "disk"])
        .assert()
        .code(1)
        .stdout("")
        .stderr("zkeys: No ZFS dataset named unknown is configured\n");
    assert!(!tempdir.path().join("log").exists());
    service.check_no_requests().await;
    service.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_failure_does_not_print_key_warning() {
    let mut service = MockService::start(StatusCode::OK, "service-secret").await;
    let config = make_config(&service.url);
    let (_tempdir, mut command) = zkeys_with_zpool();

    command
        .env("MOCK_ZPOOL_ERROR", "Injected failure")
        .args(["zpool-create", "--config-file"])
        .arg(config.path())
        .args(["pool", "disk"])
        .assert()
        .code(1)
        .stdout("")
        .stderr(
            "zkeys: INFO: Loading key test\nzkeys: INFO: Running: zpool create -O encryption=on -O keyformat=passphrase -O keylocation=prompt pool disk\nInjected failure\nzkeys: zpool create failed: exit status: 42\n",
        );

    service.check_request(KEY_ID, "remote-password").await;
    service.shutdown().await;
}

#[test]
fn test_zpool_create_help() {
    zkeys()
        .args(["help", "zpool-create"])
        .assert()
        .code(0)
        .stdout(
            r#"Usage: zkeys zpool-create [options] zpool-arg1 [.. zpool-argN]

Options:
    --quiet             do not print the full key after success
    --config-file FILE  path to the configuration file (default:
                        /non-existent/prefix/etc/zkeys.toml)

Arguments:
    zpool-arg1 [.. zpool-argN]
                        arguments to pass to zpool

The pool name after zpool options must be a configured ZFS dataset.
Pass `--` before zpool-arg arguments that begin with a hyphen.

zkeys home page: https://zkeys.jmmv.dev/
"#,
        )
        .stderr("");
}
