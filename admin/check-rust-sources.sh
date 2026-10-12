#!/bin/sh
# ZKeys
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

cd "$(dirname "$0")/.."

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/zkeys-rust-sources.XXXXXX")
cleanup() {
    rm -rf "${work_dir}"
}
trap cleanup EXIT HUP INT TERM QUIT

# shellcheck disable=SC3043
check_sources() {
    local directory="${1}"; shift
    local variable="${1}"; shift

    find "${directory}" -type f -name '*.rs' -print \
        | LC_ALL=C sort >"${work_dir}/actual"
    awk -v variable="${variable}" '
        $1 == variable && $2 == "=" {
            in_sources = 1
            next
        }
        in_sources {
            line = $0
            sub(/^[[:space:]]*/, "", line)
            continued = sub(/[[:space:]]*\\$/, "", line)
            print line
            if (!continued) exit
        }
    ' src/Makefile.am.inc | LC_ALL=C sort >"${work_dir}/listed"

    if ! diff -u "${work_dir}/actual" "${work_dir}/listed"; then
        echo "ERROR: ${variable} in src/Makefile.am.inc is out of date" 1>&2
        exit 1
    fi
}

check_sources src rust_sources
check_sources tests rust_test_sources
