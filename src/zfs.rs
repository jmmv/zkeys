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

//! Utilities to call into the `zfs` tool.

use log::{info, warn};
use std::io;
use std::process::{ExitStatus, Output, Stdio};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Status of a ZFS dataset's encryption key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KeyStatus {
    Available,
    Unavailable,
}

/// Interface to the ZFS command-line utility.
pub(crate) trait Zfs {
    /// Creates an encrypted dataset using `key`.
    async fn create(&self, key: &str, args: &[String]) -> io::Result<()>;

    /// Returns the key status of `dataset`.
    async fn key_status(&self, dataset: &str) -> io::Result<KeyStatus>;

    /// Loads `key` for `dataset`.
    async fn load_key(&self, dataset: &str, key: &str) -> io::Result<()>;
}

/// ZFS implementation backed by the command-line utility.
pub(crate) struct CommandZfs;

/// Builds an error for an unsuccessful ZFS subprocess.
fn command_error(operation: &str, output: Output) -> io::Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    let message = if stderr.is_empty() {
        format!("{operation} failed with {}", output.status)
    } else {
        format!("{operation} failed: {stderr}")
    };
    io::Error::other(message)
}

/// Warns the user to back up the full key.
pub(crate) fn print_key_warning(secret: &str) {
    warn!("THIS IS YOUR FULL KEY:");
    warn!("");
    warn!("    {secret}");
    warn!("");
    warn!("BACK IT UP NOW.");
}

/// Runs a ZFS operation and provides `key` on its standard input.
async fn run_with_key(args: &[String], operation: &str, key: &str) -> io::Result<ExitStatus> {
    info!("Running: zfs {}", args.to_vec().join(" "));
    let mut command = Command::new("zfs");
    command.args(args);
    command.stdin(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        io::Error::new(error.kind(), format!("Failed to execute {operation}: {error}"))
    })?;

    let mut stdin = child.stdin.take().expect("Piped stdin is missing");
    let write_result = stdin.write_all(key.as_bytes()).await;
    drop(stdin);
    let wait_result = child.wait().await;

    write_result.map_err(|error| {
        io::Error::new(error.kind(), format!("Failed to provide the key to {operation}: {error}"))
    })?;
    wait_result.map_err(|error| {
        io::Error::new(error.kind(), format!("Failed to wait for {operation}: {error}"))
    })
}

impl Zfs for CommandZfs {
    async fn create(&self, key: &str, args: &[String]) -> io::Result<()> {
        let args = [
            "create",
            "-o",
            "encryption=on",
            "-o",
            "keyformat=passphrase",
            "-o",
            "keylocation=prompt",
        ]
        .into_iter()
        .map(str::to_owned)
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
        let status = run_with_key(&args, "zfs create", key).await?;
        if !status.success() {
            return Err(io::Error::other(format!("zfs create failed: {status}")));
        }
        Ok(())
    }

    async fn key_status(&self, dataset: &str) -> io::Result<KeyStatus> {
        let output = Command::new("zfs")
            .args(["get", "-H", "-o", "value", "keystatus", dataset])
            .output()
            .await
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("Failed to execute zfs get for {dataset}: {error}"),
                )
            })?;
        if !output.status.success() {
            return Err(command_error(&format!("zfs get for {dataset}"), output));
        }

        match String::from_utf8_lossy(&output.stdout).trim() {
            "available" => Ok(KeyStatus::Available),
            "unavailable" => Ok(KeyStatus::Unavailable),
            status => Err(io::Error::other(format!(
                "Unexpected key status {status:?} for ZFS dataset {dataset}"
            ))),
        }
    }

    async fn load_key(&self, dataset: &str, key: &str) -> io::Result<()> {
        let args = ["load-key", "-L", "prompt", dataset].map(str::to_owned);
        let operation = format!("zfs load-key for {dataset}");
        let status = run_with_key(&args, &operation, key).await?;
        if !status.success() {
            return Err(io::Error::other(format!("{operation} failed with {status}")));
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod testutils {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    type KeyStatusMock = (String, io::Result<KeyStatus>);
    type LoadKeyMock = ((String, String), io::Result<()>);
    type WithKeyMock = ((String, Vec<String>), io::Result<()>);

    /// ZFS implementation that returns predefined results for expected requests.
    #[derive(Default)]
    pub(crate) struct MockZfs {
        create: RefCell<VecDeque<WithKeyMock>>,
        key_status: RefCell<VecDeque<KeyStatusMock>>,
        load_key: RefCell<VecDeque<LoadKeyMock>>,
    }

    impl MockZfs {
        /// Records an expected create request and its result.
        pub(crate) fn add_create(&self, key: &str, args: &[&str], result: io::Result<()>) {
            self.create.borrow_mut().push_back((
                (key.to_owned(), args.iter().map(|arg| (*arg).to_owned()).collect()),
                result,
            ));
        }

        /// Records an expected key-status request and its result.
        pub(crate) fn add_key_status(&self, dataset: &str, result: io::Result<KeyStatus>) {
            self.key_status.borrow_mut().push_back((dataset.to_owned(), result));
        }

        /// Records an expected load-key request and its result.
        pub(crate) fn add_load_key(&self, dataset: &str, key: &str, result: io::Result<()>) {
            self.load_key.borrow_mut().push_back(((dataset.to_owned(), key.to_owned()), result));
        }
    }

    impl Drop for MockZfs {
        fn drop(&mut self) {
            assert!(self.create.borrow().is_empty(), "Mock requests not fully consumed");
            assert!(self.key_status.borrow().is_empty(), "Mock requests not fully consumed");
            assert!(self.load_key.borrow().is_empty(), "Mock requests not fully consumed");
        }
    }

    impl Zfs for MockZfs {
        async fn create(&self, key: &str, args: &[String]) -> io::Result<()> {
            let mock = self.create.borrow_mut().pop_front().expect("No mock request available");
            assert_eq!(mock.0.0, key);
            assert_eq!(mock.0.1, args);
            mock.1
        }

        async fn key_status(&self, dataset: &str) -> io::Result<KeyStatus> {
            let mock = self.key_status.borrow_mut().pop_front().expect("No mock request available");
            assert_eq!(mock.0, dataset);
            mock.1
        }

        async fn load_key(&self, dataset: &str, key: &str) -> io::Result<()> {
            let mock = self.load_key.borrow_mut().pop_front().expect("No mock request available");
            assert_eq!(mock.0.0, dataset);
            assert_eq!(mock.0.1, key);
            mock.1
        }
    }
}
