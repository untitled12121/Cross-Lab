#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="${1:-${repo_root}/target/generated/uniffi/kotlin}"

cd "${repo_root}"
cargo build --locked -p crosslab-mobile-ffi --lib

case "$(uname -s)" in
  Darwin)
    library="${repo_root}/target/debug/libcrosslab_mobile_ffi.dylib"
    ;;
  Linux)
    library="${repo_root}/target/debug/libcrosslab_mobile_ffi.so"
    ;;
  CYGWIN*|MINGW*|MSYS*)
    library="${repo_root}/target/debug/crosslab_mobile_ffi.dll"
    ;;
  *)
    echo "unsupported host for UniFFI binding generation" >&2
    exit 1
    ;;
esac

mkdir -p "${out_dir}"

cargo run --locked -p crosslab-mobile-ffi --features bindgen --bin crosslab-uniffi-bindgen -- \
  generate --library "${library}" --language kotlin --no-format --out-dir "${out_dir}"
