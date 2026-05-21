---
name: understand-anything-usage
description: Use Understand-Anything (LLM + static analysis dashboard generator) when starting on an unfamiliar area of ruvector, onboarding to a crate, or producing an architecture map for a refactor. Provides 8 slash-commands (/understand, /understand-onboard, /understand-explain, /understand-diff, /understand-chat, /understand-domain, /understand-knowledge, /understand-dashboard) that emit structured artifacts under `.understand-anything/`. Skill is plugin-style — runs through Claude Code, Codex, Gemini, OpenCode, or pi-coder.
---

# Understand-Anything Usage — RuVector

## What it is

Upstream: https://github.com/Lum1104/Understand-Anything

A pnpm-workspace tool that combines tree-sitter static analysis with LLM
prompts to produce interactive code-comprehension dashboards. Plugin-style
distribution — slash-commands resolve from `~/.agents/skills/` and point
into a single clone at `~/.{platform}/understand-anything`.

## When to use it

Reach for an `/understand-*` command when you're about to:

| Situation | Command | What you get |
|---|---|---|
| Onboarding to a new crate | `/understand-onboard <crate>` | Guided walkthrough + key entry points |
| "What does this codebase do?" | `/understand` (full pass) | Knowledge graph + dashboard JSON |
| "Why does X work this way?" | `/understand-explain <symbol>` | Explanation grounded in the static graph |
| "What changed in this PR?" | `/understand-diff <ref>` | Diff-aware impact analysis |
| Domain-specific question | `/understand-domain <topic>` | Filtered view of the graph |
| Q&A over the codebase | `/understand-chat` | Persistent chat with graph context |
| Pre-existing graph already? | `/understand-dashboard` | Render the dashboard from cached JSON |
| Cross-link with knowledge base | `/understand-knowledge` | Bridges code graph ↔ memory store |

## Install

```bash
scripts/install-understand-anything.sh
```

The script is idempotent. It:
1. Detects which agent runtimes are on PATH (`claude`, `codex`, `gemini`,
   `opencode`, `pi-coder`, `openclaw`).
2. Picks one as the canonical clone (under `~/.<runtime>/understand-anything/`)
   and symlinks the others to it — single source of truth.
3. Refuses to silently overwrite local edits or diverged commits in the
   clone (uses `merge --ff-only`, fails loudly if the working tree is
   dirty). Remove the directory and re-run for a clean upstream sync.

If `claude` is on PATH the script also surfaces the marketplace install
(`/plugin install understand-anything@understand-anything`) which is the
recommended path for Claude Code users.

## Output location

| Path | Contents | Status |
|---|---|---|
| `.understand-anything/knowledge-graph.json` | Per-repo knowledge graph | **Tracked.** The canonical artifact — re-`/understand` only when source has materially shifted. |
| `.understand-anything/onboarding.md`, `tours/` | `/understand-onboard` + dashboard tours output | **Tracked.** Curated, hand-edited later. |
| `.understand-anything/intermediate/`, `diff-overlay.json`, `file-content.cache.json` | Per-agent scratch / dashboard cache | **Gitignored.** Transient. |

Selective tracking is enforced by `.understand-anything/.gitignore`
(ignore-everything pattern with explicit `!knowledge-graph.json`,
`!onboarding.md`, `!tours/` allow-listings). Output stays inside this
repo; it does NOT pollute upstream's directory.

## Versioning

Upstream pins `pnpm` and `node` via `packageManager` field — `node>=22`
and `pnpm>=10` are required to run the dashboard. The bootstrap script
covers the runtime side; the dashboard itself is `pnpm dev:dashboard`
inside the clone if you want to launch it.

## Cross-repo

The matching skill in **weftos** is at
`.claude/skills/understand-anything-usage/SKILL.md`. The bootstrap is
the same; only the workspace being analyzed differs.
