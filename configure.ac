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

AC_PREREQ([2.69])
AC_INIT(
    [ZKeys],
    m4_esyscmd_s([awk -F'"' '/^version = / { print $2; exit }' Cargo.toml]),
    [https://github.com/jmmv/zkeys/issues],
    [zkeys],
    [https://zkeys.jmmv.dev/])

AC_CANONICAL_HOST
AC_CONFIG_AUX_DIR([admin])
AC_CONFIG_FILES([Makefile])
AC_CONFIG_SRCDIR([Cargo.toml])

AM_INIT_AUTOMAKE([1.16 foreign no-dependencies subdir-objects])

AC_PROG_AWK
AC_PATH_PROG([CARGO], [cargo])
AS_IF([test -z "$CARGO"], [AC_MSG_ERROR([cargo is required])])
AC_PATH_PROG([RUSTC], [rustc])
AS_IF([test -z "$RUSTC"], [AC_MSG_ERROR([rustc is required])])
AC_PATH_PROGS([MTREE], [mtree bsdtar])
AS_IF([test -z "$MTREE"], [
    AC_MSG_WARN([neither mtree nor bsdtar was found; make installcheck and make distcheck will fail])
])

AC_ARG_WITH(
    [dracut],
    [AS_HELP_STRING(
        [--with-dracut=auto|yes|no],
        [install the dracut integration (default: auto)])],
    [zkeys_with_dracut="$withval"],
    [zkeys_with_dracut=auto])
AS_CASE(
    [$zkeys_with_dracut],
    [auto], [
        AC_CHECK_PROG([zkeys_dracut_detected], [dracut], [yes], [no])
        zkeys_with_dracut="$zkeys_dracut_detected"
    ],
    [yes|no], [],
    [AC_MSG_ERROR([invalid value for --with-dracut: $zkeys_with_dracut])])

AC_ARG_WITH(
    [freebsd],
    [AS_HELP_STRING(
        [--with-freebsd=auto|yes|no],
        [install the FreeBSD integration (default: auto)])],
    [zkeys_with_freebsd="$withval"],
    [zkeys_with_freebsd=auto])
AS_IF([test "x$zkeys_with_freebsd" = xauto], [
    AS_CASE(
        [$host_os],
        [freebsd*], [zkeys_with_freebsd=yes],
        [zkeys_with_freebsd=no])
])
AS_CASE(
    [$zkeys_with_freebsd],
    [yes|no], [],
    [AC_MSG_ERROR([invalid value for --with-freebsd: $zkeys_with_freebsd])])

AC_ARG_WITH(
    [systemd],
    [AS_HELP_STRING(
        [--with-systemd=auto|yes|no],
        [install the systemd integration (default: auto)])],
    [zkeys_with_systemd="$withval"],
    [zkeys_with_systemd=auto])
AS_CASE(
    [$zkeys_with_systemd],
    [auto], [
        AC_CHECK_PROG([zkeys_systemd_detected], [systemctl], [yes], [no])
        zkeys_with_systemd="$zkeys_systemd_detected"
    ],
    [yes|no], [],
    [AC_MSG_ERROR([invalid value for --with-systemd: $zkeys_with_systemd])])

zkeys_components=
AS_IF([test "x$zkeys_with_dracut" = xyes], [
    zkeys_components="$zkeys_components dracut"
])
AS_IF([test "x$zkeys_with_freebsd" = xyes], [
    zkeys_components="$zkeys_components freebsd"
])
AS_IF([test "x$zkeys_with_systemd" = xyes], [
    zkeys_components="$zkeys_components systemd"
])

AC_SUBST([ZKEYS_COMPONENTS], [$zkeys_components])
AC_SUBST([ZKEYS_WITH_DRACUT], [$zkeys_with_dracut])
AC_SUBST([ZKEYS_WITH_FREEBSD], [$zkeys_with_freebsd])
AC_SUBST([ZKEYS_WITH_SYSTEMD], [$zkeys_with_systemd])

zkeys_component_summary=${zkeys_components# }
AS_IF([test -z "$zkeys_component_summary"], [zkeys_component_summary=none])
AC_MSG_NOTICE([selected components: $zkeys_component_summary])

AC_OUTPUT
