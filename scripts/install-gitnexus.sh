#!/usr/bin/env bash
# Install GitNexus (https://github.com/abhigyanpatwari/GitNexus) for this
# repository — indexes the codebase into a local LadybugDB graph database
# and optionally registers the MCP server for any agent runtime that's on
# PATH (Claude Code, Codex, Cursor, OpenCode, …).
#
# GitNexus is the Phase 4 piece of the cross-repo self-learning roadmap:
# the brain (this repo, ruvector) needs structural awareness of its own
# 150+ crate workspace before it can reason about cross-repo refactors
# with weftos. The graph lives entirely on disk — no external service.
#
# Idempotent: safe to re-run. The CLI itself is staleness-aware (checks
# git HEAD against the indexed snapshot) and only re-walks changed files
# unless --force is passed.
#
# License note: GitNexus ships under PolyForm Noncommercial. This script
# only invokes the upstream CLI; no GitNexus code is vendored into this
# repo. If you have a commercial license arrangement with akonlabs.com,
# nothing here changes — it just makes the CLI available to your agents.

set -euo pipefail

GITNEXUS_VERSION="${GITNEXUS_VERSION:-latest}"

log()  { printf '\033[1;34m[gitnexus]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[gitnexus]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[1;31m[gitnexus]\033[0m %s\n' "$*" >&2; exit 1; }

# ── Preflight ────────────────────────────────────────────────────────
if ! command -v npx >/dev/null 2>&1; then
  fail "npx not on PATH. GitNexus requires Node.js 20+ — see .devin/setup.sh."
fi

# Node version check. GitNexus needs Node 20+ per its package.json engines
# field; older Node will fail with a cryptic syntax error inside the npx
# download, so we surface the version up-front.
node_major="$(node -v 2>/dev/null | sed -E 's/^v([0-9]+).*/\1/' || echo 0)"
if [[ "$node_major" -lt 20 ]]; then
  fail "Node.js >= 20 required (detected v${node_major}). Run .devin/setup.sh first."
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# ── Index ────────────────────────────────────────────────────────────
# `analyze` is idempotent: it checks git HEAD against the indexed
# snapshot and only walks changed files. `--skip-agents-md` is critical:
# without it, GitNexus rewrites CLAUDE.md / AGENTS.md and would clobber
# the hand-curated rules at the top of CLAUDE.md (workspace exclusion
# rules, lint policy, etc.). We keep our context files authoritative.
log "indexing repo with gitnexus@${GITNEXUS_VERSION} (writes to .gitnexus/)"
log "  — using --skip-agents-md to preserve hand-curated CLAUDE.md"
if [[ "${GITNEXUS_FORCE:-0}" == "1" ]]; then
  log "  — GITNEXUS_FORCE=1 set; forcing full re-index"
  npx -y "gitnexus@${GITNEXUS_VERSION}" analyze --skip-agents-md --force
else
  npx -y "gitnexus@${GITNEXUS_VERSION}" analyze --skip-agents-md
fi

# ── MCP registration ────────────────────────────────────────────────
# `gitnexus setup` writes per-editor MCP configs (~/.cursor/mcp.json,
# ~/.config/opencode/config.json, etc.) and is editor-aware — it only
# touches the configs of editors that are actually installed. Safe to
# re-run.
#
# We invoke setup unconditionally (not gated on a specific CLI being
# present) because it auto-detects and skips missing editors.
log "registering MCP server for any installed agent runtime"
npx -y "gitnexus@${GITNEXUS_VERSION}" setup || warn "gitnexus setup returned non-zero — MCP may need manual config; see .gitnexus/README.md"

# ── Done ─────────────────────────────────────────────────────────────
log "GitNexus install complete."
log ""
log "  Index location:  $REPO_ROOT/.gitnexus/  (gitignored)"
log "  Registry:        ~/.gitnexus/registry.json"
log "  MCP server:      npx -y gitnexus@latest mcp"
log ""
log "Quick smoke-test (run from repo root):"
log "  npx -y gitnexus@${GITNEXUS_VERSION} status"
log "  npx -y gitnexus@${GITNEXUS_VERSION} list"
log ""
log "From within an MCP-aware agent: ask it to call the gitnexus 'context'"
log "or 'impact' tool with a symbol name to confirm it's wired up."
