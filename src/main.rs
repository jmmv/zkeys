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
use std::io::{self, Write};
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

/// Adds the configuration file option to a command.
fn config_setup(builder: CommandBuilder) -> CommandBuilder {
    let paths = Paths::default();
    builder.optopt(
        "",
        "config-file",
        &format!("path to the configuration file (default: {})", paths.config_file.display()),
        "FILE",
    )
}

/// Loads the configuration selected by a command's options.
fn load_config(matches: &Matches) -> Result<Config> {
    let paths = Paths::default().with_overrides(matches);
    Config::parse(&paths.config_file).with_context(|| {
        format!("Failed to load configuration file {}", paths.config_file.display())
    })
}

/// Adds the options for the `list-keys` command.
fn list_keys_setup(builder: CommandBuilder) -> CommandBuilder {
    config_setup(builder)
}

/// Runs the `list-keys` command.
async fn list_keys_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    let config = load_config(&command_matches)?;
    let mut keys: Vec<_> = config.keys.keys().collect();
    keys.sort();
    for key in keys {
        println!("{key}");
    }
    Ok(0)
}

/// Adds the options for the `zfs-list` command.
fn zfs_list_setup(builder: CommandBuilder) -> CommandBuilder {
    config_setup(builder)
}

/// Runs the `zfs-list` command.
async fn zfs_list_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    let config = load_config(&command_matches)?;
    let mut datasets: Vec<_> = config.zfs.iter().collect();
    datasets.sort_by_key(|(dataset, _)| *dataset);
    for (dataset, zfs) in datasets {
        println!("{dataset} {}", zfs.key);
    }
    Ok(0)
}

/// Adds the positional arguments for the `get-key` command.
fn get_key_setup(builder: CommandBuilder) -> CommandBuilder {
    config_setup(builder).posarg("name", "name of the key to retrieve")
}

/// Runs the `get-key` command.
async fn get_key_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    init_env_logger(env!("CARGO_BIN_NAME"));
    let config = load_config(&command_matches)?;
    get_key(config, command_matches.arg_pos("name")).await?;
    Ok(0)
}

/// Adds the options and arguments for the `keep-alive` command.
fn keep_alive_setup(builder: CommandBuilder) -> CommandBuilder {
    config_setup(builder).trailarg(
        "name",
        0,
        usize::MAX,
        "names of the keys to keep alive (default: all)",
    )
}

/// Runs the `keep-alive` command.
async fn keep_alive_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let config = load_config(&command_matches)?;
    keep_alive(config, command_matches.arg_trail()).await?;
    Ok(0)
}

/// Adds the options for the `luks-stage-keys` command.
fn luks_stage_keys_setup(builder: CommandBuilder) -> CommandBuilder {
    config_setup(builder.optopt(
        "",
        "output-dir",
        "directory in which to stage LUKS keys (default: /run/cryptsetup-keys.d)",
        "DIR",
    ))
}

/// Runs the `luks-stage-keys` command.
async fn luks_stage_keys_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    init_env_logger(env!("CARGO_BIN_NAME"));
    let config = load_config(&command_matches)?;
    let output_dir = command_matches
        .opt_str("output-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/run/cryptsetup-keys.d"));
    luks_stage_keys(config, &output_dir).await?;
    Ok(0)
}

/// Adds the options and arguments for the `zfs-load-key` command.
fn zfs_load_key_setup(builder: CommandBuilder) -> CommandBuilder {
    config_setup(builder.optflag("a", "all", "load keys for all configured ZFS datasets")).trailarg(
        "dataset",
        0,
        1,
        "ZFS dataset whose key to load",
    )
}

/// Runs the `zfs-load-key` command.
async fn zfs_load_key_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    init_env_logger(env!("CARGO_BIN_NAME"));
    let all = command_matches.opt_present("all");
    let dataset = command_matches.arg_trail().first().map(String::as_str);
    let dataset = match (all, dataset) {
        (false, None) => {
            return Err(bad_usage!("Either a dataset or --all must be specified").into());
        }
        (false, Some(dataset)) => Some(dataset),
        (true, None) => None,
        (true, Some(_)) => {
            return Err(bad_usage!("A dataset and --all cannot be specified together").into());
        }
    };

    let config = load_config(&command_matches)?;
    zfs_load_key(config, dataset).await?;
    Ok(0)
}

/// Prints additional help for commands that forward arguments to ZFS.
fn zfs_extra_help(writer: &mut dyn Write) -> io::Result<()> {
    writeln!(writer, "The final zfs-arg is the configured ZFS dataset name.")?;
    writeln!(writer, "Pass `--` before zfs-arg arguments that begin with a hyphen.")
}

/// Adds the common options and arguments for a ZFS command that receives a key.
fn zfs_key_input_setup(builder: CommandBuilder) -> CommandBuilder {
    config_setup(builder.optflag("", "quiet", "do not print the full key after success"))
        .trailarg("zfs-arg", 1, usize::MAX, "arguments to pass to zfs")
        .extra_help(zfs_extra_help)
}

/// Prints additional help for the `zpool-create` command.
fn zpool_extra_help(writer: &mut dyn Write) -> io::Result<()> {
    writeln!(writer, "The pool name after zpool options must be a configured ZFS dataset.")?;
    writeln!(writer, "Pass `--` before zpool-arg arguments that begin with a hyphen.")
}

/// Adds the options and arguments for the `zpool-create` command.
fn zpool_create_setup(builder: CommandBuilder) -> CommandBuilder {
    config_setup(builder.optflag("", "quiet", "do not print the full key after success"))
        .trailarg("zpool-arg", 1, usize::MAX, "arguments to pass to zpool")
        .extra_help(zpool_extra_help)
}

/// Runs the `zfs-change-key` command.
async fn zfs_change_key_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    init_env_logger(env!("CARGO_BIN_NAME"));
    let config = load_config(&command_matches)?;
    zfs_change_key(config, command_matches.opt_present("quiet"), command_matches.arg_trail())
        .await?;
    Ok(0)
}

/// Runs the `zfs-create` command.
async fn zfs_create_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    init_env_logger(env!("CARGO_BIN_NAME"));
    let config = load_config(&command_matches)?;
    zfs_create(config, command_matches.opt_present("quiet"), command_matches.arg_trail()).await?;
    Ok(0)
}

/// Runs the `zpool-create` command.
async fn zpool_create_main(_app_matches: Matches, command_matches: Matches) -> Result<i32> {
    init_env_logger(env!("CARGO_BIN_NAME"));
    let config = load_config(&command_matches)?;
    zpool_create(config, command_matches.opt_present("quiet"), command_matches.arg_trail()).await?;
    Ok(0)
}

/// Configures the command-line application and its subcommands.
fn app_setup(builder: Builder) -> Builder {
    builder
        .copyright("Copyright 2025-2026 Julio Merino")
        .disable_init_env_logger()
        .homepage(env!("CARGO_PKG_HOMEPAGE"))
        .manpage(env!("CARGO_BIN_NAME"), "8")
        .cmd_async("get-key", "retrieve a key", get_key_setup, get_key_main)
        .cmd_async("list-keys", "list configured keys", list_keys_setup, list_keys_main)
        .cmd_async("keep-alive", "periodically keep keys alive", keep_alive_setup, keep_alive_main)
        .cmd_async(
            "luks-stage-keys",
            "retrieve and stage configured LUKS keys",
            luks_stage_keys_setup,
            luks_stage_keys_main,
        )
        .cmd_async(
            "zfs-change-key",
            "change a ZFS encryption key",
            zfs_key_input_setup,
            zfs_change_key_main,
        )
        .cmd_async(
            "zfs-create",
            "create an encrypted ZFS dataset",
            zfs_key_input_setup,
            zfs_create_main,
        )
        .cmd_async(
            "zfs-load-key",
            "load a key for a ZFS dataset",
            zfs_load_key_setup,
            zfs_load_key_main,
        )
        .cmd_async("zfs-list", "list configured ZFS datasets", zfs_list_setup, zfs_list_main)
        .cmd_async(
            "zpool-create",
            "create an encrypted ZFS pool",
            zpool_create_setup,
            zpool_create_main,
        )
}

tokio_app!("zkeys", app_setup, tokio_command_dispatcher);
