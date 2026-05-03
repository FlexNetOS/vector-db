# ADR-163: FlexNetOS Unification & Cognitum Integration

- **Status:** Proposed (planning only — no execution this PR)
- **Date:** 2026-04-29
- **Deciders:** FlexNetOS team
- **Companion:** [`weftos/docs/adr/adr-048-flexnetos-unification-cognitum-integration.md`](https://github.com/FlexNetOS/weftos/blob/master/docs/adr/adr-048-flexnetos-unification-cognitum-integration.md) (identical content, mirrored)
- **Related:** ruvector ADR-001..ADR-162; weftos ADR-001..ADR-047
- **PR scope:** This commit installs the Cognitum SDK (`cognitum-one` Rust crate, `@cognitum-one/sdk` Node package) at workspace level, registers the Cognitum cloud + Seed MCP servers in `.mcp.json`, and lays out the unified-flow plan below. It does NOT execute the repo merge.

## Context

`FlexNetOS/ruvector` (128 crates, vector-centric, SONA / ReasoningBank / Witness Chain / RVF / mcp-brain-server) and `FlexNetOS/weftos` (~36 crates, kernel + agent framework, ECC cognitive substrate / EML / ExoChain / SurfaceTree) are complementary but currently independent codebases. The product direction calls for a single, fully auto-agentic flow that:

1. Uses ruvector as the **vector / reasoning substrate** (HNSW, RaBitQ, RVF, ReasoningBank, SONA, sheaf-laplacian coherence).
2. Uses weftos as the **runtime / orchestration substrate** (kernel daemon, K0–K6 phase model, ECC ticks, agent supervisor, mesh, ExoChain audit log, Substrate state tree, SurfaceTree UI).
3. Integrates with **Cognitum** (cognitum.one) edge appliances and cloud control plane as a **first-class peer** — not just an external service. Cognitum exposes 98 local REST endpoints, 12 cloud MCP tools, and 114 device MCP tools across an aligned domain (vector store, witness chain, sensor streams, mesh, OTA, custody).

## Decision

Merge `ruvector` and `weftos` into a single Cargo workspace under a new repo `FlexNetOS/flexnetos` (or in-place if we keep the larger of the two). Cognitum is the **canonical hardware/edge target** and the SDK is a workspace-level dependency available to any crate that needs it.

### High-level architecture (post-merge)

```
                              ┌─────────────────────────────────────────┐
                              │              UI Surfaces                │
                              │  egui (native + wasm)  · SurfaceTree IR │
                              └────────────────┬────────────────────────┘
                                               │
                              ┌────────────────▼────────────────────────┐
                              │       weftos K0–K6 Kernel Daemon        │
                              │  (weft binary + JSON-RPC over Unix sock)│
                              ├────────────────┬────────────────────────┤
                              │ K0 Boot/Lifecycle  K1 Process/Supervise │
                              │ K2 IPC/A2A        K3 WASM/Plugins       │
                              │ K3c ECC Substrate K5 App Framework      │
                              │ K6 Mesh           ExoChain audit        │
                              └────────────────┬────────────────────────┘
                                               │
                ┌──────────────────────────────┼──────────────────────────────┐
                │                              │                              │
        ┌───────▼─────────┐          ┌─────────▼──────────┐         ┌─────────▼─────────┐
        │  ruvector core  │          │  Cognitum adapter  │         │  Tools / agents   │
        │  • HNSW/DiskANN │          │  • SeedClient (LAN │         │  • rvAgent fleet  │
        │  • RaBitQ       │          │    /USB/mDNS)      │         │  • prime-radiant  │
        │  • RVF storage  │          │  • Cloud MCP SSE   │         │  • weaver loops   │
        │  • SONA / RB    │          │  • 98 REST + 114   │         │  • A2A router     │
        │  • mcp-brain    │          │    device MCP      │         │  • LLM router 11p │
        │  • witness chain│          │  • witness sync    │         │                   │
        └─────────────────┘          └────────────────────┘         └───────────────────┘
```

### Integration mapping

| ruvector concept | weftos concept | Cognitum concept | Unified resolution |
|---|---|---|---|
| HNSW + RaBitQ index | `hnsw_service` (K3c) | Seed `/store/query` & `/store/ingest` | One `VectorStore` trait; Seed and local HNSW are interchangeable backends. |
| ReasoningBank | (not present) | Seed `/cognitive/snapshot` | `ReasoningBank` becomes a kernel service of type `Core`; snapshots can be replicated to a Seed for offline/ambient inference. |
| Witness Chain (SHAKE-256) | ExoChain (BLAKE3 + Ed25519 + ML-DSA-65) | Seed `/witness/chain` | Pick **ExoChain** as primary (newer crypto, post-quantum). Witness Chain becomes a compatibility translator. Seed witness chains anchor into ExoChain via mesh sync. |
| SONA | EML (`exp(x) - ln(y)`) | Cognitive container (.rvf) | SONA continual-learning loop drives EML coordinate descent inside the K3c cognitive tick. RVF cognitive containers ship between Seeds via mesh. |
| RVF format | (consumes RVF for cognitive containers) | RVF on disk in Seed `/store/` | Single source of truth: `rvf-types`, `rvf-runtime`, `rvf-wire` from ruvector — referenced by weftos via workspace deps (already partially done in weftos with `weftos-rvf-{crypto,wire}`). |
| `mcp-brain-server` | (no equivalent; uses claude-flow MCP) | Cognitum cloud MCP (`/mcpSse`) | Both register as peer MCP servers on the agent runtime. The brain proxies to ReasoningBank; Cognitum proxies to cloud catalog/orders/witness. |
| 60+ claude-flow agents | `agents/weftos/<role>.md` (12 personas) | Cognitum doesn't define agents | Adopt weftos's role-specific persona model (kernel-architect, weaver, mesh-engineer, etc.). Map ruvector agent types onto roles: `coder` → general agent, `security-auditor` → governance-counsel, `performance-engineer` → ecc-analyst, etc. |
| `package.json` workspace + napi | (no top-level npm) | `@cognitum-one/sdk` (Node) | Keep ruvector's npm workspace as the JS bindings root. Cognitum SDK joins `dependencies`. |
| `scripts/build.sh` (weftos only) | mandatory build wrapper | n/a | Adopt as the unified build wrapper. Extend to cover ruvector's WASM and napi build paths. |

### Phasing (proposed)

| Phase | Goal | Duration | Risk |
|---|---|---|---|
| **M0 — preparation** (this PR) | Cognitum SDK at workspace level; MCP config in both repos; this ADR mirrored. | 1 day | Low |
| **M1 — cross-repo dep graph** | Add `weftos` as a `[workspace.dependencies]` reference in ruvector via path or git revision. Validate ruvector crates can depend on `weftos-rvf-{wire,crypto}`. Or vice versa. | 1 week | Medium (build-graph cycles) |
| **M2 — workspace merge** | New monorepo `FlexNetOS/flexnetos`. Top-level `crates/` retains ruvector's 128 crates and weftos's 36 crates, deduplicated where overlap exists (rvf, mcp). Single `Cargo.toml` workspace. Single `scripts/build.sh`. Two binaries: `weft` (daemon) and `ruvector-cli` (vector ops). Optional unified `flexnetos` orchestrator binary. | 3–4 weeks | High |
| **M3 — Cognitum substrate adapter** | New crate `crates/clawft-substrate-cognitum/` implementing the `OntologyAdapter` trait (ADR-017) backed by `cognitum-one::SeedClient`. Topics: `substrate/cognitum/seed/<endpoint>/<key>`. Pulls live `/status`, `/sensor/*`, `/store/*`, `/witness/*` into the substrate state tree. Cognitive tick predicts coherence via remote Seed inference. | 2 weeks | Medium |
| **M4 — Mesh peering** | Seed mesh (`/peers`, `/sync/delta`) and weftos mesh (K6) bridge: ExoChain segments synced between Seeds via mesh, peer-discovery unified (mDNS + Kad). | 3 weeks | Medium |
| **M5 — Agent topology** | Hierarchical-mesh swarm with 6–8 specialized agents (kernel-architect, weaver, mesh-engineer, governance-counsel, ecc-analyst, vector-engineer, cognitum-fleet-mgr, doc-weaver). RAFT consensus for kernel state; CRDT for ReasoningBank. | 2 weeks | Low |
| **M6 — Release** | cargo-dist multi-target for `weft`, `weaver`, `ruvector-cli`, `cog` (Cognitum CLI). npm publishes for `@flexnetos/sdk`. Single Homebrew tap `FlexNetOS/homebrew-tap`. | 1 week | Low |

### Cognitum SDK installation (this PR)

#### Rust (cognitum-one v0.2.1, vendored in-tree)

The SDK source has been **vendored** into both repos under `vendor/cognitum-one/` (MIT licensed; upstream `cognitum-one/sdks` @ `a9e1c073`, source tarball SHA-256 `0db3dccd…`). See `vendor/cognitum-one/VENDORED.md` for full provenance and the `scripts/vendor-cognitum-one.sh` helper for re-vendoring future versions.

Added to `[workspace.dependencies]` in both `ruvector/Cargo.toml` and `weftos/Cargo.toml` as a path dependency:

```toml
cognitum-one = { path = "vendor/cognitum-one", default-features = false, features = ["rustls", "seed"] }
```

Feature menu: `seed`, `mdns`, `stream`, `blocking`, `rustls`, `native-tls`. Crates opt in by listing `cognitum-one.workspace = true` in their own `Cargo.toml`. The vendored crate is **not** added to `[workspace.members]` — keeping it out of `cargo check --workspace` / `cargo clippy --workspace` runs preserves the host workspace's lint policy without rewriting upstream code.

Why vendor: (1) builds no longer require crates.io reachability, (2) the source is auditable in-tree, (3) we control the version bump cadence, (4) downstream consumers of the merged repo see a stable, self-contained workspace. Trade-off: re-vendoring on every upstream release is slightly more friction than a `cargo update`, mitigated by the helper script.

Upstream: `github.com/cognitum-one/sdks` (path `sdks/rust`) · Docs: `docs.rs/cognitum-one`.

#### Node (@cognitum-one/sdk v0.2.1)

Added to `ruvector/package.json` under `dependencies`. Node ≥ 18 required. Subpath exports: `/seed`, `/seed/discovery/mdns` (opt-in via `multicast-dns` peer dep).

```ts
import { SeedClient } from "@cognitum-one/sdk/seed";
const c = new SeedClient({ endpoints: "https://cognitum.local:8443", tls: { insecure: true } });
const s = await c.status();
```

#### Python (cognitum v0.0.1.dev2)

NOT installed in either repo (no Python workspace yet). Available via `pip install cognitum` if/when needed.

### MCP configuration (this PR)

Added to both `ruvector/.mcp.json` and `weftos/.mcp.json`:

| Server | Transport | URL | Tools | Auth |
|---|---|---|---|---|
| `claude-flow` | stdio | `npx @claude-flow/cli@latest mcp start` | (existing) | none |
| `cognitum` | SSE | `https://cognitum.one/mcpSse` | 7 cloud (catalog, orders, payment, witness, lead, trackEvent, status) | `X-API-Key: cog_*` for writes (env: `COGNITUM_API_KEY`) |
| `cognitum-seed` | SSE | `https://cognitum.local:8443/mcp` (USB fallback `http://169.254.42.1/mcp`) | 114 device tools | paired bearer token (env: `COGNITUM_SEED_TOKEN`); USB-gadget path is unauth, physical cable is trust anchor |

Cloud discovery endpoints:

```
https://api.cognitum.one/apiHealth        # health
https://api.cognitum.one/apiCatalog       # product catalog
https://api.cognitum.one/apiMcpTools      # tool list
https://api.cognitum.one/mcpSse           # MCP SSE entry (also cognitum.one/mcpSse)
https://cognitum.one/marketplace.json     # marketplace
```

Discovery returned `{"status":"healthy","version":"1.0.0", ...}` from `apiMcpTools` at the time of writing.

`autoStart: false` for both Cognitum entries — they remain dormant until an agent invokes a tool, to avoid SSE handshakes in CI sandboxes.

For ruvector specifically, `.mcp.json` was previously gitignored. The gitignore entry has been replaced with `.mcp.local.json` (per-developer overrides) so the project-level Cognitum config is now committed.

## Consequences

### Positive
- **Single source of vector-store truth.** ruvector's HNSW/RaBitQ/RVF stack supplies both the local kernel cognitive substrate (weftos K3c) and Cognitum Seed appliances (over the network).
- **Single audit log.** ExoChain (BLAKE3 + Ed25519 + ML-DSA-65) supersedes ruvector's SHAKE-256 chain. Post-quantum-ready by default (ADR-043).
- **Unified agent persona model.** Drop the 60+ claude-flow agent zoo in favor of weftos's 12 specialized roles — fewer agents, clearer responsibilities.
- **Cognitum is a first-class peer.** Seeds become substrate-layer participants, not external integrations.

### Negative / Risks
- **Workspace size.** 164 crates is large; cargo build times will dominate iteration. Mitigation: `release-wasm` profile per ADR; cargo-chef in CI; per-crate caching.
- **Crypto migration cost.** Replacing SHAKE-256 with BLAKE3 + dual-sig requires touching every Witness Chain consumer in ruvector (search `crates/prime-radiant/`).
- **Dual workspace exclusion gotchas.** ruvector excludes `ruvector-postgres`, `ruvix/*`, `rvf` sub-workspaces, and embedded examples; weftos has no exclusions today. Merging requires careful preservation of these excludes.
- **CLAUDE.md divergence.** ruvector's CLAUDE.md says use `npm run build`; weftos's says use `scripts/build.sh`. Unification rule: **`scripts/build.sh` everywhere**. Update CLAUDE.md.
- **Cognitum hardware dependency.** Local Seed MCP (`cognitum-seed`) requires physical hardware. Document graceful degradation when unreachable.
- **Lockstep semver.** weftos uses lockstep 0.x; ruvector publishes per-crate. Decision deferred to M2 — likely to adopt lockstep for the merged repo.

## Validation

- `cargo metadata --no-deps` parses cleanly on both repos with the new workspace dep. ✓
- `.mcp.json` validates as JSON on both repos. ✓
- Cognitum SDK versions confirmed via `crates.io` and `npmjs.com` API. ✓
- Cognitum cloud `apiMcpTools` health check returns `healthy`. ✓
- Local Seed MCP NOT validated — no hardware available in this environment.

## Next steps (post-PR)

1. Pick consolidation strategy (new repo `FlexNetOS/flexnetos` vs in-place merge into one of the existing repos).
2. M1 cross-dep graph experiment: have one ruvector crate consume `weftos-rvf-wire` over a path dep.
3. Build the `clawft-substrate-cognitum` adapter crate (M3).
4. Update CLAUDE.md in both repos to point at the unified `scripts/build.sh`.
