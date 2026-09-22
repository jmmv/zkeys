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

//! Command-line interface for the ZKeys service.

use anyhow::Context;
use getoptsargs::prelude::*;
use std::env;
use std::path::PathBuf;
use zkeys::*;

/// Paths describing a zkeys installation.
struct Paths {
    /// Path to the configuration file.
    config_file: PathBuf,
}

impl Default for Paths {
    /// Instantiates a set of default paths.
    fn default() -> Self {
        let prefix = env::var("ZKEYS_TEST_PREFIX").map(PathBuf::from).unwrap_or(
            option_env!("PREFIX").map(PathBuf::from).unwrap_or(PathBuf::from("/usr/local")),
        );
        Self { config_file: prefix.join("etc/zkeys.toml") }
    }
}

impl Paths {
    /// Applies configuration options to a set of paths.
    fn with_overrides(mut self, matches: &Matches) -> Self {
        if let Some(config_file) = matches.opt_str("config-file").map(PathBuf::from) {
            self.config_file = config_file;
        }
        self
    }
}

/// Adds the positional arguments for the `get-key` command.
fn get_key_setup(builder: CommandBuilder) -> CommandBuilder {
    let paths = Paths::default();
    builder
        .optopt(
            "",
            "config-file",
            &format!("path to the configuration file (default: {})", paths.config_file.display()),
            "FILE",
        )
        .posarg("name", "name of the key to retrieve")
}

/// Runs the `get-key` command.
async fn get_key_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    let paths = Paths::default().with_overrides(&command_matches);
    let config = Config::parse(&paths.config_file).with_context(|| {
        format!("Failed to load configuration file {}", paths.config_file.display())
    })?;
    get_key(config, command_matches.arg_pos("name")).await?;
    Ok(0)
}

/// Configures the command-line application and its subcommands.
fn app_setup(builder: Builder) -> Builder {
    builder
        .copyright("Copyright 2025-2026 Julio Merino")
        .homepage(env!("CARGO_PKG_HOMEPAGE"))
        .manpage(env!("CARGO_BIN_NAME"), "8")
        .cmd_async("get-key", "retrieve a key", get_key_setup, get_key_main)
}

tokio_app!("zkeys", app_setup, tokio_command_dispatcher);
