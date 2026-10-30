#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
[ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ]
mkdir -p target/bridge-tests
xcrun swiftc -D BRIDGE_TESTS -O -module-cache-path target/bridge-tests/swift-cache \
  -target arm64-apple-macos13.0 -framework Cocoa -framework ApplicationServices \
  src-tauri/native/MacBridge.swift -o target/bridge-tests/macos-bridge-tests
/usr/bin/python3 - <<'PY'
import json, subprocess
helper = 'target/bridge-tests/macos-bridge-tests'
def run(*args):
    return json.loads(subprocess.check_output([helper, *args], text=True, timeout=5))
assert run('self-test')['passed'] == 14
for args in [('check',), ('check', '{}'), ('check', 'null'), ('check', '{'), ('check', '[]')]:
    assert 'error' in run(*args), args
print('Swift bridge: 14 synthetic checks and 5 malformed-request checks passed. No AX or model writes.')
PY
