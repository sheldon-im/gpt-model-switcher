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
# Prepare the sidecar before the Tauri CLI checks external binary paths.
mkdir -p target/bridge-build/swift-cache
xcrun swiftc -O -module-cache-path target/bridge-build/swift-cache -target arm64-apple-macos13.0 -framework Cocoa -framework ApplicationServices src-tauri/native/MacBridge.swift -o src-tauri/native/macos-bridge-aarch64-apple-darwin
codesign --force --sign - --identifier io.github.gpt-model-switcher.bridge src-tauri/native/macos-bridge-aarch64-apple-darwin
# CI mode skips Finder's cosmetic AppleScript and interactive packaging prompts.
CI=true pnpm tauri build --ci --target aarch64-apple-darwin --config src-tauri/tauri.macos.conf.json -- --locked
