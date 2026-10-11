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

AC_PREREQ([2.69])
AC_INIT(
    [zkeys],
    m4_esyscmd_s([awk -F'"' '/^version = / { print $2; exit }' Cargo.toml]),
    [https://github.com/jmmv/zkeys/issues],
    [zkeys],
    [https://zkeys.jmmv.dev/])

AC_CANONICAL_HOST
AC_CONFIG_AUX_DIR([admin])
AC_CONFIG_FILES([Makefile])
AC_CONFIG_SRCDIR([Cargo.toml])

AM_INIT_AUTOMAKE([1.16 foreign no-dependencies subdir-objects])

AC_PATH_PROG([CARGO], [cargo])
AS_IF([test -z "$CARGO"], [AC_MSG_ERROR([cargo is required])])
AC_PATH_PROG([RUSTC], [rustc])
AS_IF([test -z "$RUSTC"], [AC_MSG_ERROR([rustc is required])])
AC_PATH_PROGS([MTREE], [mtree bsdtar])
AS_IF([test -z "$MTREE"], [
    AC_MSG_WARN([neither mtree nor bsdtar was found; make installcheck and make distcheck will fail])
])

AC_ARG_WITH(
    [platform],
    [AS_HELP_STRING(
        [--with-platform=auto|linux|freebsd],
        [payload to install (default: auto)])],
    [zkeys_platform="$withval"],
    [zkeys_platform=auto])

AS_IF([test "x$zkeys_platform" = xauto], [
    AS_CASE(
        [$host_os],
        [linux*], [zkeys_platform=linux],
        [freebsd*], [zkeys_platform=freebsd],
        [AC_MSG_ERROR([unsupported host operating system $host_os; use --with-platform])])
])
AS_CASE(
    [$zkeys_platform],
    [linux|freebsd], [],
    [AC_MSG_ERROR([invalid platform $zkeys_platform; expected auto, linux, or freebsd])])

AS_IF([test "x$zkeys_platform" = xlinux && test "x$sysconfdir" = 'x${prefix}/etc'], [
    sysconfdir=/etc
])

AC_SUBST([ZKEYS_PLATFORM], [$zkeys_platform])

AC_OUTPUT
