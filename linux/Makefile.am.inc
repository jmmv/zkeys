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
	linux/Makefile.am.inc \
	linux/dracut/50zkeys/module-setup.sh.in \
	linux/dracut/50zkeys/zkeys-cryptsetup.conf \
	linux/dracut/50zkeys/zkeys-initrd.service \
	linux/install.mtree \
	linux/README.md \
	linux/setup.sh.in \
	linux/systemd/zkeys-keep-alive.service.in

linux_built_sources = \
	linux/dracut/50zkeys/module-setup.sh \
	linux/setup.sh \
	linux/systemd/zkeys-keep-alive.service

linux/dracut/50zkeys/module-setup.sh: \
		$(srcdir)/linux/dracut/50zkeys/module-setup.sh.in Makefile
	$(AM_V_GEN)$(MKDIR_P) "$(dir $@)"
	$(AM_V_at)$(do_subst) "$<" >"$@"
	$(AM_V_at)chmod 755 "$@"

linux/setup.sh: $(srcdir)/linux/setup.sh.in Makefile
	$(AM_V_GEN)$(MKDIR_P) "$(dir $@)"
	$(AM_V_at)$(do_subst) "$<" >"$@"
	$(AM_V_at)chmod 755 "$@"

linux/systemd/zkeys-keep-alive.service: \
		$(srcdir)/linux/systemd/zkeys-keep-alive.service.in Makefile
	$(AM_V_GEN)$(MKDIR_P) "$(dir $@)"
	$(AM_V_at)$(do_subst) "$<" >"$@"

BUILT_SOURCES += $(linux_built_sources)
CLEANFILES += $(linux_built_sources)

PHONY_TARGETS += linux-install-data-local
linux-install-data-local: $(linux_built_sources)
	$(MKDIR_P) "$(DESTDIR)$(docdir)"
	$(INSTALL_DATA) "$(srcdir)/linux/README.md" \
		"$(DESTDIR)$(docdir)/README.linux.md"
	$(MKDIR_P) "$(DESTDIR)$(datadir)/examples/zkeys"
	$(INSTALL) -m 600 "$(srcdir)/zkeys.toml.tmpl" \
		"$(DESTDIR)$(datadir)/examples/zkeys/zkeys.toml"
	$(MKDIR_P) "$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys"
	$(INSTALL_SCRIPT) linux/dracut/50zkeys/module-setup.sh \
		"$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/module-setup.sh"
	$(INSTALL_DATA) "$(srcdir)/linux/dracut/50zkeys/zkeys-cryptsetup.conf" \
		"$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/zkeys-cryptsetup.conf"
	$(INSTALL_DATA) "$(srcdir)/linux/dracut/50zkeys/zkeys-initrd.service" \
		"$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/zkeys-initrd.service"
	$(MKDIR_P) "$(DESTDIR)$(libexecdir)/zkeys"
	$(INSTALL_SCRIPT) linux/setup.sh "$(DESTDIR)$(libexecdir)/zkeys/setup.sh"
	$(MKDIR_P) "$(DESTDIR)$(prefix)/lib/systemd/system"
	$(INSTALL_DATA) linux/systemd/zkeys-keep-alive.service \
		"$(DESTDIR)$(prefix)/lib/systemd/system/zkeys-keep-alive.service"

PHONY_TARGETS += linux-uninstall-local
linux-uninstall-local:
	rm -f "$(DESTDIR)$(datadir)/examples/zkeys/zkeys.toml"
	rm -f "$(DESTDIR)$(docdir)/README.linux.md"
	rm -f "$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/module-setup.sh"
	rm -f "$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/zkeys-cryptsetup.conf"
	rm -f "$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/zkeys-initrd.service"
	rm -f "$(DESTDIR)$(libexecdir)/zkeys/setup.sh"
	rm -f "$(DESTDIR)$(prefix)/lib/systemd/system/zkeys-keep-alive.service"
