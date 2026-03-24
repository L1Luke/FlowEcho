#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

cargo test -p flowecho_core --release --test transfer_tcp_recovery_test one_gib_file_transfer_stays_stable -- --ignored --nocapture
