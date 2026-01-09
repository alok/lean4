#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: script/run_aeneas.sh

Runs Charon + Aeneas over the safe verification crate.

Env overrides:
  AENEAS_ROOT         Path to Aeneas repo (default: ../aeneas)
  CHARON_BIN          Path to charon binary
  AENEAS_BIN          Path to aeneas binary
  AENEAS_CRATE_DIR    Path to Rust crate to translate (default: src/rust/verify)
  AENEAS_LLBC_DIR     Destination directory for .llbc (default: <crate>/target/charon)
  AENEAS_OUT_DIR      Output directory for Aeneas backend files (default: <crate>/target/aeneas)
  AENEAS_BACKEND      Backend (default: lean)
  AENEAS_SPLIT_FILES  Set to 0 to disable -split-files (default: 1)
  AENEAS_GEN_LAKEFILE Set to 1 to enable -lean-default-lakefile (default: 0)
USAGE
}

if [[ ${1:-} == "-h" || ${1:-} == "--help" ]]; then
  usage
  exit 0
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
aeneas_root="${AENEAS_ROOT:-"$root/../aeneas"}"
crate_dir="${AENEAS_CRATE_DIR:-"$root/src/rust/verify"}"
llbc_dir="${AENEAS_LLBC_DIR:-"$crate_dir/target/charon"}"
out_dir="${AENEAS_OUT_DIR:-"$crate_dir/target/aeneas"}"
backend="${AENEAS_BACKEND:-lean}"

charon_bin="${CHARON_BIN:-"$aeneas_root/charon/bin/charon"}"
if [[ ! -x "$charon_bin" ]]; then
  charon_bin="${CHARON_BIN:-"$aeneas_root/bin/charon"}"
fi
aeneas_bin="${AENEAS_BIN:-"$aeneas_root/bin/aeneas"}"

if [[ ! -x "$charon_bin" ]]; then
  echo "charon not found: $charon_bin" >&2
  exit 1
fi

if [[ ! -x "$aeneas_bin" ]]; then
  echo "aeneas not found: $aeneas_bin" >&2
  exit 1
fi

mkdir -p "$llbc_dir" "$out_dir"

llbc_file="${AENEAS_LLBC_FILE:-"$llbc_dir/kernel_verify.llbc"}"

"$charon_bin" cargo --preset=aeneas --dest-file "$llbc_file" -- --manifest-path "$crate_dir/Cargo.toml"
if [[ ! -f "$llbc_file" ]]; then
  llbc_file="$(fd -t f -e llbc . "$llbc_dir" | head -n 1 || true)"
fi

if [[ -z "${llbc_file:-}" || ! -f "$llbc_file" ]]; then
  echo "no .llbc file found under: $llbc_dir" >&2
  exit 1
fi

split_files="${AENEAS_SPLIT_FILES:-1}"
gen_lakefile="${AENEAS_GEN_LAKEFILE:-0}"

args=("-backend" "$backend" "-dest" "$out_dir")
if [[ "$split_files" != "0" && "$split_files" != "false" ]]; then
  args+=("-split-files" "-gen-lib-entry")
fi
if [[ "$backend" == "lean" && "$gen_lakefile" != "0" && "$gen_lakefile" != "false" ]]; then
  args+=("-lean-default-lakefile")
fi

"$aeneas_bin" "${args[@]}" "$llbc_file"
