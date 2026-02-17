#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK_DIR="$ROOT_DIR/.git/hooks"
mkdir -p "$HOOK_DIR"

cp "$ROOT_DIR/scripts/pre-commit" "$HOOK_DIR/pre-commit"
chmod +x "$HOOK_DIR/pre-commit"

echo "Installed pre-commit hook to .git/hooks/pre-commit"
