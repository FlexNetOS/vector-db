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

1. `mcp-brain-server` HTTP API. Default bind is `0.0.0.0:8080` (override
   with the `PORT` env var, see `crates/mcp-brain-server/src/main.rs`).
   Use `http://localhost:8080` in dev; the public deployment is
   `https://pi.ruv.io`. `/v1/status` is unauthenticated; `/v1/memories*`
   require `Authorization: Bearer $BRAIN_API_KEY`.
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
# Default bind is 0.0.0.0:8080. Override with PORT=<n> if you need it.
PORT=8080 cargo run -p mcp-brain-server &
curl -fsS http://localhost:8080/v1/status | jq
```

`/v1/status` is unauthenticated. Expected:
`{ "ok": true, "memories": <n>, "graph_edges": <n> }`.

### 2. MCP brain tools register

```bash
# from a weftos repo
cd /path/to/weftos
weaver mcp list 2>&1 | grep -E '(pi-brain|ruvector)'
```

### 3. Round-trip a memory share + search

Both `/v1/memories` (POST) and `/v1/memories/search` require an
`Authorization: Bearer $BRAIN_API_KEY` header (extractor:
`AuthenticatedContributor` in `crates/mcp-brain-server/src/auth.rs`).
Missing or malformed header → 401. The API key must be ≥ 8 chars (`MIN_API_KEY_LEN` in `crates/mcp-brain-server/src/auth.rs:53`); for a
local dev run, export your own value (e.g. `export BRAIN_API_KEY=dev-$(openssl
rand -hex 16)`) and re-launch the server with `BRAIN_SYSTEM_KEY=$BRAIN_API_KEY`
so the constant-time check accepts it.

```bash
export BRAIN_API_KEY=...   # at least 8 chars; never commit this

# share
curl -X POST http://localhost:8080/v1/memories \
  -H "Authorization: Bearer $BRAIN_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"category":"convention","title":"weftos uses scripts/build.sh","content":"Always use scripts/build.sh, never raw cargo.","tags":["weftos","build"]}'

# search
curl -fsS "http://localhost:8080/v1/memories/search?q=scripts/build.sh&limit=3" \
  -H "Authorization: Bearer $BRAIN_API_KEY" \
  | jq '.[].title'
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
| `curl /v1/memories` or `brain_search` returns 401 | Missing/short `Authorization: Bearer ...` header | Set `BRAIN_API_KEY` (≥ 8 chars per `MIN_API_KEY_LEN`); for `pi.ruv.io`, retrieve via `gcloud secrets versions access latest --secret=BRAIN_API_KEY`. For local dev, also set `BRAIN_SYSTEM_KEY` to the **same value and same byte length** as `BRAIN_API_KEY` when launching `mcp-brain-server` — `subtle::ConstantTimeEq::ct_eq` short-circuits on length mismatch and would otherwise leak length via timing |
| `curl http://localhost:7333` connection refused | Port mismatch | Brain server defaults to **8080**, not 7333. Either `curl :8080` or relaunch with `PORT=7333 cargo run -p mcp-brain-server` |
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
