#!/bin/sh
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

BINDIR="${DESTDIR:-}${BINDIR?}"
PREFIX="${DESTDIR:-}${PREFIX?}"
SYSCONFDIR="${DESTDIR:-}${SYSCONFDIR?}"

install() {
    echo "install ${*}" 1>&2
    command install "${@}"
}

install -m 755 -d "${BINDIR}"
install -m 755 target/release/zkeys "${BINDIR}/zkeys"

install -m 755 -d "${PREFIX}/share/examples/zkeys"
install -m 600 zkeys.toml.tmpl "${PREFIX}/share/examples/zkeys/zkeys.toml"

install -m 755 -d "${PREFIX}/share/man/man5"
install -m 644 man/zkeys.toml.5 "${PREFIX}/share/man/man5/zkeys.toml.5"
install -m 755 -d "${PREFIX}/share/man/man8"
install -m 644 target/zkeys.8 "${PREFIX}/share/man/man8/zkeys.8"

install -m 755 -d "${PREFIX}/share/doc/zkeys"
install -m 644 LICENSE "${PREFIX}/share/doc/zkeys/LICENSE"
install -m 644 NOTICE "${PREFIX}/share/doc/zkeys/NOTICE"
install -m 644 README.md "${PREFIX}/share/doc/zkeys/README.md"
install -m 644 linux/README.md "${PREFIX}/share/doc/zkeys/README.linux.md"

# dracut does NOT recognize files under PREFIX unless PREFIX=/usr.
# That's OK: we handle that in our post-install setup.sh script because
# we want to keep installation here completely PREFIX-clean.
DRACUT_MODULEDIR="${PREFIX}/lib/dracut/modules.d"
install -m 755 -d "${DRACUT_MODULEDIR}/50zkeys"
install -m 755 target/linux/dracut/50zkeys/module-setup.sh \
    "${DRACUT_MODULEDIR}/50zkeys/module-setup.sh"
install -m 644 linux/dracut/50zkeys/zkeys-cryptsetup.conf \
    "${DRACUT_MODULEDIR}/50zkeys/zkeys-cryptsetup.conf"
install -m 644 linux/dracut/50zkeys/zkeys-initrd.service \
    "${DRACUT_MODULEDIR}/50zkeys/zkeys-initrd.service"
install -m 644 linux/dracut/50zkeys/zkeys-network-online.conf \
    "${DRACUT_MODULEDIR}/50zkeys/zkeys-network-online.conf"

install -m 755 -d "${PREFIX}/libexec/zkeys"
install -m 755 target/linux/setup.sh "${PREFIX}/libexec/zkeys/setup.sh"

SYSTEMD_UNITDIR="${PREFIX}/lib/systemd/system"
install -m 755 -d "${SYSTEMD_UNITDIR}"
install -m 644 target/zkeys-keep-alive.service "${SYSTEMD_UNITDIR}/zkeys-keep-alive.service"
