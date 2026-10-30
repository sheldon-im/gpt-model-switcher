# Source from the project root. Reuse optional project-local tools without
# changing the user's shell startup files or global tool installations.
if [ -x "$PWD/work/tooling/cargo/bin/cargo" ]; then
  export CARGO_HOME="$PWD/work/tooling/cargo"
  export RUSTUP_HOME="$PWD/work/tooling/rustup"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
if ! command -v node >/dev/null 2>&1; then
  switcher_node_bin="$HOME/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin"
  if [ -x "$switcher_node_bin/node" ]; then
    export PATH="$switcher_node_bin:$PATH"
  fi
fi
if [ -f "$PWD/work/tooling/pnpm10/bin/pnpm.cjs" ]; then
  ln -sf pnpm.cjs "$PWD/work/tooling/pnpm10/bin/pnpm"
  export PATH="$PWD/work/tooling/pnpm10/bin:$PATH"
fi
