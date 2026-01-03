#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: script/run_external_checkers.sh [module]

Runs external kernel checkers against the current Lean project.

Env overrides:
  LEAN4CHECKER_BIN  Path to lean4checker binary
  LEAN4LEAN_BIN     Path to lean4lean binary
  LEAN4LEAN_COMPARE Set to 1 to pass --compare to lean4lean
  LEAN4CHECKER_ARGS Extra args for lean4checker
  LEAN4LEAN_ARGS    Extra args for lean4lean
  LAKE_BIN          Path to lake (default: lake)

Defaults:
  ../lean4checker/.lake/build/bin/lean4checker
  ../lean4lean/.lake/build/bin/lean4lean
USAGE
}

if [[ ${1:-} == "-h" || ${1:-} == "--help" ]]; then
  usage
  exit 0
fi

module=${1:-}

lake_bin=${LAKE_BIN:-lake}
lean4checker_bin=${LEAN4CHECKER_BIN:-"$(pwd)/../lean4checker/.lake/build/bin/lean4checker"}
lean4lean_bin=${LEAN4LEAN_BIN:-"$(pwd)/../lean4lean/.lake/build/bin/lean4lean"}

if [[ ! -x "$lean4checker_bin" ]]; then
  echo "lean4checker not found: $lean4checker_bin" >&2
  exit 1
fi

if [[ ! -x "$lean4lean_bin" ]]; then
  echo "lean4lean not found: $lean4lean_bin" >&2
  exit 1
fi

lean4checker_args=${LEAN4CHECKER_ARGS:-}
lean4lean_args=${LEAN4LEAN_ARGS:-}

if [[ -n ${LEAN4LEAN_COMPARE:-} ]]; then
  lean4lean_args="$lean4lean_args --compare"
fi

if [[ -n "$module" ]]; then
  $lake_bin env "$lean4checker_bin" $lean4checker_args "$module"
  $lake_bin env "$lean4lean_bin" $lean4lean_args "$module"
else
  $lake_bin env "$lean4checker_bin" $lean4checker_args
  $lake_bin env "$lean4lean_bin" $lean4lean_args
fi
