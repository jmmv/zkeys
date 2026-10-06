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

//! Integration tests for top-level command-line behavior.

use crate::common::zkeys;

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
    keep-alive          periodically keep keys alive
    list-keys           list configured keys
    version             show version information
    zfs-change-key      change a ZFS encryption key
    zfs-create          create an encrypted ZFS dataset
    zfs-list            list configured ZFS datasets
    zfs-load-key        load a key for a ZFS dataset
    zpool-create        create an encrypted ZFS pool

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
