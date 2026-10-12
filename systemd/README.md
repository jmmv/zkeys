# systemd setup

This has been tested with Fedora 44.

The systemd installation provides a service that keeps configured keys
unlocked.

Run the installed setup script once:

```sh
$ sudo /usr/local/libexec/zkeys/setup.sh
```

This creates the configured `zkeys.toml` from the installed example if the
file does not already exist and enables the systemd `keep-alive` service
without starting it.

Start the keep-alive service to ensure keys remain unlocked from now on:

```sh
$ sudo systemctl start zkeys-keep-alive.service
```

Use `journalctl -u zkeys-keep-alive.service` to verify that the service
succeeds at sending the keep-alive messages.
