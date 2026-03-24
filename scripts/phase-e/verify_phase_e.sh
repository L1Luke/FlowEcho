#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FLUTTER_BIN="/opt/homebrew/bin/flutter"
XCODE_DEV_DIR="/Applications/Xcode.app/Contents/Developer"

echo "[Phase E] 1/4 Rust tests"
(
  cd "${ROOT_DIR}"
  cargo test -p flowecho_core
)

echo "[Phase E] 2/4 Flutter analyze"
(
  cd "${ROOT_DIR}/flutter/flowecho_app"
  FLUTTER_SUPPRESS_ANALYTICS=true "${FLUTTER_BIN}" --suppress-analytics analyze
)

echo "[Phase E] 3/4 Flutter tests"
(
  cd "${ROOT_DIR}/flutter/flowecho_app"
  FLUTTER_SUPPRESS_ANALYTICS=true "${FLUTTER_BIN}" --suppress-analytics test
)

echo "[Phase E] 4/4 iOS build (no signing)"
(
  cd "${ROOT_DIR}"
  DEVELOPER_DIR="${XCODE_DEV_DIR}" \
    xcodebuild \
      -project FlowEcho.xcodeproj \
      -scheme FlowEcho \
      -destination "generic/platform=iOS" \
      CODE_SIGNING_ALLOWED=NO \
      build
)

echo "[Phase E] verification completed"
