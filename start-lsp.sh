#!/usr/bin/bash
# start-lsp.sh
# per-project LSP launcher.

echo "=== start-lsp.sh ran for $(basename "$(pwd)") ===" >&2

# Put any project-specific env vars or setup here (toolchain, RUST_BACKTRACE, PYTHONPATH, etc.)

# === Choose the right server for this project ===
# Rust
#exec rust-analyzer "$@"

# Python (uncomment whichever you prefer)
# exec pyright-langserver --stdio "$@"
# exec pylsp "$@"
# exec ruff-lsp "$@"
#uv run basedpyright-langserver --stdio

# Bash / shell
# exec bash-language-server start "$@"

# You can add more languages later the same way.

# NOTE: sh lsp command assumes node is available and bash-language-server has been installed.
# (either local or global installation.)

case "$1" in
  python) exec uv run basedpyright-langserver --stdio ;;
  rust)   exec rust-analyzer ;;
  #sh)     exec shellcheck --lsp ;;
  sh)     exec npx bash-language-server start ;;
  *)      exec uv run basedpyright-langserver --stdio ;;
esac
