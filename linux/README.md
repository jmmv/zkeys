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
 
## Wi-Fi networking

Wi-Fi support is opt-in.  Create a dedicated NetworkManager connection named
`zkeys-initrd` that uses DHCP, connects automatically, and stores its
credentials.  Open, WPA2-Personal, and WPA3-Personal connections are
supported.  Enterprise connections that require 802.1X credentials are not
supported yet.

An easy way to create the dedicated connection is to clone an existing working
profile:

```sh
$ sudo nmcli connection clone "Home Wi-Fi" zkeys-initrd
$ sudo nmcli connection modify zkeys-initrd \
    connection.autoconnect yes connection.permissions "" \
    802-11-wireless-security.psk-flags 0
```

NetworkManager normally stores that profile as
`/etc/NetworkManager/system-connections/zkeys-initrd.nmconnection`.  Confirm
the exact filename and saved secret, and ensure that it is owned by root with
mode `0600`:

```sh
$ sudo nmcli -g connection.filename connection show zkeys-initrd
$ sudo nmcli --show-secrets -g 802-11-wireless-security.psk connection show zkeys-initrd
$ sudo chown root:root /etc/NetworkManager/system-connections/zkeys-initrd.nmconnection
$ sudo chmod 600 /etc/NetworkManager/system-connections/zkeys-initrd.nmconnection
```

If NetworkManager reports a different filename, rename that file to the path
above and run `sudo nmcli connection reload`.  If the secret is empty, use
`sudo nmcli --ask connection up zkeys-initrd` to provide and save it before
rebuilding the initramfs.

The dracut module recognizes that exact filename as the request to include
Wi-Fi support.  It copies the profile and `wpa_supplicant` into the initramfs,
so the latter must be installed on the host.  The profile must contain its
saved secret because no NetworkManager secret agent is available during early
boot.

For Wi-Fi-only boot on Fedora, remove the generic `ip=dhcp` option configured
above while retaining `rd.neednet=1`:

```sh
$ sudo grubby --update-kernel=ALL --remove-args="ip=dhcp"
```

To let wired and Wi-Fi networking race, replace `ip=dhcp` with an explicit
wired interface instead.  For example, after using `nmcli device status` to
identify `enp1s0` as the wired interface, run:

```sh
$ sudo grubby --update-kernel=ALL --remove-args="ip=dhcp"
$ sudo grubby --update-kernel=ALL \
    --args="rd.neednet=1 ip=:::::enp1s0:dhcp"
```

On other distributions, make the equivalent kernel command-line change with
the boot loader's tooling.  Rebuild the image after changing the profile:

```sh
$ sudo dracut --force
```

Use `lsinitrd` to confirm that the image contains the profile,
`wpa_supplicant`, and the Wi-Fi adapter's driver and firmware.  If host-only
generation omits the driver, add it with dracut's `add_drivers` setting and
rebuild the image.

The Wi-Fi passphrase is stored in the initramfs.  Anyone who can read an
unencrypted `/boot` can extract it, just as they can extract the ZKeys client
configuration already stored there.  Keep the local LUKS passphrase slot as a
recovery path: an unavailable access point or invalid Wi-Fi configuration
falls back to the normal prompt after approximately 30 seconds.

The setup script enables the keep-alive service without starting it.  After the
machine boots successfully, start it so the configured keys remain unlocked
and ready for unattended reboots:

```sh
$ sudo systemctl start zkeys-keep-alive.service
```

The service logs to the journal.  Inspect its activity with
`journalctl -u zkeys-keep-alive.service`.  The client retries individual HTTP
failures itself; systemd restarts the process only after an abnormal exit.
