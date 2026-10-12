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

EXTRA_DIST += \
	freebsd/Makefile.am.inc \
	freebsd/install.mtree \
	freebsd/keep-alive.rc.d.sh.in \
	freebsd/mount-all.rc.d.sh.in \
	freebsd/newsyslog.conf \
	freebsd/README.md

freebsd_built_sources = \
	freebsd/keep-alive.rc.d.sh \
	freebsd/mount-all.rc.d.sh

freebsd/keep-alive.rc.d.sh: $(srcdir)/freebsd/keep-alive.rc.d.sh.in Makefile
	$(AM_V_GEN)$(MKDIR_P) "$(dir $@)"
	$(AM_V_at)$(do_subst) "$<" >"$@"
	$(AM_V_at)chmod 755 "$@"

freebsd/mount-all.rc.d.sh: $(srcdir)/freebsd/mount-all.rc.d.sh.in Makefile
	$(AM_V_GEN)$(MKDIR_P) "$(dir $@)"
	$(AM_V_at)$(do_subst) "$<" >"$@"
	$(AM_V_at)chmod 755 "$@"

BUILT_SOURCES += $(freebsd_built_sources)
CLEANFILES += $(freebsd_built_sources)

PHONY_TARGETS += freebsd-install-data-local
freebsd-install-data-local: $(freebsd_built_sources)
	$(MKDIR_P) "$(DESTDIR)$(docdir)"
	$(INSTALL_DATA) "$(srcdir)/freebsd/README.md" \
		"$(DESTDIR)$(docdir)/README.freebsd.md"
	$(MKDIR_P) "$(DESTDIR)$(sysconfdir)"
	$(MKDIR_P) "$(DESTDIR)$(sysconfdir)/newsyslog.conf.d"
	$(INSTALL_DATA) "$(srcdir)/freebsd/newsyslog.conf" \
		"$(DESTDIR)$(sysconfdir)/newsyslog.conf.d/zkeys.conf"
	$(MKDIR_P) "$(DESTDIR)$(sysconfdir)/rc.d"
	$(INSTALL_SCRIPT) freebsd/keep-alive.rc.d.sh \
		"$(DESTDIR)$(sysconfdir)/rc.d/zkeys-keep-alive"
	$(INSTALL_SCRIPT) freebsd/mount-all.rc.d.sh \
		"$(DESTDIR)$(sysconfdir)/rc.d/zkeys-mount-all"

PHONY_TARGETS += freebsd-uninstall-local
freebsd-uninstall-local:
	rm -f "$(DESTDIR)$(sysconfdir)/newsyslog.conf.d/zkeys.conf"
	rm -f "$(DESTDIR)$(sysconfdir)/rc.d/zkeys-keep-alive"
	rm -f "$(DESTDIR)$(sysconfdir)/rc.d/zkeys-mount-all"
	rm -f "$(DESTDIR)$(docdir)/README.freebsd.md"
