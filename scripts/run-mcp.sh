#!/usr/bin/env bash
# Resolve nimvault-mcp for Grok/Claude plugin installs (GROK_PLUGIN_ROOT / CLAUDE_PLUGIN_ROOT).
set -euo pipefail
ROOT="${GROK_PLUGIN_ROOT:-${CLAUDE_PLUGIN_ROOT:-}}"
candidates=()
if [[ -n "${NIMVAULT_MCP_BIN:-}" ]]; then candidates+=("$NIMVAULT_MCP_BIN"); fi
if command -v nimvault-mcp >/dev/null 2>&1; then candidates+=("$(command -v nimvault-mcp)"); fi
if [[ -n "$ROOT" ]]; then
  candidates+=(
    "$ROOT/target/release/nimvault-mcp"
    "$ROOT/target/debug/nimvault-mcp"
    "$ROOT/npm/bin/nimvault-mcp"
  )
fi
candidates+=("$HOME/.local/bin/nimvault-mcp" "$HOME/.cargo/bin/nimvault-mcp")

for c in "${candidates[@]}"; do
  if [[ -x "$c" ]]; then
    exec "$c" "$@"
  fi
done

# Last resort: cargo run from plugin source (slow first time)
if [[ -n "$ROOT" && -f "$ROOT/Cargo.toml" ]] && command -v cargo >/dev/null 2>&1; then
  exec cargo run --quiet --manifest-path "$ROOT/Cargo.toml" --release --bin nimvault-mcp -- "$@"
fi

echo "nimvault-mcp: binary not found. Install with:" >&2
echo "  cargo install --git https://github.com/HaoZeke/nimvault-mcp" >&2
echo "  # or: nimble install nimvault  (CLI) + cargo install --path $ROOT" >&2
exit 127
