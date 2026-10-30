#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
[ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] || { echo 'Requires Apple Silicon macOS'; exit 1; }
xcode-select -p >/dev/null
. ./scripts/macos-env.sh
pnpm install --frozen-lockfile --store-dir work/tooling/pnpm-store
cargo test --workspace --locked
pnpm test
pnpm build
sh scripts/test-macos-bridge.sh
# build.rs links the Swift bridge into the app, so AX uses the app's permission.
# CI mode skips Finder's cosmetic AppleScript and interactive packaging prompts.
CI=true pnpm tauri build --ci --target aarch64-apple-darwin --config src-tauri/tauri.macos.conf.json -- --locked
