# Devin workspace guide — FlexNetOS/ruvector

This document tells Devin (and any other AI agent or new contributor) how to
build, test, lint, and format the ruvector workspace correctly. The CI
workflow at `.github/workflows/ci.yml` is the source of truth; the commands
below mirror it.

Bootstrap a fresh environment with [`./setup.sh`](./setup.sh). The setup
script installs GitHub CLI (`gh`), configures `origin` as
`https://github.com/FlexNetOS/ruvector.git`, configures `upstream` as
`https://github.com/ruvnet/RuVector.git`, and then leaves authentication to
`gh auth login` or `GH_TOKEN` so CI/Devin secrets stay explicit.

## Workspace shape

- ~150+ Rust crates under `crates/` (workspace `[members]` in the root
  `Cargo.toml`). Several crates are deliberately workspace-excluded — most
  importantly `ruvector-postgres`, plus a handful of WASM/embedded/nested
  workspaces. See `Cargo.toml` `[workspace]` `exclude = [...]`.
- npm packages under `npm/` (workspace root has `npm/core`,
  `npm/packages/<feature>`, `npm/wasm`). NAPI-RS native bindings and
  wasm-bindgen browser bundles live here.
- Minimum Rust version: **1.77**, edition **2021**.
- Patched `hnsw_rs` lives at `patches/hnsw_rs/` (WASM-compatible fork —
  resolves a `rand` / `getrandom 0.2 vs 0.3` conflict). Do not bypass the
  `[patch.crates-io]` entry in the root `Cargo.toml`.
- `.cargo/config.toml` sets `RUST_MIN_STACK = 16777216` so trait resolution
  in `ruvector-filter` (which carries `#![recursion_limit = "4096"]`) does
  not stack-overflow rustc. Always run cargo from the repo root (or a child
  directory) so this env is picked up.

## Canonical commands

### Build / type-check

```bash
cargo check --workspace --exclude ruvector-postgres
```

This is what CI runs (`.github/workflows/ci.yml` line 48). `ruvector-postgres`
**must always be excluded** from workspace builds because its pgrx build
script requires a separate toolchain bootstrap (see "pgrx extension" below).

### Test

CI uses `cargo-nextest` and shards the workspace by domain. Locally, prefer
running tests for a single crate at a time — the full workspace build hits
~90 minutes on a cold cache.

```bash
cargo nextest run --no-fail-fast -p <crate-name>
# Doctests are not run by nextest; do them separately when needed:
cargo test --doc -p <crate-name>
```

### Lint

```bash
cargo clippy --workspace --exclude ruvector-postgres --all-targets -- -W warnings
```

Note: `-W warnings` (warn), not `-D warnings` (deny). The clippy job in CI
is `continue-on-error: true` — clippy is **advisory**. Per-crate `[lints]`
deny `correctness` and `suspicious` only; pedantic / stylistic clippy lints
are explicitly allowed. Do not bundle pedantic clippy fixes into feature
PRs.

### Format

```bash
cargo fmt --all -- --check
```

The fmt CI job is also `continue-on-error: true`, but stay consistent —
run `cargo fmt --all` before committing.

## pgrx extension (`ruvector-postgres`)

Excluded from workspace builds because pgrx needs a one-time bootstrap:

```bash
cargo install cargo-pgrx --version 0.12.9 --locked
cargo pgrx init --pg17=$(which pg_config)
cargo build -p ruvector-postgres
```

Run those commands from the `crates/ruvector-postgres/` directory (or pass
`--manifest-path`).

## npm workspace

Top-level orchestration scripts live in `package.json`:

```bash
npm run build       # cargo build --workspace --release (host bins)
npm run build:node  # cd crates/ruvector-node && napi build --release
npm run build:wasm  # cd crates/ruvector-wasm && bash build.sh
npm run build:all   # everything
npm run cli         # cargo run -p ruvector-cli
npm run mcp         # cargo run -p ruvector-cli --bin ruvector-mcp
```

Per-platform NAPI binary packages under `npm/packages/<feature>-<os>-<arch>/`
are auto-managed by CI (`chore: Update NAPI-RS binaries for all platforms`).
**Do not hand-edit those `.node` files.**

## Pre-commit hook

The repo ships a pre-commit hook at `.githooks/pre-commit` that runs
`scripts/sync-lockfile.sh` to keep `package-lock.json` aligned with the npm
workspace state. Install it once after cloning:

```bash
ln -sf ../../.githooks/pre-commit .git/hooks/pre-commit
```

## Quick sanity check after clone

```bash
cd "$(git rev-parse --show-toplevel)"
echo "$RUST_MIN_STACK"      # → 16777216 (cargo wrapper picks this up)
cargo check --workspace --exclude ruvector-postgres
```

If `cargo check` stack-overflows, you are not running cargo from a directory
where `.cargo/config.toml` is visible.

## Codebase intelligence — Understand-Anything

This repo is wired into [Understand-Anything](https://github.com/Lum1104/Understand-Anything),
a multi-agent code-intelligence plugin. To install it for the agent runtime
on this machine (Claude Code, Codex, Gemini CLI, OpenCode, Pi Agent), run:

```bash
./scripts/install-understand-anything.sh
```

The script clones the upstream tool to `~/.codex/understand-anything` (or the
runtime-specific path), wires `~/.agents/skills/understand*` symlinks, and is
idempotent.

Once installed, run any of the following from inside this repo:

| Command | Purpose | Output |
|---|---|---|
| `/understand` | Full multi-agent analysis (Claude Code style) | `.understand-anything/knowledge-graph.json` |
| `/understand-onboard` | Generate an onboarding tour | `.understand-anything/onboarding.md` |
| `/understand-knowledge` | Build/refresh the knowledge graph only | `.understand-anything/knowledge-graph.json` |
| `/understand-domain` | Domain-focused deep dive | merged into the graph |
| `/understand-diff` | Change-impact analysis on the working tree | `.understand-anything/diff-overlay.json` |
| `/understand-dashboard` | Launch the interactive dashboard | local web UI |

`.understand-anything/.gitignore` excludes the agent intermediate files
(`intermediate/`, `diff-overlay.json`) but keeps `knowledge-graph.json`,
`onboarding.md`, and `tours/` so they can be committed and shared.
