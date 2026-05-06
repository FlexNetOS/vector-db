#!/usr/bin/env bash
# Install MemPalace (https://github.com/MemPalace/mempalace) for this
# repository — local-first AI memory store with verbatim retrieval and
# semantic search via ChromaDB. The palace itself lives at ~/.mempalace/
# (per-user, NOT in this repo); this script installs the CLI, scopes a
# wing for this repo via `mempalace init <root>`, and surfaces the
# Claude Code plugin install command for any agent runtime on PATH.
#
# MemPalace is the Phase 5 piece of the cross-repo self-learning
# roadmap: the brain (ruvector) needs persistent memory across sessions
# so trajectories from prior runs can be recalled before the next
# /understand or attractor pass kicks off. Combined with Phase 4
# (GitNexus structural graph) and Phase 1 (Understand-Anything
# comprehension graph), this gives the agent three complementary
# retrieval surfaces: structure, comprehension, and history.
#
# Idempotent: safe to re-run. `pip install --upgrade` and
# `mempalace init` are both stable across invocations.
#
# License note: MemPalace ships under MIT. This script only invokes the
# upstream CLI; no MemPalace code is vendored.
#
# SECURITY NOTE: the legitimate MemPalace project is hosted ONLY at
#   https://github.com/MemPalace/mempalace
#   https://pypi.org/project/mempalace/
#   https://mempalaceofficial.com/
# The domain `mempalace.tech` is a known impostor. Do NOT install from
# any other source. Upstream README: docs/HISTORY.md.

set -euo pipefail

MEMPALACE_VERSION="${MEMPALACE_VERSION:-latest}"
MEMPALACE_PIP_SPEC="mempalace"
if [[ "$MEMPALACE_VERSION" != "latest" ]]; then
  MEMPALACE_PIP_SPEC="mempalace==${MEMPALACE_VERSION}"
fi

# All helpers route to stderr so callers can capture script output without
# mixing in informational chatter (matches the in-repo logging convention).
log()  { printf '\033[1;35m[mempalace]\033[0m %s\n' "$*" >&2; }
warn() { printf '\033[1;33m[mempalace]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[1;31m[mempalace]\033[0m %s\n' "$*" >&2; exit 1; }

# ── Preflight ────────────────────────────────────────────────────────
# MemPalace requires Python 3.9+ per its setup.cfg. ChromaDB pulls in
# numpy, sentence-transformers, etc. — first install downloads the
# all-MiniLM-L6-v2 embedding model (~80MB), one-time only.
if ! command -v python3 >/dev/null 2>&1; then
  fail "python3 not on PATH. MemPalace requires Python 3.9+."
fi

py_major=$(python3 -c 'import sys; print(sys.version_info[0])' 2>/dev/null || echo 0)
py_minor=$(python3 -c 'import sys; print(sys.version_info[1])' 2>/dev/null || echo 0)
if [[ "$py_major" -lt 3 ]] || { [[ "$py_major" -eq 3 ]] && [[ "$py_minor" -lt 9 ]]; }; then
  fail "Python >= 3.9 required (detected ${py_major}.${py_minor})."
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# ── Install ──────────────────────────────────────────────────────────
# Prefer pipx if available (isolated venv per CLI is the recommended
# packaging for end-user Python tools), otherwise fall back to
# `pip install --user`. We avoid system-wide installs to stay sudo-free.
if command -v pipx >/dev/null 2>&1; then
  log "installing ${MEMPALACE_PIP_SPEC} via pipx (isolated venv)"
  pipx install --force "${MEMPALACE_PIP_SPEC}" || fail "pipx install failed"
else
  log "installing ${MEMPALACE_PIP_SPEC} via pip --user (pipx not found)"
  log "  — install pipx for cleaner CLI isolation: pip install --user pipx"
  python3 -m pip install --user --upgrade "${MEMPALACE_PIP_SPEC}" || \
    fail "pip install failed — see https://github.com/MemPalace/mempalace#installation"
fi

# Locate the installed CLI. pipx puts it under ~/.local/bin; pip --user
# also targets ~/.local/bin on POSIX. Surface the path in case it isn't
# on PATH yet (common after a fresh pip --user install).
if ! command -v mempalace >/dev/null 2>&1; then
  warn "'mempalace' not yet on PATH. Add ~/.local/bin to PATH to use the CLI."
  warn "  e.g.  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# ── Initialize palace for this repo ──────────────────────────────────
# `mempalace init <dir>` is idempotent: it scans the directory, detects
# people/projects from file content, creates rooms from folder structure,
# and ensures ~/.mempalace/ exists. Re-running on the same dir merges the
# new scan into the existing palace (additive, no clobber).
#
# Storage lives at ~/.mempalace/ (per-user, cross-project). Multiple
# repos share one palace — wings come from people, rooms from folders.
# This is by design: agents remember context across all your projects.
log "initializing palace wing for $(basename "$REPO_ROOT") (writes to ~/.mempalace/)"
if command -v mempalace >/dev/null 2>&1; then
  mempalace init "$REPO_ROOT" || warn "'mempalace init' returned non-zero — see ~/.mempalace/ for partial state"
else
  # Fall back to module invocation if the entry-point script isn't on PATH.
  python3 -m mempalace init "$REPO_ROOT" || warn "'python3 -m mempalace init' returned non-zero"
fi

# ── MCP registration (optional) ──────────────────────────────────────
# MemPalace ships a Claude Code plugin marketplace entry that registers
# the MCP server with all 9 tools (status, list_wings, list_rooms,
# get_taxonomy, search, check_duplicate, add_drawer, delete_drawer,
# reconnect). We don't auto-install — that's a per-user choice (palace
# is shared across projects, plugin install is a global Claude config
# change). Surface the command instead.
if command -v claude >/dev/null 2>&1; then
  log "Claude Code detected. To wire the MCP server, run:"
  log "  claude plugin marketplace add MemPalace/mempalace"
  log "  claude plugin install --scope user mempalace"
  log "  # then restart Claude Code and run /skills to verify"
  log ""
  log "Or for manual MCP registration without the marketplace:"
  log "  claude mcp add mempalace -- python3 -m mempalace.mcp_server"
fi

# ── Done ─────────────────────────────────────────────────────────────
log "MemPalace install complete."
log ""
log "  Palace storage:  ~/.mempalace/  (per-user, NOT in this repo)"
log "  Mining (opt-in): mempalace mine \"$REPO_ROOT\""
log "  Search:          mempalace search \"<your query>\""
log ""
log "Mining is OFF by default. To index this repo's source/docs into the"
log "palace (one-time, ~minutes for a large workspace):"
log "  mempalace mine \"$REPO_ROOT\""
log ""
log "Conversation exports (Claude/ChatGPT/Slack) are mined separately:"
log "  mempalace mine ~/chats/ --mode convos --extract general"
