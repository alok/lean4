#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
stage1_bin="${LEAN_STAGE1_BIN:-"$root/build/release/stage1/bin"}"
lake_bin="${LAKE_BIN:-"$stage1_bin/lake"}"
lean4checker_bin="${LEAN4CHECKER_BIN:-"$root/../lean4checker/.lake/build/bin/lean4checker"}"
lean4lean_bin="${LEAN4LEAN_BIN:-"$root/../lean4lean/.lake/build/bin/lean4lean"}"
lean4lean_args="${LEAN4LEAN_ARGS:-}"
aeneas_verify="${RIIR_AENEAS:-0}"
ffi_tests=()

profile="${RIIR_PROFILE:-full}"

case "$profile" in
  fast)
    default_modules="Init Lean"
    kernel_tests=(
      kernel1.lean
      kernel2.lean
    )
    ;;
  full)
    default_modules="Init Lean Std"
    kernel_tests=(
      kernel1.lean
      kernel2.lean
      kernelBacktrack.lean
      kernelErrorFollowup.lean
      kernelInterrupt.lean
      kernel_maxheartbeats.lean
      decideTacticKernel.lean
      skipKernelTC.lean
    )
    ffi_tests=(
      tests/lake/examples/ffi/test.sh
      tests/lake/examples/reverse-ffi/test.sh
      tests/lake/tests/externLib/test.sh
    )
    ;;
  ffi)
    default_modules=""
    kernel_tests=()
    ffi_tests=(
      tests/lake/examples/ffi/test.sh
      tests/lake/examples/reverse-ffi/test.sh
      tests/lake/tests/externLib/test.sh
    )
    ;;
  *)
    echo "unknown RIIR_PROFILE: $profile (expected fast or full)" >&2
    exit 2
    ;;
esac

lean4checker_modules="${LEAN4CHECKER_MODULES:-"$default_modules"}"

lean4lean_compare="${LEAN4LEAN_COMPARE:-1}"
if [[ "$lean4lean_compare" != "0" && "$lean4lean_compare" != "false" ]]; then
  lean4lean_args="$lean4lean_args --compare"
fi

if [[ ! -x "$lake_bin" ]]; then
  echo "lake not found: $lake_bin" >&2
  exit 1
fi

if [[ ! -x "$lean4checker_bin" ]]; then
  echo "lean4checker not found: $lean4checker_bin" >&2
  exit 1
fi

if [[ ! -x "$lean4lean_bin" ]]; then
  echo "lean4lean not found: $lean4lean_bin" >&2
  exit 1
fi

make -j -C "$root/build/release"

export PATH="$stage1_bin:$PATH"

for t in "${kernel_tests[@]}"; do
  (cd "$root/tests/lean/run" && ./test_single.sh "$t")
done

if [[ -n "$lean4checker_modules" ]]; then
  for m in $lean4checker_modules; do
    "$lake_bin" env "$lean4checker_bin" ${LEAN4CHECKER_ARGS:-} "$m"
    "$lake_bin" env "$lean4lean_bin" $lean4lean_args "$m"
  done
fi

if [[ ${#ffi_tests[@]} -ne 0 ]]; then
  for t in "${ffi_tests[@]}"; do
    (cd "$root/$(dirname "$t")" && LAKE="$lake_bin" LAKE_NO_CACHE=1 LAKE_CACHE_DIR="" ./$(basename "$t"))
  done
fi

if [[ "$aeneas_verify" != "0" && "$aeneas_verify" != "false" ]]; then
  "$root/script/run_aeneas.sh"
fi
