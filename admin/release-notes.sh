#! /bin/sh
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

readonly PROGNAME="${0##*/}"

err() {
    echo "${PROGNAME}: ${*}" 1>&2
    exit 1
}

# shellcheck disable=SC3043
main() {
    [ "${#}" -eq 0 ] || err "No arguments allowed"

    local version=
    case "${GITHUB_REF-}" in
        refs/tags/zkeys-*)
            version="${GITHUB_REF#refs/tags/zkeys-}"
            ;;

        *)
            err "Not building a release"
            ;;
    esac

    local cargo_version
    cargo_version="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
    [ "${version}" = "${cargo_version}" ] \
        || err "Cargo.toml version does not match tag"

    awk -v version="${version}" '
        $0 == "## Changes in version " version {
            found = 1
            next
        }
        found && /^## Changes in version / {
            exit
        }
        found && /NOT RELEASED YET/ {
            exit 1
        }
        found && started {
            print
            next
        }
        found && $0 != "" {
            started = 1
            print
        }
        END {
            if (!found || !started) {
                exit 1
            }
        }
    ' NEWS.md || err "Cannot find finalized release notes for version ${version}"
}

main "${@}"
