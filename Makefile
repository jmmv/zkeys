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

CARGO = PREFIX="$(PREFIX)" cargo
MANPAGES := target/zkeys.8 man/zkeys.toml.5
SRCS := Cargo.toml Cargo.lock rust-toolchain.toml \
    $(shell find "src" "tests" \( -name "*.rs" -o -name "Cargo.*" \) -and -not -path "./target/*")

.PHONY: all
all: release manpages

.PHONY: target/stamp.prefix.new
target/stamp.prefix.new:
	@mkdir -p "$(dir $@)"
	@printf '%s\n' '$(PREFIX)' >"$@"

target/stamp.prefix: target/stamp.prefix.new
	@mkdir -p "$(dir $@)"
	@cmp -s "$<" "$@" || cp "$<" "$@"

.PHONY: debug
debug: target/debug/zkeys

target/debug/zkeys: $(SRCS) target/stamp.prefix
	$(CARGO) build
	@touch "$@"

.PHONY: release
release: target/release/zkeys

target/release/zkeys: $(SRCS) target/stamp.prefix
	$(CARGO) build --release
	@touch "$@"

manpages: $(MANPAGES)

target/zkeys.8: man/zkeys.8.in target/stamp.prefix
	@mkdir -p "$(dir $@)"
	sed -e 's|@PREFIX@|$(PREFIX)|g' "$<" >"$@"

.PHONY: install
install: all
	sh "./install.sh" "$(PREFIX)"

.PHONY: test
test:
	$(CARGO) test

.PHONY: lint
lint:
	prek run --all-files

.PHONY: clean
clean:
	$(CARGO) clean
