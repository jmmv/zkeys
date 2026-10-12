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
	dracut/Makefile.am.inc \
	dracut/50zkeys/module-setup.sh.in \
	dracut/50zkeys/zkeys-cryptsetup.conf \
	dracut/50zkeys/zkeys-initrd.service \
	dracut/install.mtree \
	dracut/README.md

dracut_built_sources = dracut/50zkeys/module-setup.sh

dracut/50zkeys/module-setup.sh: \
		$(srcdir)/dracut/50zkeys/module-setup.sh.in Makefile
	$(AM_V_GEN)$(MKDIR_P) "$(dir $@)"
	$(AM_V_at)$(do_subst) "$<" >"$@"
	$(AM_V_at)chmod 755 "$@"

BUILT_SOURCES += $(dracut_built_sources)
CLEANFILES += $(dracut_built_sources)

PHONY_TARGETS += dracut-install-data-local
dracut-install-data-local: $(dracut_built_sources)
	$(MKDIR_P) "$(DESTDIR)$(docdir)"
	$(INSTALL_DATA) "$(srcdir)/dracut/README.md" \
		"$(DESTDIR)$(docdir)/README.dracut.md"
	$(MKDIR_P) "$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys"
	$(INSTALL_SCRIPT) dracut/50zkeys/module-setup.sh \
		"$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/module-setup.sh"
	$(INSTALL_DATA) "$(srcdir)/dracut/50zkeys/zkeys-cryptsetup.conf" \
		"$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/zkeys-cryptsetup.conf"
	$(INSTALL_DATA) "$(srcdir)/dracut/50zkeys/zkeys-initrd.service" \
		"$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/zkeys-initrd.service"

PHONY_TARGETS += dracut-uninstall-local
dracut-uninstall-local:
	rm -f "$(DESTDIR)$(docdir)/README.dracut.md"
	rm -f "$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/module-setup.sh"
	rm -f "$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/zkeys-cryptsetup.conf"
	rm -f "$(DESTDIR)$(prefix)/lib/dracut/modules.d/50zkeys/zkeys-initrd.service"
