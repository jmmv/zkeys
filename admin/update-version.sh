#! /bin/sh
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

err() {
    echo "${PROGNAME}: ${*}" 1>&2
    exit 1
}

# shellcheck disable=SC3043
replace() {
    local path="${1}"; shift

    sed "${@}" "${path}" >"${path}.new"
    mv "${path}.new" "${path}"
}

# shellcheck disable=SC3043
add_devel_news_section() {
    local path="${1}"; shift
    local version="${1}"; shift

    POSIXLY_CORRECT=1 awk -v version="${version}" '
        /^## Changes in / && !done {
            print "## Changes in version " version
            print ""
            print "**NOT RELEASED YET; STILL UNDER DEVELOPMENT.**"
            print ""
            print "*   No changes recorded."
            print ""
            done = 1
        }
        { print }
        END {
            if (!done) {
                exit 1
            }
        }
    ' "${path}" >"${path}.new" || err "Cannot find release notes in ${path}"
    mv "${path}.new" "${path}"
}

# shellcheck disable=SC3043
fix_cargo_lock() {
    local path="${1}"; shift
    local version="${1}"; shift

    POSIXLY_CORRECT=1 awk -v version="${version}" '
        /^\[/ { found = 0 }
        /^name = "zkeys"$/ { found = 1 }
        /^version = / && found {
            sub(/[0-9]+\.[0-9]+\.[0-9]+/, version)
            found = 0
        }
        { print }
    ' "${path}" >"${path}.new"
    mv "${path}.new" "${path}"
}

# shellcheck disable=SC3043
release_news() {
    local path="${1}"; shift
    local version="${1}"; shift
    local date="${1}"; shift

    POSIXLY_CORRECT=1 awk -v date="${date}" -v version="${version}" '
        /^## Changes in version / && !heading {
            print "## Changes in version " version
            heading = 1
            next
        }
        /NOT RELEASED/ && heading && !banner {
            print "**Released on " date ".**"
            banner = 1
            next
        }
        { print }
        END {
            if (!heading || !banner) {
                exit 1
            }
        }
    ' "${path}" >"${path}.new" || err "Cannot update release notes in ${path}"
    mv "${path}.new" "${path}"
}

# shellcheck disable=SC3043
main() {
    [ "${#}" -eq 1 ] || err "Must specify the new version number"
    local version="${1}"; shift

    case "${version}" in
        *.99)
            add_devel_news_section NEWS.md "${version}"
            ;;

        *)
            local date
            date="$(date +%Y-%m-%d)"

            replace README.md -E "/latest version/s/[0-9]+\.[0-9]+\.[0-9]+/${version}/g"
            replace README.md -E "/releases\/tag/s/[0-9]+\.[0-9]+\.[0-9]+/${version}/g"
            replace README.md -E "/released on/s/[0-9]{4}-[0-9]{2}-[0-9]{2}/${date}/g"

            release_news NEWS.md "${version}" "${date}"
            ;;
    esac

    replace Cargo.toml -E "/^version = /s/[0-9]+\.[0-9]+\.[0-9]+/${version}/"
    fix_cargo_lock Cargo.lock "${version}"
}

main "${@}"
