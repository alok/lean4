#!/usr/bin/env bash
source ../common.sh

./clean.sh

# Script driver
echo "# TEST: Script driver"
test_out "script: []" -f script.lean test
test_out "script: []" -f script.lean lint
test_out "script: []" -f script.lean bench
test_out "hello" -f script.lean test -- hello
test_out "hello" -f script.lean lint -- hello
test_out "hello" -f script.lean bench -- hello

# Executable driver
echo "# TEST: Executable driver"
test_out "exe: []" -f exe.lean test
test_out "exe: []" -f exe.toml test
test_out "exe: []" -f exe.lean lint
test_out "exe: []" -f exe.toml lint
test_out "exe: []" -f exe.lean bench
test_out "exe: []" -f exe.toml bench
test_out "hello" -f exe.lean test -- hello
test_out "hello" -f exe.toml test -- hello
test_out "hello" -f exe.lean lint -- hello
test_out "hello" -f exe.toml lint -- hello
test_out "hello" -f exe.lean bench -- hello
test_out "hello" -f exe.toml bench -- hello

# Library test driver
echo "# TEST: Library test driver"
rm -f .lake/build/lib/lean/Test.olean
test_out_pat "(Built|Replayed|Fetched) Test" -f lib.lean test
rm -f .lake/build/lib/lean/Test.olean
test_out_pat "(Built|Replayed|Fetched) Test" -f lib.toml test
test_err "arguments cannot be passed" -f lib.lean test -- hello
test_err "arguments cannot be passed" -f lib.toml test -- hello
rm -f .lake/build/lib/lean/Test.olean
test_out_pat "(Built|Replayed|Fetched) Test" -f lib.lean bench
rm -f .lake/build/lib/lean/Test.olean
test_out_pat "(Built|Replayed|Fetched) Test" -f lib.toml bench
test_err "arguments cannot be passed" -f lib.lean bench -- hello
test_err "arguments cannot be passed" -f lib.toml bench -- hello

# Upstream driver
echo "# TEST: Driver from dependency"
rm -f lake-manifest.json
test_out "dep: []" -f dep.lean test
test_out "dep: []" -f dep.toml test
test_out "dep: []" -f dep.lean lint
test_out "dep: []" -f dep.toml lint
test_out "dep: []" -f dep.lean bench
test_out "dep: []" -f dep.toml bench
test_out "hello" -f dep.lean test -- hello
test_out "hello" -f dep.toml test -- hello
test_out "hello" -f dep.lean lint -- hello
test_out "hello" -f dep.toml lint -- hello
test_out "hello" -f dep.lean bench -- hello
test_out "hello" -f dep.toml bench -- hello

# Test runner
echo "# TEST: Test runner"
test_out " @[test_runner] has been deprecated" -f runner.lean test
test_run -f runner.toml test
test_run -f runner.lean check-test
test_run -f runner.toml check-test

# Driver validation
echo "# TEST: Driver validation"
test_err "only one" -f two.lean test
test_err "no test driver" -f none.lean test
test_err "no test driver" -f none.toml test
test_err "invalid test driver: unknown" -f unknown.lean test
test_err "invalid test driver: unknown" -f unknown.toml test
test_err "unknown test driver package" -f dep-unknown.lean test
test_err "unknown test driver package" -f dep-unknown.toml test
test_err "invalid test driver" -f dep-invalid.lean test
test_err "invalid test driver" -f dep-invalid.toml test
test_err "only one" -f two.lean lint
test_err "no lint driver" -f none.lean lint
test_err "no lint driver" -f none.toml lint
test_err "invalid lint driver: unknown" -f unknown.lean lint
test_err "invalid lint driver: unknown" -f unknown.toml lint
test_err "unknown lint driver package" -f dep-unknown.lean lint
test_err "unknown lint driver package" -f dep-unknown.toml lint
test_err "invalid lint driver" -f dep-invalid.lean lint
test_err "invalid lint driver" -f dep-invalid.toml lint
test_err "only one" -f two.lean bench
test_err "no bench driver" -f none.lean bench
test_err "no bench driver" -f none.toml bench
test_err "invalid bench driver: unknown" -f unknown.lean bench
test_err "invalid bench driver: unknown" -f unknown.toml bench
test_err "unknown bench driver package" -f dep-unknown.lean bench
test_err "unknown bench driver package" -f dep-unknown.toml bench
test_err "invalid bench driver" -f dep-invalid.lean bench
test_err "invalid bench driver" -f dep-invalid.toml bench

# Driver checker
echo "# TEST: Check driver"
test_run -f exe.lean check-test
test_run -f exe.toml check-test
test_run -f exe.lean check-lint
test_run -f exe.toml check-lint
test_run -f exe.lean check-bench
test_run -f exe.toml check-bench
test_run -f dep.lean check-test
test_run -f dep.toml check-test
test_run -f dep.lean check-lint
test_run -f dep.toml check-lint
test_run -f dep.lean check-bench
test_run -f dep.toml check-bench
test_run -f script.lean check-test
test_run -f script.lean check-lint
test_run -f script.lean check-bench
test_run -f lib.lean check-test
test_run -f lib.lean check-bench
test_fails -f two.lean check-test
test_fails -f two.lean check-lint
test_fails -f two.lean check-bench
test_fails -f none.lean check-test
test_fails -f none.toml check-test
test_fails -f none.lean check-lint
test_fails -f none.toml check-lint
test_fails -f none.lean check-bench
test_fails -f none.toml check-bench

# Build checker
echo "# TEST: Check build"
test_run -f build.lean check-build
test_run -f build.toml check-build
test_fails -f none.lean check-build
test_fails -f none.toml check-build

# Cleanup
rm -f produced.out
