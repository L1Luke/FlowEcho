#!/usr/bin/env bash
set -euo pipefail

src_root="${SRCROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
rustup_bin="/opt/homebrew/opt/rustup/bin/rustup"

if command -v rustup >/dev/null 2>&1; then
  rustup_bin="$(command -v rustup)"
fi

if [[ ! -x "$rustup_bin" ]]; then
  echo "error: rustup is required to build flowecho_core for Apple targets."
  echo "Install with: brew install rustup && /opt/homebrew/opt/rustup/bin/rustup default stable"
  exit 1
fi

case "${PLATFORM_NAME:-macosx}" in
  macosx)
    current_arch="${CURRENT_ARCH:-}"
    if [[ -z "$current_arch" || "$current_arch" == "undefined_arch" ]]; then
      current_arch="${NATIVE_ARCH_ACTUAL:-$(uname -m)}"
    fi
    case "$current_arch" in
      arm64|arm64e)
        rust_target="aarch64-apple-darwin"
        ;;
      x86_64)
        rust_target="x86_64-apple-darwin"
        ;;
      *)
        echo "error: unsupported macOS arch ${current_arch:-unknown}"
        exit 1
        ;;
    esac
    ;;
  iphoneos)
    rust_target="aarch64-apple-ios"
    ;;
  iphonesimulator)
    if [[ "${CURRENT_ARCH:-arm64}" == "x86_64" ]]; then
      rust_target="x86_64-apple-ios"
    else
      rust_target="aarch64-apple-ios-sim"
    fi
    ;;
  *)
    echo "error: unsupported Apple platform ${PLATFORM_NAME:-unknown}"
    exit 1
    ;;
esac

profile_dir="debug"
if [[ "${CONFIGURATION:-Debug}" == "Release" ]]; then
  profile_dir="release"
fi

export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

if ! "$rustup_bin" target list --installed | grep -qx "$rust_target"; then
  "$rustup_bin" target add "$rust_target"
fi

if [[ "$profile_dir" == "release" ]]; then
  "$rustup_bin" run stable cargo build \
    --manifest-path "$src_root/rust/flowecho_core/Cargo.toml" \
    --target "$rust_target" \
    --lib \
    --release
else
  "$rustup_bin" run stable cargo build \
    --manifest-path "$src_root/rust/flowecho_core/Cargo.toml" \
    --target "$rust_target" \
    --lib
fi

output_dir="${BUILT_PRODUCTS_DIR}/rust"
mkdir -p "$output_dir"
cp "$src_root/target/$rust_target/$profile_dir/libflowecho_core.a" \
  "$output_dir/libflowecho_core.a"
