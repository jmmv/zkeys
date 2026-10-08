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

## Installation from source

With a Rust toolchain and GNU Make installed, build and install the client with:

```sh
$ make
$ sudo make install
```

This installs to `/usr/local` by default.  To use another prefix, pass it to
`make`:

```sh
$ make install PREFIX=/opt/local
```

Note that `make install` installs different payloads depending on the host OS,
following common conventions on each platform.  This does _not_ configure the
host to enable any zkeys services, however.

## Documentation

Once `zkeys` is installed, take a look at the `zkeys(8)` and `zkeys.toml(5)`
manual pages for detailed usage and configuration information.

For quick, platform-specific setup notes, see:

*   [freebsd/README.md](freebsd/README.md)
*   [linux/README.md](linux/README.md)
