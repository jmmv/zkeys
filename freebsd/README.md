# FreeBSD setup

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
