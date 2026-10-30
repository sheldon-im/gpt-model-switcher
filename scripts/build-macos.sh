#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
[ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] || { echo 'Requires Apple Silicon macOS'; exit 1; }
xcode-select -p >/dev/null
pnpm install --frozen-lockfile
cargo test --workspace --locked
pnpm test
pnpm build
# Prepare the sidecar before the Tauri CLI checks external binary paths.
xcrun swiftc -O -target arm64-apple-macos13.0 -framework Cocoa -framework ApplicationServices src-tauri/native/MacBridge.swift -o src-tauri/native/macos-bridge-aarch64-apple-darwin
codesign --force --sign - --identifier io.github.gpt-model-switcher.bridge src-tauri/native/macos-bridge-aarch64-apple-darwin
pnpm tauri build --target aarch64-apple-darwin --config src-tauri/tauri.macos.conf.json
