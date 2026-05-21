---
name: gitnexus-usage
description: Use the GitNexus code-graph MCP server to query, impact-analyze, and refactor across the ruvector workspace (~150 crates). Reach for this skill BEFORE doing wide refactors, large grep sweeps, or "where is X called from" questions — the graph is faster and more accurate than reading files. Lives at .gitnexus/ (gitignored). MCP tools: query, context, impact, detect_changes, rename, cypher.
---

# GitNexus Usage — RuVector

## What it is

GitNexus is a code-intelligence MCP server. The CLI walks the repo, builds a
LadybugDB knowledge graph in `.gitnexus/`, and exposes it to any agent
runtime via MCP. It understands Rust, TypeScript, Python, and a few other
languages and tracks symbols, call edges, type relationships, and impact
graphs — all on disk, no external service.

## When to use it (vs. grep / ripgrep / reading files)

Reach for GitNexus first when you're about to do any of these:

| Task | GitNexus tool | Why it beats grep |
|---|---|---|
| "Where is `fn bar` called from?" | `impact` | Resolves through traits, generics, re-exports |
| "Show me everything in module X" | `context` | Returns the structural neighborhood, not flat text |
| "Find all impls of trait T" | `query` | Type-aware; grep finds the string `impl T`, GitNexus knows what `T` resolves to |
| "What's affected if I rename `Foo`?" | `rename` (dry-run) | Walks every call/use site at the AST level |
| "What changed since last index?" | `detect_changes` | Uses git HEAD diff against the indexed snapshot |
| Custom traversal | `cypher` | Full Cypher-style query against the graph |

## Install

```bash
scripts/install-gitnexus.sh
```

This is idempotent. It:
1. Verifies Node 20+ is on PATH (GitNexus engine requirement).
2. Runs `npx -y gitnexus@${GITNEXUS_VERSION:-latest} analyze --skip-agents-md`
   to (re)index. `--skip-agents-md` is **required** — without it GitNexus
   rewrites CLAUDE.md and clobbers the hand-curated rules.
3. Runs `npx -y gitnexus@${GITNEXUS_VERSION:-latest} setup` to register
   the MCP server with whatever editor configs exist on disk
   (`~/.cursor/mcp.json`, `~/.config/opencode/config.json`, etc.).

## Index location

| Path | Contents | Status |
|---|---|---|
| `.gitnexus/` | LadybugDB graph DB, embedding cache | **Gitignored.** Regenerable by re-running `analyze`. |
| `~/.gitnexus/registry.json` | Per-machine registry of all indexed repos | Per-machine, never committed. |
| `.claude/skills/gitnexus/` | 6 helper SKILLs auto-installed by `analyze` | **Gitignored** — gitnexus 1.x always writes here, even with `--skip-agents-md`, so we keep them out of source control. |

Re-index manually if HEAD has moved a lot:

```bash
npx -y gitnexus@latest analyze --skip-agents-md            # incremental
GITNEXUS_FORCE=1 scripts/install-gitnexus.sh               # full re-walk
```

## Smoke test

```bash
npx -y gitnexus@latest status     # confirms graph exists + last index time
npx -y gitnexus@latest list       # lists all indexed repos in your registry
```

If MCP integration is wired up, an agent can verify by calling the
`gitnexus.context` tool with any symbol name (e.g. `BrainServer`,
`AgentDB`, `HnswIndex`).

## Known gotchas

1. **`.claude/skills/gitnexus/` is auto-clobbered.** Even with
   `--skip-agents-md`, gitnexus 1.x always re-installs 6 SKILL files
   there (verified in upstream `dist/cli/ai-context.js` lines 177-241).
   The repo's top-level `.gitignore` keeps them untracked. Don't try to
   commit anything under that path — it'll be overwritten on the next
   `analyze`. Hand-curated SKILLs go elsewhere (`.claude/skills/<other>`).
2. **Indexing scope is `.gitignore` + `.gitnexusignore`, NOT Cargo
   workspace membership.** A crate that's `exclude`d from the workspace
   in `Cargo.toml` will still be walked unless it's also gitignored or
   listed in `.gitnexusignore`. (Currently irrelevant for ruvector since
   no crate is workspace-excluded, but worth knowing.)
3. **License**: PolyForm Noncommercial 1.0.0. We invoke the upstream
   CLI; nothing is vendored. Commercial licenses via akonlabs.com.

## Cross-repo

The matching skill in **weftos** is at
`.claude/skills/gitnexus-usage/SKILL.md`. WeftOS additionally ships a
`.gitnexusignore` to exclude `gui/src-tauri/` (the Tauri shell that's
out of the cargo workspace).
