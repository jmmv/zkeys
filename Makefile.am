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

AUTOMAKE_OPTIONS = no-dependencies

AM_DISTCHECK_CONFIGURE_FLAGS = --with-platform=$(ZKEYS_PLATFORM)

dist_doc_DATA = LICENSE NOTICE README.md

BUILT_SOURCES =
CLEANFILES =
PHONY_TARGETS =

rust_deps = \
	Cargo.lock \
	Cargo.toml \
	rust-toolchain.toml

EXTRA_DIST = \
	admin/check-install.sh \
	admin/check-rust-sources.sh \
	prek.toml \
	$(rust_deps) \
	rustfmt.toml \
	zkeys.toml.tmpl

do_subst = sed \
	-e 's|@BINDIR@|$(sbindir)|g' \
	-e 's|@CONFIG_FILE@|$(sysconfdir)/zkeys.toml|g' \
	-e 's|@PREFIX@|$(prefix)|g' \
	-e 's|@SYSCONFDIR@|$(sysconfdir)|g'

include $(srcdir)/freebsd/Makefile.am.inc
include $(srcdir)/linux/Makefile.am.inc
include $(srcdir)/man/Makefile.am.inc
include $(srcdir)/src/Makefile.am.inc

cargo_target_dir = $(abs_top_builddir)/target
cargo_env = \
	CARGO_TARGET_DIR="$(cargo_target_dir)" \
	SYSCONFDIR="$(sysconfdir)"
cargo_manifest = --manifest-path="$(abs_top_srcdir)/Cargo.toml"

PHONY_TARGETS += lint
lint:
	cd "$(abs_top_srcdir)" && \
		CARGO_TARGET_DIR="$(cargo_target_dir)" prek run --all-files

all-local: $(BUILT_SOURCES) $(cargo_target_dir)/release/zkeys

PHONY_TARGETS += release
release: $(cargo_target_dir)/release/zkeys

$(cargo_target_dir)/release/zkeys: Makefile $(rust_deps) $(rust_sources)
	$(cargo_env) $(CARGO) build $(cargo_manifest) --locked --release

PHONY_TARGETS += debug
debug: $(cargo_target_dir)/debug/zkeys

$(cargo_target_dir)/debug/zkeys: Makefile $(rust_deps) $(rust_sources)
	$(cargo_env) $(CARGO) build $(cargo_manifest) --locked

check-local: $(rust_deps) $(rust_sources) $(rust_test_sources)
	$(cargo_env) $(CARGO) test $(cargo_manifest) --locked

clean-local:
	$(cargo_env) $(CARGO) clean $(cargo_manifest)

install-data-local: $(ZKEYS_PLATFORM)-install-data-local

install-exec-local:
	$(MKDIR_P) "$(DESTDIR)$(sbindir)"
	$(INSTALL_PROGRAM) "$(cargo_target_dir)/release/zkeys" \
		"$(DESTDIR)$(sbindir)/zkeys"

uninstall-local: $(ZKEYS_PLATFORM)-uninstall-local
	rm -f "$(DESTDIR)$(sbindir)/zkeys"

installcheck-local:
	"$(abs_top_srcdir)/admin/check-install.sh" \
		"$(MTREE)" \
		"$(abs_top_srcdir)/$(ZKEYS_PLATFORM)/install.mtree" \
		"$(DESTDIR)" \
		"$(prefix)" \
		"$(MAKE)" $(AM_MAKEFLAGS)

.PHONY: $(PHONY_TARGETS)
