# ZKeys: Remote key custody for unattended infrastructure

This repository contains the `zkeys` tool: a CLI application to access secrets
managed by a ZKeys Server.

The goal of this client is to facilitate unlocking encrypted pools and file
systems in an unattended fashion by leveraging offsite keys that are never
stored on the machine where they are used.  This protects the encrypted volumes
in case of physical theft.

For the reasons above, `zkeys` does intentionally _not_ provide key management
features and restricts those to the frontend UI.  This is to prevent leaving
administrative keys on the machine by mistake, as access to these would defeat
the whole purpose of ZKeys.

ZKeys is named after ZFS as this project originated with the desire to automate
unattended boots of FreeBSD machines in a secure fashion.  However, there is
not much here that's ZFS-specific and, in fact, zkeys supports Linux systems
with dracut as well (such as Fedora) for unattended boot scenarios.

Visit <https://zkeys.jmmv.dev/> for more details on the product.

See the [release notes](NEWS.md) for information about the changes in this and
all previous releases.

## Installation

### From a release archive (preferred)

Install a Rust toolchain and GNU Make, and then run:

```sh
$ mkdir build && cd build
$ ../configure
$ make
$ sudo make install
```

This installs to `/usr/local` by default.  To use another prefix, specify it
when configuring the build:

```sh
$ ../configure --prefix=/opt/local
$ make
$ sudo make install
```

The configure script selects the Linux or FreeBSD installation payload from
the host operating system.  Packagers can override this with
`--with-platform=linux` or `--with-platform=freebsd`.  This does _not_
configure the host to enable any zkeys services, however.

### From a Git checkout

Install GNU Autoconf and GNU Automake, and then run:

```sh
$ autoreconf -is
```

Afterward, follow the installation instructions for a release archive above.

## Documentation

Once `zkeys` is installed, take a look at the `zkeys(8)` and `zkeys.toml(5)`
manual pages for detailed usage and configuration information.

For quick, platform-specific setup notes, see:

*   [freebsd/README.md](freebsd/README.md)
*   [linux/README.md](linux/README.md)
