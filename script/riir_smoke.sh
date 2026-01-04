#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
stage1_bin="${LEAN_STAGE1_BIN:-"$root/build/release/stage1/bin"}"
lake_bin="${LAKE_BIN:-"$stage1_bin/lake"}"
lean4checker_bin="${LEAN4CHECKER_BIN:-"$root/../lean4checker/.lake/build/bin/lean4checker"}"
lean4lean_bin="${LEAN4LEAN_BIN:-"$root/../lean4lean/.lake/build/bin/lean4lean"}"
lean4checker_modules="${LEAN4CHECKER_MODULES:-"Init Lean"}"
lean4lean_args="${LEAN4LEAN_ARGS:-}"

if [[ -n ${LEAN4LEAN_COMPARE:-} ]]; then
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

for t in "${kernel_tests[@]}"; do
  (cd "$root/tests/lean/run" && ./test_single.sh "$t")
done

if [[ -n "$lean4checker_modules" ]]; then
  for m in $lean4checker_modules; do
    "$lake_bin" env "$lean4checker_bin" ${LEAN4CHECKER_ARGS:-} "$m"
    "$lake_bin" env "$lean4lean_bin" $lean4lean_args "$m"
  done
fi
