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

EXTRA_DIST += \
	systemd/Makefile.am.inc \
	systemd/install.mtree \
	systemd/README.md \
	systemd/zkeys-keep-alive.service.in

systemd_built_sources = systemd/zkeys-keep-alive.service

systemd/zkeys-keep-alive.service: \
		$(srcdir)/systemd/zkeys-keep-alive.service.in Makefile
	$(AM_V_GEN)$(MKDIR_P) "$(dir $@)"
	$(AM_V_at)$(do_subst) "$<" >"$@"

BUILT_SOURCES += $(systemd_built_sources)
CLEANFILES += $(systemd_built_sources)

PHONY_TARGETS += systemd-install-data-local
systemd-install-data-local: $(systemd_built_sources)
	$(MKDIR_P) "$(DESTDIR)$(docdir)"
	$(INSTALL_DATA) "$(srcdir)/systemd/README.md" \
		"$(DESTDIR)$(docdir)/README.systemd.md"
	$(MKDIR_P) "$(DESTDIR)$(prefix)/lib/systemd/system"
	$(INSTALL_DATA) systemd/zkeys-keep-alive.service \
		"$(DESTDIR)$(prefix)/lib/systemd/system/zkeys-keep-alive.service"

PHONY_TARGETS += systemd-uninstall-local
systemd-uninstall-local:
	rm -f "$(DESTDIR)$(docdir)/README.systemd.md"
	rm -f "$(DESTDIR)$(prefix)/lib/systemd/system/zkeys-keep-alive.service"
