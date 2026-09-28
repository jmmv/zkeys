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
pidfile="/var/run/${name}.pid"
start_cmd="zkeys_keep_alive_start"
required_files="/usr/local/etc/zkeys.toml /usr/local/sbin/zkeys"

zkeys_keep_alive_start()
{
    if [ ! -f /var/log/${name}.log ]; then
        touch /var/log/${name}.log
        chmod 600 /var/log/${name}.log
        chown root:wheel /var/log/${name}.log
    fi

    echo "Starting ${name}."
    daemon -P "${pidfile}" -o /var/log/${name}.log -H -t "${name}" \
        /usr/local/sbin/zkeys keep-alive --config-file /usr/local/etc/zkeys.toml
}

load_rc_config $name
run_rc_command "$1"
