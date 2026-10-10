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

readonly mtree="${1}"; shift
readonly manifest="${1}"; shift
readonly destdir="${1}"; shift
readonly prefix="${1}"; shift
readonly make="${1}"; shift
# "${@}" contains extra make flags.

if [ -z "${mtree}" ]; then
    echo "ERROR: mtree is required for installcheck; run ./configure again" 1>&2
    exit 1
fi

"${mtree}" -f "${manifest}" -p "${destdir}${prefix}" || exit $?

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/zkeys-installcheck.XXXXXX")
cleanup() {
    rm -rf "${work_dir}"
}
trap cleanup EXIT HUP INT TERM QUIT

"${make}" "${@}" DESTDIR="${work_dir}" install
"${mtree}" -f "${manifest}" -p "${work_dir}${prefix}"

"${make}" "${@}" DESTDIR="${work_dir}" uninstall
leftover_files="$(find "${work_dir}" \! -type d)"
if [ -n "${leftover_files}" ]; then
    echo "ERROR: make uninstall left these behind: ${leftover_files}" 1>&2
    exit 1
fi
