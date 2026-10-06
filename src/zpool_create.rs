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
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! `zpool-create` command implementation.

use crate::Config;
use crate::get_key::get_key_internal;
use crate::service::HttpService;
use crate::zfs::{print_key_warning, run_with_key};
use getopts::{Options, ParsingStyle};
use log::info;
use std::io;

/// Finds the pool name after the options accepted by `zpool create`.
fn pool_name(args: &[String]) -> io::Result<String> {
    let mut options = Options::new();
    for flag in ["d", "f", "n"] {
        options.optflagmulti(flag, "", "");
    }
    for option in ["m", "o", "O", "R", "t"] {
        options.optmulti(option, "", "", "VALUE");
    }
    options.parsing_style(ParsingStyle::StopAtFirstFree);
    let matches =
        options.parse(args).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    matches
        .free
        .into_iter()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No ZFS pool was specified"))
}

/// Creates a pool with encryption properties on its root file system.
pub async fn zpool_create(config: Config, quiet: bool, args: &[String]) -> io::Result<()> {
    let pool = pool_name(args)?;

    let mapping = config.zfs.get(&pool).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("No ZFS dataset named {pool} is configured"),
        )
    })?;

    let service = HttpService::new(config.service_url.clone())?;

    info!("Loading key {}", mapping.key);
    let key = get_key_internal(&config.keys, &mapping.key, &service).await?;
    let command_args =
        ["create", "-O", "encryption=on", "-O", "keyformat=passphrase", "-O", "keylocation=prompt"]
            .into_iter()
            .map(str::to_owned)
            .chain(args.iter().cloned())
            .collect::<Vec<_>>();

    let status = run_with_key("zpool", &command_args, "zpool create", &key).await?;
    if !status.success() {
        return Err(io::Error::other(format!("zpool create failed: {status}")));
    }

    if !quiet {
        print_key_warning(&key);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_name_after_options() {
        let args = [
            "-fn",
            "-o",
            "ashift=12",
            "-Ocompression=zstd",
            "-R",
            "/mnt",
            "pool",
            "mirror",
            "disk1",
            "disk2",
        ]
        .map(str::to_owned);
        assert_eq!("pool", pool_name(&args).unwrap());
        assert_eq!("pool", pool_name(&["pool", "-unknown"].map(str::to_owned)).unwrap());
        assert_eq!("-pool", pool_name(&["--", "-pool"].map(str::to_owned)).unwrap());
    }

    #[test]
    fn test_pool_name_errors() {
        assert_eq!("No ZFS pool was specified", pool_name(&[]).unwrap_err().to_string());
        assert_eq!(
            "Argument to option 'o' missing",
            pool_name(&["-o".to_owned()]).unwrap_err().to_string()
        );
        assert_eq!(
            "Unrecognized option: 'x'",
            pool_name(&["-x".to_owned(), "pool".to_owned()]).unwrap_err().to_string()
        );
    }
}
