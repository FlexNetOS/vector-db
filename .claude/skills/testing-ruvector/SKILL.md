---
name: testing-ruvector
description: Test ruvector Rust crates and npm packages end-to-end. Use when verifying bug fixes, PR changes, or regression testing across the workspace.
---

# Testing ruvector

## Prerequisites

- Rust nightly toolchain (workspace MSRV 1.77, but nightly recommended)
- Node.js v22+
- Run all cargo commands from repo root so `.cargo/config.toml` is picked up (`RUST_MIN_STACK=16777216`)

## Devin Secrets Needed

None for local testing. CI runs on GitHub Actions with repo secrets.

## Quick Smoke Test

```bash
cd /path/to/ruvector
cargo check --workspace --exclude ruvector-postgres
```

If this stack-overflows, you're not in a directory where `.cargo/config.toml` is visible.

## Full Test Commands

```bash
# Workspace compilation (all ~1198 crates)
cargo check --workspace --exclude ruvector-postgres

# Core vector library (HNSW, distance, SIMD, quantization, advanced features)
cargo test -p ruvector-core           # ~437 tests

# Graph database (Cypher, SQL, SPARQL, storage)
cargo test -p rvlite                  # ~71 tests

# Clippy (correctness + suspicious only — per repo lint policy)
cargo clippy -p ruvector-core -p rvlite -p ruvector-sparse-inference -p ruvector-gnn -- -D clippy::correctness -D clippy::suspicious

# TypeScript packages (install deps first if needed)
cd npm/packages/ruvector && npm install --ignore-scripts --force && npx tsc --noEmit
```

## Key Testing Patterns

### HNSW Distance Metrics
- Existing tests use Cosine metric only (`test_hnsw_insert_and_search`, etc.)
- DotProduct metric returns **negative** values (`-dot`). Any `.max(0.0)` clamp destroys ranking.
- The `test_dot_product_distance` test in `distance.rs` verifies negative values are preserved.
- No existing HNSW test creates an index with `DistanceMetric::DotProduct` — this is a gap.

### Cypher DELETE
- No existing integration test for DELETE operations in `tests/cypher_integration_test.rs`.
- DELETE with multi-row MATCH can yield duplicate node IDs — the fix uses `HashSet` deduplication.
- Test gap: should add a test that creates nodes with relationships, then `DETACH DELETE` to verify no double-delete errors.

### SQL Vector Search
- `test_vector_search` in `sql/executor.rs` covers basic vector search.
- The `saturating_mul(20)` fix only matters on wasm32 (32-bit usize). On 64-bit, it's effectively a no-op safety net.

### TypeScript Packages
- `npm install` may fail with CPU architecture mismatch (arm64 optional deps on x64). Use `--force` flag.
- `npx tsc` (without specifying TypeScript) may install the wrong package (`tsc@2.0.4`). Always use `npx tsc` after `npm install` has placed TypeScript in `node_modules`.
- The `tsconfig.json` has a preexisting `moduleResolution=node10` deprecation warning — ignore it.

### mcp-brain-server
- Cannot be unit-tested easily — requires running Axum server with real memory data.
- `spawn_blocking` patterns and training cursor fixes are verified by compilation + code review.
- The crate pulls in `ruvector-solver` which may have preexisting clippy issues.

## CI Notes

- All CI checks are `[optional]` (continue-on-error: true) per repo lint policy.
- `ruvector-postgres` is workspace-excluded — PostgreSQL CI failures are infrastructure, not code.
- Clippy uses `-W warnings` (warn) in CI, not `-D warnings` (deny). Clippy hits don't block merge.
- `ubuntu-26.04` runner label is intentional (Ubuntu 26.04 LTS released April 2026).

## What's NOT Testable Locally

| Area | Why |
|------|-----|
| Training cursor advancement | Requires brain server with real memory/vote data |
| spawn_blocking for reclassify | Requires Axum server with 1500+ memories |
| Embedder mutex 503 | Requires poisoning mutex at runtime |
| rvAgent env sanitization | Requires running rvagent-cli with target commands |
| getBackendInfo RVF type | Requires native/rvf backend loaded at runtime |
| burst-scaling totalErrorRate | No test suite; verified by TS compilation |

## Common Pitfalls

1. **Don't use `RUSTC_WRAPPER`** — it can interfere with compilation. Set `RUSTC_WRAPPER=""` if needed.
2. **Don't run clippy on mcp-brain-server with `-D`** — it transitively compiles `ruvector-solver` which may have preexisting issues.
3. **Don't hand-edit NAPI-RS binary packages** (`npm/packages/*-darwin-*`, etc.) — these are CI-managed.
4. **Don't include `ruvector-postgres`** in workspace builds without pgrx toolchain.
