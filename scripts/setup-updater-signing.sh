#!/usr/bin/env bash
set -euo pipefail

# Generate a Tauri updater keypair and store the CI secrets in GitHub.
#
#   scripts/setup-updater-signing.sh
#
# The private key is written outside the repository (default
# ~/.tauri/fm27-scout-updater.key). Set UPDATER_KEY_PATH to choose another location,
# UPDATER_KEY_PASSWORD to protect it with a password, and REPO to override the GitHub
# repository slug.
#
# Existing keys are never overwritten unless FORCE=1 is set.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="${REPO:-$(git -C "$ROOT" remote get-url origin | sed -E 's#(git@github.com:|https://github.com/)##; s#\.git$##')}"
KEY_PATH="${UPDATER_KEY_PATH:-$HOME/.tauri/fm27-scout-updater.key}"
CONFIG_PATH="${UPDATER_CONFIG_PATH:-$HOME/.tauri/fm27-scout-updater.tauri.json}"
PASSWORD="${UPDATER_KEY_PASSWORD:-}"

if [[ ! "$REPO" =~ ^[^/]+/[^/]+$ ]]; then
  echo "error: could not derive a GitHub repository from origin; set REPO=owner/name" >&2
  exit 1
fi
if [[ "$KEY_PATH" == "$ROOT"* ]]; then
  echo "error: refuse to write a private key inside the repository ($KEY_PATH)" >&2
  exit 1
fi
if [[ "$CONFIG_PATH" == "$ROOT"* ]]; then
  echo "error: refuse to write the updater config inside the repository ($CONFIG_PATH)" >&2
  exit 1
fi
if ! command -v gh >/dev/null 2>&1; then
  echo "error: gh is required to set repository secrets" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required to write the local Tauri config" >&2
  exit 1
fi
if ! gh auth status >/dev/null 2>&1; then
  echo "error: run 'gh auth login' first" >&2
  exit 1
fi
if [[ -e "$KEY_PATH" && "${FORCE:-0}" != "1" ]]; then
  echo "error: $KEY_PATH already exists; set FORCE=1 to replace it" >&2
  exit 1
fi

mkdir -p "$(dirname "$KEY_PATH")"
cd "$ROOT/apps/desktop"
SIGNER_ARGS=(signer generate --ci -w "$KEY_PATH" -f)
if [[ -n "$PASSWORD" ]]; then
  SIGNER_ARGS+=(--password "$PASSWORD")
fi
pnpm tauri "${SIGNER_ARGS[@]}"

PUBLIC_KEY="$(cat "$KEY_PATH.pub")"
jq -n --arg pubkey "$PUBLIC_KEY" \
  '{bundle:{createUpdaterArtifacts:true},plugins:{updater:{pubkey:$pubkey}}}' \
  > "$CONFIG_PATH"
gh secret set FM27_UPDATER_PUBKEY --repo "$REPO" --body "$PUBLIC_KEY"
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo "$REPO" --body "$(cat "$KEY_PATH")"
if [[ -n "$PASSWORD" ]]; then
  gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo "$REPO" --body "$PASSWORD"
else
  gh secret delete TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo "$REPO" >/dev/null 2>&1 || true
fi

echo
echo "Configured updater secrets for $REPO."
echo "Private key: $KEY_PATH (keep it secret and back it up)"
echo "Public key:  $KEY_PATH.pub"
echo
echo "For a local signed build:"
if [[ -n "$PASSWORD" ]]; then
  echo "  export TAURI_SIGNING_PRIVATE_KEY=\"$KEY_PATH\""
  echo "  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD='…'"
else
  echo "  export TAURI_SIGNING_PRIVATE_KEY=\"$KEY_PATH\""
fi
echo "  cd apps/desktop && pnpm tauri build --config \"$CONFIG_PATH\""
