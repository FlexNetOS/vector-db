# GitNexus index — local code-graph cache

This directory holds the [GitNexus](https://github.com/abhigyanpatwari/GitNexus)
knowledge graph for **ruvector**. The graph is what gives MCP-aware
agents (Claude Code, Codex, Cursor, OpenCode) structural awareness of
this 150+ crate workspace — every dependency, call chain, and cluster
the indexer extracts.

Everything in this directory is **gitignored** except this README and
the `.gitignore` itself. The graph is regenerable in seconds via the
bootstrapper.

## Install / refresh

```bash
scripts/install-gitnexus.sh
```

The script runs `npx -y gitnexus@latest analyze --skip-agents-md`
(staleness-aware; only re-walks changed files unless
`GITNEXUS_FORCE=1`) and then `npx -y gitnexus@latest setup` to
register the MCP server in any installed agent runtime's config.

`--skip-agents-md` is **important**: GitNexus would otherwise rewrite
`CLAUDE.md` / `AGENTS.md` and clobber the hand-curated rules at the
top of `CLAUDE.md` (workspace exclusion rules, brain integration,
swarm config, etc.). We keep our context files authoritative.

## Verify the install

```bash
npx -y gitnexus@latest status
npx -y gitnexus@latest list
```

`status` confirms staleness state for the current repo; `list` shows
all repos GitNexus has indexed system-wide (registry lives at
`~/.gitnexus/registry.json`).

From within an MCP-aware agent, ask it to call the GitNexus `context`
tool with a known symbol name (e.g. a public function in
`crates/mcp-brain-server/src/lib.rs`) — if it returns categorized
references and process participation, the wiring is live.

## What lives here

GitNexus writes a single LadybugDB file plus auxiliary indexes (BM25,
HNSW for embeddings if `--embeddings` was passed, and a process
trace cache). Concrete contents depend on the upstream version, so
read the [GitNexus README](https://github.com/abhigyanpatwari/GitNexus/blob/main/gitnexus/README.md)
for the authoritative layout.

Notable sizes you might see:

- **Tens of MB** for the graph DB on a workspace this size (no
  embeddings).
- **Hundreds of MB** if you run `gitnexus analyze --embeddings` to
  enable hybrid search.

Nuking the cache (`rm -rf .gitnexus/*` minus the gitignored entries
this README excludes) is safe — the bootstrapper rebuilds from
scratch.

## MCP tools the agent gets

Once the MCP server is registered, the agent has these tools
available without any extra prompting (see the upstream README for
full schemas):

| Tool | Purpose |
|------|---------|
| `list_repos` | Discover all indexed repositories |
| `query` | Process-grouped hybrid search (BM25 + semantic + RRF) |
| `context` | 360° symbol view — categorized refs, process participation |
| `impact` | Blast-radius analysis with depth grouping + confidence |
| `detect_changes` | Git-diff impact — maps changed lines to affected processes |
| `rename` | Multi-file coordinated rename via graph + text search |
| `cypher` | Raw Cypher graph queries |

## Where this fits in the roadmap

GitNexus is **Phase 4** of the cross-repo self-learning roadmap (see
`/home/ubuntu/devin-work/integration-roadmap.md` and
`.claude/skills/cross-repo-integration/SKILL.md`):

- **Phase 1**: Understand-Anything (semantic dashboards)
- **Phase 2**: cross-repo-integration + self-learning-loop SKILLs
- **Phase 3**: Attractor pipeline (Identify → Implement → Validate → Optimize → Distill)
- **Phase 4**: GitNexus (this) — structural awareness layer
- **Phase 5**: MemPalace — long-term memory
- **Phase 6**: GitHub Actions self-learning workflow
- **Phase 7**: cross-repo orchestrator

In the Attractor loop, GitNexus is what the **identify** node calls
to figure out the *real* surface area of a proposed change before
the implement node touches a single line — `impact` + `detect_changes`
let us answer "what depends on this" without grepping by hand.

## Licensing

GitNexus ships under
[PolyForm Noncommercial 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0/).
This bootstrapper only invokes the upstream CLI via `npx`; no
GitNexus code is vendored into ruvector. If you need to use GitNexus
in a commercial setting, talk to akonlabs.com about their commercial
license.
