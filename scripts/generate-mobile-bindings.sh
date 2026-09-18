#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="${1:-${repo_root}/target/generated/uniffi/kotlin}"

mkdir -p "${out_dir}"
cd "${repo_root}"

cargo run --locked   -p crosslab-mobile-ffi   --features bindgen   --bin crosslab-uniffi-bindgen   --   generate   --language kotlin   --no-format   --out-dir "${out_dir}"   src:crosslab-mobile-ffi
