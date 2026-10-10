#!/bin/sh
# shellcheck disable=SC1091,SC2034
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

# PROVIDE: zkeys_keep_alive
# REQUIRE: NETWORKING

. /etc/rc.subr

name="zkeys_keep_alive"
desc="The ZKeys keep-alive service"
command="daemon"
rcvar="zkeys_keep_alive_enable"
pidfile="/var/run/zkeys-keep-alive.pid"
start_cmd="zkeys_keep_alive_start"
required_files="@SYSCONFDIR@/zkeys.toml @BINDIR@/zkeys"

zkeys_keep_alive_start()
{
    if [ ! -f /var/log/zkeys-keep-alive.log ]; then
        touch /var/log/zkeys-keep-alive.log
        chmod 600 /var/log/zkeys-keep-alive.log
        chown root:wheel /var/log/zkeys-keep-alive.log
    fi

    echo "Starting zkeys-keep-alive."
    daemon \
        -P "${pidfile}" \
        -o /var/log/zkeys-keep-alive.log \
        -H \
        -t "zkeys-keep-alive" \
        @BINDIR@/zkeys \
        keep-alive \
        --config-file @SYSCONFDIR@/zkeys.toml
}

load_rc_config $name
run_rc_command "$1"
