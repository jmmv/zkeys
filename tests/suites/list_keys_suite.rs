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

//! Integration tests for the `list-keys` command.

use crate::common::zkeys;
use std::fs;
use tempfile::NamedTempFile;

#[test]
fn test_list_keys() {
    let config = NamedTempFile::new().unwrap();
    fs::write(
        config.path(),
        r#"
[keys.zulu]
id = "00000000-0000-0000-0000-000000000002"
remote_password = "password"
local_secret = "secret"

[keys.alpha]
id = "00000000-0000-0000-0000-000000000001"
remote_password = "password"
local_secret = "secret"
"#,
    )
    .unwrap();

    zkeys()
        .args(["list-keys", "--config-file"])
        .arg(config.path())
        .assert()
        .code(0)
        .stdout("alpha\nzulu\n")
        .stderr("");
}

#[test]
fn test_list_keys_invalid_config() {
    let config = NamedTempFile::new().unwrap();
    fs::write(config.path(), "this is not = valid TOML =").unwrap();

    let output = zkeys().args(["list-keys", "--config-file"]).arg(config.path()).output().unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert_eq!(Some(1), output.status.code());
    assert_eq!("", String::from_utf8(output.stdout).unwrap());
    assert!(stderr.starts_with(&format!(
        "zkeys: Failed to load configuration file {}: ",
        config.path().display()
    )));
    assert!(stderr.contains("TOML parse error"));
}

#[test]
fn test_list_keys_unknown_key() {
    let config = NamedTempFile::new().unwrap();
    fs::write(config.path(), "[zfs.dataset]\nkey = \"missing\"\n").unwrap();

    zkeys()
        .args(["list-keys", "--config-file"])
        .arg(config.path())
        .assert()
        .code(1)
        .stdout("")
        .stderr(format!(
            "zkeys: Failed to load configuration file {}: ZFS dataset dataset references undefined key missing\n",
            config.path().display()
        ));
}

#[test]
fn test_list_keys_extra_argument() {
    zkeys().args(["list-keys", "key"]).assert().code(2).stdout("").stderr(
        "Usage error: Too many arguments
Type `zkeys help` or `man 8 zkeys` for more information
",
    );
}

#[test]
fn test_list_keys_help() {
    zkeys()
        .args(["help", "list-keys"])
        .assert()
        .code(0)
        .stdout(
            r#"Usage: zkeys list-keys [options]

Options:
    --config-file FILE  path to the configuration file (default:
                        /non-existent/prefix/etc/zkeys.toml)

zkeys home page: https://zkeys.jmmv.dev/
"#,
        )
        .stderr("");
}
