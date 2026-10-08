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

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/sbin
SYSCONFDIR ?= /etc

DRACUT_MODULE := target/linux/dracut/50zkeys/module-setup.sh
all: $(DRACUT_MODULE)
$(DRACUT_MODULE): linux/dracut/50zkeys/module-setup.sh.in target/stamp.paths
	@mkdir -p "$(dir $@)"
	$(SUBST) "$<" >"$@"
	chmod 755 "$@"

SETUP := target/linux/setup.sh
all: $(SETUP)
$(SETUP): linux/setup.sh target/stamp.paths
	@mkdir -p "$(dir $@)"
	$(SUBST) "$<" >"$@"
	chmod 755 "$@"

KEEP_ALIVE_SERVICE := target/zkeys-keep-alive.service
all: $(KEEP_ALIVE_SERVICE)
$(KEEP_ALIVE_SERVICE): linux/systemd/zkeys-keep-alive.service.in target/stamp.paths
	@mkdir -p "$(dir $@)"
	$(SUBST) "$<" >"$@"

test: test-dracut
test-dracut: target/linux/dracut/50zkeys/module-setup.sh
	./linux/dracut_test.sh
