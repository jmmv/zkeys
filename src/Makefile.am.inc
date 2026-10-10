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
	src/Makefile.am.inc \
	$(rust_sources) \
	$(rust_test_sources)

rust_sources = \
	src/config.rs \
	src/get_key.rs \
	src/keep_alive.rs \
	src/lib.rs \
	src/luks_stage_keys.rs \
	src/main.rs \
	src/service.rs \
	src/zfs.rs \
	src/zfs_change_key.rs \
	src/zfs_create.rs \
	src/zfs_load_key.rs \
	src/zpool_create.rs

rust_test_sources = \
	tests/common/mocks.rs \
	tests/common/mod.rs \
	tests/integration_test.rs \
	tests/suites/cli_suite.rs \
	tests/suites/get_key_suite.rs \
	tests/suites/keep_alive_suite.rs \
	tests/suites/list_keys_suite.rs \
	tests/suites/luks_stage_keys_suite.rs \
	tests/suites/mod.rs \
	tests/suites/zfs_change_key_suite.rs \
	tests/suites/zfs_create_suite.rs \
	tests/suites/zfs_list_suite.rs \
	tests/suites/zfs_load_key_suite.rs \
	tests/suites/zpool_create_suite.rs

all-local: $(rust_sources)
