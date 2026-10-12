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

AUTOMAKE_OPTIONS = no-dependencies

AM_DISTCHECK_CONFIGURE_FLAGS = \
	--with-dracut=$(ZKEYS_WITH_DRACUT) \
	--with-freebsd=$(ZKEYS_WITH_FREEBSD) \
	--with-systemd=$(ZKEYS_WITH_SYSTEMD)

dist_doc_DATA = LICENSE NEWS.md NOTICE README.md

BUILT_SOURCES =
CLEANFILES =
DIST_HOOKS =
PHONY_TARGETS =

rust_deps = \
	Cargo.lock \
	Cargo.toml \
	rust-toolchain.toml

EXTRA_DIST = \
	RELEASE.md \
	admin/check-install.sh \
	admin/check-rust-sources.sh \
	admin/merge-mtree.awk \
	admin/release-notes.sh \
	admin/update-version.sh \
	install.mtree \
	prek.toml \
	$(rust_deps) \
	rustfmt.toml \
	setup.sh.in \
	zkeys.toml.tmpl

do_subst = sed \
	-e 's|@BINDIR@|$(sbindir)|g' \
	-e 's|@CONFIG_FILE@|$(sysconfdir)/zkeys.toml|g' \
	-e 's|@PREFIX@|$(prefix)|g' \
	-e 's|@SYSCONFDIR@|$(sysconfdir)|g'

include $(srcdir)/dracut/Makefile.am.inc
include $(srcdir)/freebsd/Makefile.am.inc
include $(srcdir)/man/Makefile.am.inc
include $(srcdir)/src/Makefile.am.inc
include $(srcdir)/systemd/Makefile.am.inc

setup.sh: $(srcdir)/setup.sh.in Makefile
	$(AM_V_GEN)$(do_subst) "$<" >"$@"
	$(AM_V_at)chmod 755 "$@"
BUILT_SOURCES += setup.sh
CLEANFILES += setup.sh

component_install_targets = $(addsuffix -install-data-local,$(ZKEYS_COMPONENTS))
component_uninstall_targets = $(addsuffix -uninstall-local,$(ZKEYS_COMPONENTS))
mtree_manifests = \
	$(srcdir)/install.mtree \
	$(addprefix $(srcdir)/,$(addsuffix /install.mtree,$(ZKEYS_COMPONENTS)))

all.mtree: Makefile $(mtree_manifests) $(srcdir)/admin/merge-mtree.awk
	$(AM_V_GEN)$(AWK) -f "$(srcdir)/admin/merge-mtree.awk" \
		$(mtree_manifests) >"$@"
BUILT_SOURCES += all.mtree
CLEANFILES += all.mtree

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

# Replace Automake's builtin check-news functionality so that we can validate
# the NEWS.md file instead of NEWS.
DIST_HOOKS += check-news
check-news: Makefile $(srcdir)/NEWS.md
	@case "$$(sed 15q "$(srcdir)/NEWS.md")" in \
	*"$(VERSION)"*) : ;; \
	*) \
	    echo "NEWS.md not updated; not releasing" 1>&2; \
	    exit 1 \
	    ;; \
	esac

clean-local:
	$(cargo_env) $(CARGO) clean $(cargo_manifest)

dist-hook: $(DIST_HOOKS)

PHONY_TARGETS += base-install-data-local
base-install-data-local: setup.sh
	$(MKDIR_P) "$(DESTDIR)$(datadir)/examples/zkeys"
	$(INSTALL) -m 600 "$(srcdir)/zkeys.toml.tmpl" \
		"$(DESTDIR)$(datadir)/examples/zkeys/zkeys.toml"
	$(MKDIR_P) "$(DESTDIR)$(libexecdir)/zkeys"
	$(INSTALL_SCRIPT) setup.sh "$(DESTDIR)$(libexecdir)/zkeys/setup.sh"

install-data-local: base-install-data-local $(component_install_targets)

install-exec-local:
	$(MKDIR_P) "$(DESTDIR)$(sbindir)"
	$(INSTALL_PROGRAM) "$(cargo_target_dir)/release/zkeys" \
		"$(DESTDIR)$(sbindir)/zkeys"

PHONY_TARGETS += base-uninstall-local
base-uninstall-local:
	rm -f "$(DESTDIR)$(datadir)/examples/zkeys/zkeys.toml"
	rm -f "$(DESTDIR)$(libexecdir)/zkeys/setup.sh"

uninstall-local: $(component_uninstall_targets) base-uninstall-local
	rm -f "$(DESTDIR)$(sbindir)/zkeys"

installcheck-local: all.mtree
	"$(abs_top_srcdir)/admin/check-install.sh" \
		"$(MTREE)" \
		"$(abs_top_builddir)/all.mtree" \
		"$(DESTDIR)" \
		"$(prefix)" \
		"$(MAKE)" $(AM_MAKEFLAGS)

.PHONY: $(PHONY_TARGETS)
