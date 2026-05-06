---
name: cross-repo-integration
description: Map and execute the FlexNetOS/ruvector ↔ FlexNetOS/weftos integration topology. Use when a task spans both repos — e.g. exposing a brain capability from ruvector to a weft runtime, validating that a ruvector API change is reflected in weftos clients, or routing a weftos agent through pi.ruv.io. ruvector is the BRAIN; weftos is the RUNTIME.
---

# Cross-Repo Integration — RuVector (brain) ↔ WeftOS (runtime)

## Topology

```
                ┌──────────────────────────────┐
                │         WeftOS               │
                │  (FlexNetOS/weftos, master)  │
                │                              │
                │  weft daemon  ◄── kernel  ──►│
                │  weaver CLI       services   │
                │  clawft-* crates             │
                └──────────────┬───────────────┘
                               │ JSON-RPC / HTTP / MCP
                               ▼
                ┌──────────────────────────────┐
                │        RuVector              │
                │  (FlexNetOS/ruvector, main)  │
                │                              │
                │  mcp-brain-server  ◄── REST  │
                │  AgentDB + HNSW              │
                │  ReasoningBank (sona)        │
                │  prime-radiant (witness)     │
                └──────────────────────────────┘
```

This skill lives in **ruvector** (the brain). Read the matching skill in
**weftos** at `.claude/skills/cross-repo-integration/SKILL.md` for the
runtime side.

## What ruvector exposes

| Capability | Crate | Wire format |
|---|---|---|
| Shared brain (memories, search) | `crates/mcp-brain-server` | REST + MCP (`brain_*`) |
| Vector store + HNSW search | `crates/ruvector-core`, `npm/core` | NAPI / WASM |
| Reasoning patterns (Q-learning, Decision Transformer) | `crates/sona`, `crates/prime-radiant` | in-process Rust |
| Agent trajectories + verdicts | `ReasoningBank` (re-exported by `sona`) | in-process Rust |
| MCP tooling | `crates/mcp-brain`, `crates/mcp-gate` | MCP over stdio / SSE |
| Code-intelligence ingestion | `crates/ruvector-cnn` | local embedding pipeline |

**Public surfaces a weft runtime can call:**

1. `mcp-brain-server` HTTP API (default `http://localhost:7333` in dev,
   or the deployed `pi.ruv.io` instance).
2. The MCP server registered as `pi-brain` in
   `.claude/skills/.../.mcp.json` style configs — methods `brain_status`,
   `brain_search`, `brain_share`, `brain_list`, `brain_drift`,
   `brain_partition`.
3. The `@ruvector/node` and `@ruvector/wasm` npm packages for in-process
   vector search inside a weftos extension or browser bundle.

## What weftos consumes

A `weft` daemon that wants to use ruvector as its memory/reasoning
substrate currently has these integration points:

- `crates/clawft-llm` — provider abstraction. Add a `RuvectorBrainProvider`
  that wraps the `pi-brain` MCP for retrieval-augmented generation.
- `crates/clawft-kernel/src/embedding{,_onnx}.rs` — the kernel's local
  embedder. Optionally route to ruvector's HNSW for cross-process recall.
- `crates/clawft-weave` (`weaver`) — orchestration commands that should
  emit traces consumable by ruvector's `ReasoningBank`.

## Integration verification checklist

Run these after any change that touches the ruvector ↔ weftos contract.

### 1. Brain server boots and serves status

```bash
cd /path/to/ruvector
cargo run -p mcp-brain-server &  # default port 7333
curl -fsS http://localhost:7333/v1/status | jq
```

Expected: `{ "ok": true, "memories": <n>, "graph_edges": <n> }`.

### 2. MCP brain tools register

```bash
# from a weftos repo
cd /path/to/weftos
weaver mcp list 2>&1 | grep -E '(pi-brain|ruvector)'
```

### 3. Round-trip a memory share + search

```bash
# share
curl -X POST http://localhost:7333/v1/memories \
  -H 'Content-Type: application/json' \
  -d '{"category":"convention","title":"weftos uses scripts/build.sh","content":"Always use scripts/build.sh, never raw cargo.","tags":["weftos","build"]}'

# search
curl -fsS "http://localhost:7333/v1/memories/search?q=scripts/build.sh&limit=3" | jq '.[].title'
```

Expected: the just-shared title appears in the result.

### 4. AgentDB HNSW round-trip from a weft consumer

In a weftos workspace:

```bash
node --eval '
  import("@ruvector/node").then(({ RuVectorIndex }) => {
    const idx = new RuVectorIndex({ dim: 16, metric: "cosine" });
    idx.upsert("a", new Float32Array(16).fill(0.1));
    idx.upsert("b", new Float32Array(16).fill(0.2));
    console.log(idx.search(new Float32Array(16).fill(0.15), 1));
  });
'
```

### 5. Cross-repo schema drift check

If `crates/mcp-brain-server`'s response schema changes, the weftos side
**must** be updated in the same release window. To detect drift:

```bash
# in ruvector
cargo run -p mcp-brain-server -- --print-openapi > /tmp/ruv-brain.openapi.json

# in weftos
diff <(weaver brain dump-schema) /tmp/ruv-brain.openapi.json
```

Add this diff to weftos's `scripts/build.sh gate` once the dump-schema
subcommand exists.

## Versioning policy

- **ruvector** uses semver per crate. Breaking changes to
  `mcp-brain-server` HTTP routes bump the `mcp-brain-server` crate's
  major version.
- **weftos** is **lockstep semver** (ADR-001) — the entire workspace
  shares one version. A weftos release pins a specific ruvector
  `mcp-brain-server` major.
- The pin lives in `crates/clawft-llm/Cargo.toml` (when the brain client
  is implemented there) or in the npm dependency on `@ruvector/node`.

## Common failure modes

| Symptom | Likely cause | Fix |
|---|---|---|
| `weaver mcp list` shows no `pi-brain` | MCP not registered | Register in the agent runtime's MCP config; see `~/.codex/mcp.json` or `.cursor-mcp.json` |
| `brain_search` returns 401 | API key not in env | Set `BRAIN_API_KEY`; for `pi.ruv.io`, see `gcloud secrets versions access latest --secret=ANTHROPIC_API_KEY` style retrieval |
| AgentDB HNSW query stack-overflows in CI | Missing `RUST_MIN_STACK=16777216` | Run cargo from the repo root so `.cargo/config.toml` is honored |
| weftos wasm bundle exceeds 300KB after adding ruvector dep | dep not gated under `native` feature | Make the dep `optional = true`, gate it behind `native`, add a wasm shim |

## Where to look first

- `crates/mcp-brain-server/src/pipeline.rs` — request lifecycle.
- `crates/sona/src/reasoning_bank.rs` — pattern store contract.
- `crates/prime-radiant/src/execution/gate.rs` — witness chain anchor.
- `crates/mcp-brain/src/lib.rs` — MCP client used by weftos consumers.

## Related skills

- `agentdb-vector-search` — semantic search via AgentDB.
- `agentdb-learning` — RL training loops.
- `reasoningbank-intelligence` — pattern recognition + meta-cognition.
- `self-learning-loop` (this repo) — Implement → Validate → Optimize → Distill cycle.

## Forbidden actions

- Do **not** edit generated NAPI binaries under `npm/packages/<name>-<os>-<arch>/`.
- Do **not** bump `hnsw_rs` from crates.io — it is patched in-tree
  (`patches/hnsw_rs/`) for WASM compatibility.
- Do **not** include `ruvector-postgres` in workspace builds — it
  requires pgrx and is intentionally `[workspace.exclude]`.
- Do **not** check in `BRAIN_API_KEY` or any pi.ruv.io credentials.
