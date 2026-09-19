#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_root="${1:-${repo_root}/target/generated/android-jni}"
abi="${2:-arm64-v8a}"
api_level=23
ndk_version=28.2.13676358

case "${abi}" in
  arm64-v8a)
    rust_target="aarch64-linux-android"
    clang_prefix="aarch64-linux-android"
    ;;
  x86_64)
    rust_target="x86_64-linux-android"
    clang_prefix="x86_64-linux-android"
    ;;
  *)
    echo "unsupported M10 Android ABI: ${abi}" >&2
    exit 1
    ;;
esac

ndk_root="${ANDROID_NDK_ROOT:-${ANDROID_NDK_HOME:-}}"
if [[ -z "${ndk_root}" && -n "${ANDROID_SDK_ROOT:-}" ]]; then
  ndk_root="${ANDROID_SDK_ROOT}/ndk/${ndk_version}"
fi
if [[ -z "${ndk_root}" && -n "${ANDROID_HOME:-}" ]]; then
  ndk_root="${ANDROID_HOME}/ndk/${ndk_version}"
fi
if [[ -z "${ndk_root}" || ! -d "${ndk_root}" ]]; then
  echo "Android NDK ${ndk_version} is required; set ANDROID_NDK_ROOT or ANDROID_SDK_ROOT" >&2
  exit 1
fi

case "$(uname -s)" in
  Linux)
    host_tag="linux-x86_64"
    ;;
  Darwin)
    host_tag="darwin-x86_64"
    ;;
  *)
    echo "unsupported host for Android Rust build" >&2
    exit 1
    ;;
esac

toolchain_bin="${ndk_root}/toolchains/llvm/prebuilt/${host_tag}/bin"
linker="${toolchain_bin}/${clang_prefix}${api_level}-clang"
cxx="${toolchain_bin}/${clang_prefix}${api_level}-clang++"
archiver="${toolchain_bin}/llvm-ar"
if [[ ! -x "${linker}" || ! -x "${cxx}" || ! -x "${archiver}" ]]; then
  echo "Android NDK toolchain is incomplete for ${abi}" >&2
  exit 1
fi

target_env="$(printf '%s' "${rust_target}" | tr '[:lower:]-' '[:upper:]_')"
linker_env="CARGO_TARGET_${target_env}_LINKER"
export "${linker_env}=${linker}"
export CC="${linker}"
export CXX="${cxx}"
export AR="${archiver}"

cd "${repo_root}"
rustup target add "${rust_target}"

cargo_args=(build --locked -p crosslab-mobile-ffi --target "${rust_target}")
if [[ -n "${CROSSLAB_MOBILE_FFI_FEATURES:-}" ]]; then
  cargo_args+=(--features "${CROSSLAB_MOBILE_FFI_FEATURES}")
fi
cargo "${cargo_args[@]}"

mkdir -p "${out_root}/${abi}"
cp \
  "${repo_root}/target/${rust_target}/debug/libcrosslab_mobile_ffi.so" \
  "${out_root}/${abi}/libcrosslab_mobile_ffi.so"
