#!/usr/bin/env bash
# Install Understand-Anything (https://github.com/Lum1104/Understand-Anything)
# at the user/agent level so any coding agent that opens this repo can run
# /understand, /understand-onboard, /understand-knowledge, etc.
#
# The upstream tool is plugin-style: skills live under ~/.agents/skills/ and
# point into a single clone at ~/.{platform}/understand-anything. This script
# implements the same install flow as the upstream INSTALL.md files for each
# agent runtime, choosing whichever runtimes are present on this machine.
#
# Idempotent: safe to re-run. Re-runs `git fetch` and fast-forwards the
# existing clone; refuses to silently overwrite local edits/commits.
# Cross-platform: supports Linux, macOS. PowerShell users should follow the
# upstream Windows instructions in .codex/INSTALL.md.

set -euo pipefail

UA_REPO="https://github.com/Lum1104/Understand-Anything.git"
UA_BRANCH="${UA_BRANCH:-main}"

# Skills exposed by the plugin. Kept in sync with the upstream
# `understand-anything-plugin/skills/` directory; verify by running
# `ls $PRIMARY_DIR/understand-anything-plugin/skills/` after install.
UA_SKILLS=(
  understand
  understand-chat
  understand-dashboard
  understand-diff
  understand-domain
  understand-explain
  understand-knowledge
  understand-onboard
)

log()  { printf '\033[1;34m[understand-anything]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[understand-anything]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[1;31m[understand-anything]\033[0m %s\n' "$*" >&2; exit 1; }

# Detect which agent runtimes are present. Each entry maps runtime -> clone dir.
declare -a RUNTIMES=()
add_runtime() {
  local name="$1" clone_dir="$2"
  if [[ ! " ${RUNTIMES[*]:-} " == *" $name:"* ]]; then
    RUNTIMES+=("$name:$clone_dir")
  fi
}

# Claude Code: install via the marketplace if `claude` is on PATH, otherwise
# fall back to the symlink approach so the skills still resolve.
if command -v claude >/dev/null 2>&1; then
  log "claude CLI detected — recommended install path is the marketplace:"
  log "  /plugin marketplace add Lum1104/Understand-Anything"
  log "  /plugin install understand-anything@understand-anything"
fi

command -v codex      >/dev/null 2>&1 && add_runtime codex      "$HOME/.codex/understand-anything"
command -v gemini     >/dev/null 2>&1 && add_runtime gemini     "$HOME/.gemini/understand-anything"
command -v opencode   >/dev/null 2>&1 && add_runtime opencode   "$HOME/.opencode/understand-anything"
command -v pi-coder   >/dev/null 2>&1 && add_runtime pi         "$HOME/.pi/understand-anything"
command -v openclaw   >/dev/null 2>&1 && add_runtime openclaw   "$HOME/.openclaw/understand-anything"

# If no specific runtime is detected we still install once under ~/.codex/
# (the canonical clone location) so the skills are available to anything that
# reads ~/.agents/skills/.
if [[ ${#RUNTIMES[@]} -eq 0 ]]; then
  log "no agent runtime detected on PATH; installing canonical skills under ~/.codex/"
  add_runtime codex "$HOME/.codex/understand-anything"
fi

# Clone-or-pull a runtime's copy. Multiple runtimes share skills via
# ~/.agents/skills/, so we only need a single source of truth — pick the first
# entry as the canonical clone and symlink the rest.
PRIMARY_RUNTIME="${RUNTIMES[0]}"
PRIMARY_DIR="${PRIMARY_RUNTIME#*:}"

mkdir -p "$(dirname "$PRIMARY_DIR")"

if [[ -d "$PRIMARY_DIR/.git" ]]; then
  log "updating existing clone at $PRIMARY_DIR"
  git -C "$PRIMARY_DIR" fetch --quiet origin "$UA_BRANCH"
  # Refuse to touch the directory if working tree, index, or untracked
  # files are non-empty. `git status --porcelain` is the comprehensive
  # dirty check; `git diff --quiet HEAD` alone would miss staged-only
  # changes and untracked files, which is why we don't use it here.
  if [[ -n "$(git -C "$PRIMARY_DIR" status --porcelain)" ]]; then
    fail "$PRIMARY_DIR has uncommitted edits, staged changes, or untracked files; commit/stash/clean them, or remove the directory and re-run"
  fi
  # Try fast-forward. If this fails, the user has local commits that
  # diverged from origin/$UA_BRANCH (or upstream rebased). Either way we
  # refuse to `reset --hard` because that would silently destroy local
  # commits the user may want to keep. The user can remove the directory
  # and re-run for a clean upstream sync.
  if ! git -C "$PRIMARY_DIR" merge --ff-only --quiet "origin/$UA_BRANCH"; then
    fail "$PRIMARY_DIR cannot fast-forward to origin/$UA_BRANCH (local commits diverged or upstream rebased); resolve manually or remove the directory and re-run"
  fi
else
  log "cloning Understand-Anything to $PRIMARY_DIR"
  git clone --quiet --branch "$UA_BRANCH" "$UA_REPO" "$PRIMARY_DIR"
fi

# Mirror to other detected runtimes via directory symlinks.
for rt in "${RUNTIMES[@]:1}"; do
  rt_name="${rt%%:*}"
  rt_dir="${rt#*:}"
  mkdir -p "$(dirname "$rt_dir")"
  if [[ -L "$rt_dir" || -e "$rt_dir" ]]; then
    log "$rt_name: $rt_dir already exists, leaving in place"
  else
    log "$rt_name: linking $rt_dir -> $PRIMARY_DIR"
    ln -s "$PRIMARY_DIR" "$rt_dir"
  fi
done

# Wire the skills into ~/.agents/skills/ (used by Codex / Gemini CLI / Pi /
# OpenCode / OpenClaw). Idempotent.
mkdir -p "$HOME/.agents/skills"
for skill in "${UA_SKILLS[@]}"; do
  ln -sfn "$PRIMARY_DIR/understand-anything-plugin/skills/$skill" \
          "$HOME/.agents/skills/$skill"
done

# Universal plugin root symlink — required by /understand-dashboard.
if [[ ! -e "$HOME/.understand-anything-plugin" && ! -L "$HOME/.understand-anything-plugin" ]]; then
  ln -s "$PRIMARY_DIR/understand-anything-plugin" "$HOME/.understand-anything-plugin"
fi

log "installed runtimes: ${RUNTIMES[*]}"
log "skills available at: $HOME/.agents/skills/{$(IFS=,; echo "${UA_SKILLS[*]}")}"
log "to generate the knowledge graph for this repo, run /understand from your"
log "agent of choice (Claude Code, Codex, Gemini CLI, OpenCode, Pi Agent)."
