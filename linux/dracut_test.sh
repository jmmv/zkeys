#!/bin/bash
# shellcheck disable=SC1091,SC2034,SC2329
# zkeys
# Copyright 2026 Julio Merino.
# All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions are
# met:
#
# * Redistributions of source code must retain the above copyright
#   notice, this list of conditions and the following disclaimer.
# * Redistributions in binary form must reproduce the above copyright
#   notice, this list of conditions and the following disclaimer in the
#   documentation and/or other materials provided with the distribution.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
# "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
# LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
# A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
# OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
# SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
# LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
# DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
# THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
# (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

set -eu

srcdir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
workdir=$(mktemp -d)
trap 'rm -rf "${workdir}"' EXIT HUP INT TERM

assert_contains() {
    if ! grep -Fq -- "$2" "$1"; then
        echo "Missing expected call: $2" >&2
        return 1
    fi
}

assert_not_contains() {
    if grep -Fq -- "$2" "$1"; then
        echo "Unexpected call: $2" >&2
        return 1
    fi
}

run_module() (
    dracutsysrootdir=$1
    log=$2
    moddir="$srcdir/linux/dracut/50zkeys"
    systemdsystemunitdir=/usr/lib/systemd/system

    dfatal() {
        printf 'dfatal %s\n' "$*" >>"$log"
    }

    inst_binary() {
        printf 'inst_binary %s\n' "$*" >>"$log"
    }

    inst_dir() {
        printf 'inst_dir %s\n' "$*" >>"$log"
    }

    inst_multiple() {
        printf 'inst_multiple %s\n' "$*" >>"$log"
    }

    inst_simple() {
        printf 'inst_simple %s\n' "$*" >>"$log"
    }

    require_binaries() {
        if [ "$1" = wpa_supplicant ]; then
            [ "${have_wpa:-yes}" = yes ]
        else
            return 0
        fi
    }

    stat() {
        printf '%s\n' "${profile_metadata:-0:600}"
    }

    # shellcheck source=/dev/null
    . "$srcdir/target/linux/dracut/50zkeys/module-setup.sh"
    install
)

root="$workdir/root"
profile="$root/etc/NetworkManager/system-connections/zkeys-initrd.nmconnection"
mkdir -p "$(dirname -- "$profile")"

log="$workdir/without-wifi.log"
: >"$log"
run_module "$root" "$log"
assert_not_contains "$log" 'wpa_supplicant'
assert_contains "$log" 'NetworkManager-wait-online-initrd.service.d/zkeys.conf'
assert_contains "$log" 'nm-wait-online-initrd.service.d/zkeys.conf'

: >"$profile"
log="$workdir/with-wifi.log"
: >"$log"
run_module "$root" "$log"
assert_contains "$log" 'inst_binary wpa_supplicant'
assert_contains "$log" '/usr/share/dbus-1/system-services/fi.w1.wpa_supplicant1.service'
assert_contains "$log" '/usr/lib/systemd/system/wpa_supplicant.service'
assert_contains "$log" 'inst_simple '
assert_contains "$log" '/etc/NetworkManager/system-connections/zkeys-initrd.nmconnection'

log="$workdir/bad-mode.log"
: >"$log"
if profile_metadata=0:644 run_module "$root" "$log"; then
    echo 'Unsafe Wi-Fi profile unexpectedly succeeded' >&2
    exit 1
fi
assert_contains "$log" 'must be owned by root with mode 0600'

log="$workdir/bad-owner.log"
: >"$log"
if profile_metadata=1000:600 run_module "$root" "$log"; then
    echo 'Unowned Wi-Fi profile unexpectedly succeeded' >&2
    exit 1
fi
assert_contains "$log" 'must be owned by root with mode 0600'

log="$workdir/missing-wpa.log"
: >"$log"
if have_wpa=no run_module "$root" "$log"; then
    echo 'Missing wpa_supplicant unexpectedly succeeded' >&2
    exit 1
fi
assert_contains "$log" 'requires wpa_supplicant'

rm "$profile"
ln -s /tmp/not-a-profile "$profile"
log="$workdir/symlink.log"
: >"$log"
if run_module "$root" "$log"; then
    echo 'Symlinked Wi-Fi profile unexpectedly succeeded' >&2
    exit 1
fi
assert_contains "$log" 'must be a regular file'
