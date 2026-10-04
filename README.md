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
not much here that's ZFS-specific, so presumably this could be adapted to other
unattended boot scenarios.

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

Note that `make install` does different things depending on the host OS because
it sets up all the integration points for automatic key unlocking at boot time
and installs periodic key keep-alives throughout the system's uptime.

## FreeBSD setup

1.  If you don't have `/usr/local/etc/zkeys.toml` yet, use
    `/usr/local/etc/zkeys.toml.tmpl` as your starting template.  Make sure
    the file is root-protected before adding secrets to it.

1.  Once you have added secrets, enable and start the installed service:

    ```sh
    sysrc zkeys_keep_alive_enable=YES
    service zkeys-keep-alive start
    ```

1.  Review `/var/log/zkeys-keep-alive.log` and confirm that the log mentions
    that your keys are being kept alive successfully.  (The keys may have
    been auto-locked before reaching this step, so if you see errors, first
    check that the key is unlocked in the dashboard.)

1.  Associate ZFS encryption roots with their keys in `zkeys.toml`.  Dataset
    names containing slashes must be quoted:

    ```toml
    [zfs."tank/private"]
    key = "private-key"
    ```

1.  Enable and start the ZFS mounting service:

    ```sh
    sysrc zkeys_mount_all_enable=YES
    service zkeys-mount-all start
    ```

    The service loads all configured ZFS keys, mounts all available datasets,
    and shares them.  Do not enable FreeBSD's `zfskeys` service for these
    datasets because it uses the separate ZFS `keylocation` policy.

## Documentation

Once `zkeys` is installed, take a look at the `zkeys(8)` and `zkeys.toml(5)`
manual pages for detailed usage and configuration information.
