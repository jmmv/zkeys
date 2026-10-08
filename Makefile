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

ifdef ZKEYS_TEST_INSTALL_FLAVOR
FLAVOR = $(ZKEYS_TEST_INSTALL_FLAVOR)
else
FLAVOR = $(shell uname -s | tr A-Z a-z)
endif

.PHONY: all
all:

.PHONY: test
test:

SUBST = sed \
    -e 's|@BINDIR@|$(BINDIR)|g' \
    -e 's|@PREFIX@|$(PREFIX)|g' \
    -e 's|@SYSCONFDIR@|$(SYSCONFDIR)|g'

include $(FLAVOR)/Makefile.inc

CARGO = BINDIR="$(BINDIR)" PREFIX="$(PREFIX)" SYSCONFDIR="$(SYSCONFDIR)" cargo
MANPAGES := target/zkeys.8 man/zkeys.toml.5
SRCS := Cargo.toml Cargo.lock rust-toolchain.toml \
    $(shell find "src" "tests" \( -name "*.rs" -o -name "Cargo.*" \) -and -not -path "./target/*")

all: release manpages

.PHONY: target/stamp.paths.new
target/stamp.paths.new:
	@mkdir -p "$(dir $@)"
	@printf '%s\n%s\n%s\n' '$(BINDIR)' '$(PREFIX)' '$(SYSCONFDIR)' >"$@"

target/stamp.paths: target/stamp.paths.new
	@mkdir -p "$(dir $@)"
	@cmp -s "$<" "$@" || cp "$<" "$@"

.PHONY: debug
debug: target/debug/zkeys

target/debug/zkeys: $(SRCS) target/stamp.paths
	$(CARGO) build
	@touch "$@"

.PHONY: release
release: target/release/zkeys

target/release/zkeys: $(SRCS) target/stamp.paths
	$(CARGO) build --release
	@touch "$@"

manpages: $(MANPAGES)

target/zkeys.8: man/zkeys.8.in target/stamp.paths
	@mkdir -p "$(dir $@)"
	$(SUBST) "$<" >"$@"

.PHONY: install
install: all
	env BINDIR="$(BINDIR)" \
	    DESTDIR="$(DESTDIR)" \
	    PREFIX="$(PREFIX)" \
	    SYSCONFDIR="$(SYSCONFDIR)" \
	    ./install.sh

test: test-cargo
.PHONY: test-cargo
test-cargo:
	$(CARGO) test

.PHONY: lint
lint:
	prek run --all-files

.PHONY: clean
clean:
	$(CARGO) clean
