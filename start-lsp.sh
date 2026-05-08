#!/usr/bin/bash
# start-lsp.sh — per-project LSP launcher

echo "=== start-lsp.sh ran for $(basename "$(pwd)") ===" >&2

# Put any project-specific env vars or setup here (toolchain, RUST_BACKTRACE, PYTHONPATH, etc.)

# === Choose the right server for this project ===
# Rust
exec rust-analyzer "$@"

# Python (uncomment whichever you prefer)
# exec pyright-langserver --stdio "$@"
# exec pylsp "$@"
# exec ruff-lsp "$@"

# Bash / shell
# exec bash-language-server start "$@"

# You can add more languages later the same way.
