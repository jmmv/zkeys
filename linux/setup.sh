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

readonly PROGNAME="${0##*/}"

readonly PREFIX="@PREFIX@"
readonly SYSCONFDIR="@SYSCONFDIR@"
readonly ROOT="${DESTDIR:-}"
readonly DRACUT_MODULE="${PREFIX}/lib/dracut/modules.d/50zkeys"
readonly DRACUT_LINK="/usr/lib/dracut/modules.d/50zkeys"
readonly SYSTEMD_UNIT="${PREFIX}/lib/systemd/system/zkeys-keep-alive.service"

err() {
    echo "${PROGNAME}: E: ${*}" 1>&2
    exit 1
}

log() {
    echo "${*}" 1>&2
    "${@}"
}

setup_config() {
    if [ -e "${ROOT}${SYSCONFDIR}/zkeys.toml" ] \
        || [ -L "${ROOT}${SYSCONFDIR}/zkeys.toml" ]
    then
        return
    fi

    log install -m 755 -d "${ROOT}${SYSCONFDIR}"
    log install -m 600 \
        "${ROOT}${PREFIX}/share/examples/zkeys/zkeys.toml" \
        "${ROOT}${SYSCONFDIR}/zkeys.toml"
}

setup_dracut() {
    [ -d "${ROOT}${DRACUT_MODULE}" ] \
        || err "Cannot find the ZKeys dracut module at ${DRACUT_MODULE}"
    [ "${DRACUT_MODULE}" != "${DRACUT_LINK}" ] || return

    log install -m 755 -d "${ROOT}$(dirname "${DRACUT_LINK}")"

    if [ -L "${ROOT}${DRACUT_LINK}" ]; then
        [ "$(readlink "${ROOT}${DRACUT_LINK}")" != "${DRACUT_MODULE}" ] || return
        err "Refusing to replace ${DRACUT_LINK}: it points elsewhere"
    fi
    [ ! -e "${ROOT}${DRACUT_LINK}" ] \
        || err "Refusing to replace ${DRACUT_LINK}: it already exists"
    log ln -s "${DRACUT_MODULE}" "${ROOT}${DRACUT_LINK}"
}

setup_systemd() {
    [ -f "${ROOT}${SYSTEMD_UNIT}" ] \
        || err "Cannot find the ZKeys systemd unit at ${SYSTEMD_UNIT}"

    if [ -n "${ROOT}" ]; then
        systemctl --root="${ROOT}" enable "${SYSTEMD_UNIT}"
    else
        systemctl enable "${SYSTEMD_UNIT}"
        systemctl daemon-reload
    fi
}

main() {
    if [ -z "${ROOT}" ] && [ "$(id -u)" -ne 0 ]; then
        err "Must run as root"
    fi

    setup_config
    setup_dracut
    setup_systemd
}

main "${@}"
