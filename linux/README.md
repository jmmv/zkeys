# Linux setup

This assumes a distribution with systemd and dracut and has been tested with
Fedora 44.

The systemd/dracut installation supports LUKS volumes needed before the root
file system is available and thus allows handling full-disk encryption
scenarios.  zkeys installs a dracut module that copies the client and
`/etc/zkeys.toml` into the initramfs.

Fedora packages dracut's NetworkManager integration separately.  Install it
before running `make install` or rebuilding the initramfs:

```sh
$ sudo dnf install dracut-network
```

Then run the installed setup script once:

```sh
$ sudo /usr/local/libexec/zkeys/setup.sh
```

This creates `/etc/zkeys.toml` from the installed example if the file does not
already exist, links the prefix-owned dracut module into
`/usr/lib/dracut/modules.d`, and enables the systemd `keep-alive` systemd
service without starting it.  Think of this as a post-`make install` step to
activate files that don't belong in `PREFIX` because dracut only recognizes
files in the system-managed `/usr/lib/dracut/` directory.

Once that is done, follow these steps:

1.   Start the keep-alive service to ensure keys remain unlocked from now on:

    ```sh
    $ sudo systemctl start zkeys-keep-alive.service
    ```

    Use `journalctl -u zkeys-keep-alive.service` to verify that the service
    succeeds at sending the keep-alive messages.

1.  Find the LUKS mapper name in the first field of its existing
    `/etc/crypttab` entry and associate that name with the ZKeys key for it
    in the `/etc/zkeys.toml` file:

    ```toml
    [luks."<mapper-name>"]
    key = "root-key"
    ```

1.  Ensure that the LUKS volume accepts the full value reconstructed by ZKeys.
    First stage it in a temporary directory:

    ```sh
    $ sudo install -d -m 700 /run/zkeys-enroll
    $ sudo zkeys luks-stage-keys --output-dir /run/zkeys-enroll
    ```

    When onboarding a pre-existing volume key, verify the reconstructed value
    without modifying any key slots:

    ```sh
    $ sudo cryptsetup open --test-passphrase \
        --key-file /run/zkeys-enroll/<mapper-name>.key \
        /dev/disk/by-uuid/<luks-uuid>
    ```

    The command prints nothing on success.  Do not continue until it succeeds.

    For a new ZKeys key, add the staged value to a spare LUKS key slot instead:

    ```sh
    $ sudo cryptsetup luksAddKey /dev/disk/by-uuid/<luks-uuid> \
        /run/zkeys-enroll/<mapper-name>.key
    ```

    In either case, keep the existing passphrase slot as the recovery path and
    remove the staged file when done:

    ```sh
    $ sudo rm /run/zkeys-enroll/<mapper-name>.key
    ```

1.  Update the existing mapper entry in `/etc/crypttab` to include the
    `x-initrd.attach` and `keyfile-erase` options.  Order does not matter.
    A valid entry will look like this:

    ```text
    <mapper-name> UUID=<luks-uuid> none discard,x-initrd.attach,keyfile-erase
    ```

1.  Configure wired DHCP networking during early boot.  On Fedora, use
    `grubby` to add the required options to all installed kernel boot entries
    and rebuild the initramfs:

    ```sh
    $ sudo grubby --update-kernel=ALL --args="rd.neednet=1 ip=dhcp"
    $ sudo dracut --force
    ```

    Other distributions may require different boot loader tooling to persist
    the same `rd.neednet=1 ip=dhcp` kernel command-line options.

    Rebuild the initramfs whenever `/etc/zkeys.toml` changes.

During boot, the initramfs brings up wired DHCP networking and stages keys in
`/run/cryptsetup-keys.d`.  If retrieval fails or takes more than 30 seconds,
the normal LUKS passphrase prompt will show up (and, unless you have unenrolled
your original key, you should be able to log in with both that key and the
ZKeys-managed key).  The `keyfile-erase` option removes each staged key after
the unlock attempt to prevent leaking it to the local system.
