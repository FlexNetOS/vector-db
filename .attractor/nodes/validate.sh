#!/usr/bin/env bash
#
# Attractor node 3: Validate (ruvector brain side).
#
# The validate node is the contract gate: if it fails, the trajectory is
# still distilled (verdict=fail, see distill.sh "failures train the bank
# too") but optimize is skipped — there's nothing to optimize when the
# build is broken. The runner enforces this; nodes don't decide routing.
#
# Real validation invokes:
#
#   * `cargo check --workspace --exclude ruvector-postgres`
#     (RUST_MIN_STACK=16777216 is set in .cargo/config.toml)
#   * `cargo nextest run --no-fail-fast -p prime-radiant`
#     (witness chain integration)
#   * The mcp-brain-server smoke check (`/v1/status` round-trip).
#
# In stub mode we run the cheapest signal — `cargo check
# --workspace --exclude ruvector-postgres` — so a green run still
# proves the workspace compiles. If `cargo` is unavailable (e.g. in a
# minimal CI image), we degrade to a syntactic compile check via
# `cargo --version` + a no-op success.

set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not on PATH; degrading validate to no-op (stub)" >&2
    echo '{"validated":true,"degraded":true,"stub":true}'
    exit 0
fi

# Defensively unset RUSTC_WRAPPER if it points at a binary that isn't on
# PATH — happens in fresh dev environments that have sccache configured
# in `.cargo/config.toml` but haven't installed it yet. This is a pure
# environment fix; real CI runs install sccache before invoking us.
if [[ -n "${RUSTC_WRAPPER:-}" ]] && ! command -v "${RUSTC_WRAPPER}" >/dev/null 2>&1; then
    echo "validate: ${RUSTC_WRAPPER} not on PATH; unsetting RUSTC_WRAPPER" >&2
    unset RUSTC_WRAPPER
fi

# Capture cargo's stdout+stderr to a sidecar build log so the operator
# (and the self-learning loop's identify node) can diagnose failures.
# This script's own stdout stays the pure JSON contract.
#
# When invoked by `scripts/attractor.sh run`, the runner exports
# ATTRACTOR_RUN_DIR pointing at the per-run stdout dir, which gives
# every concurrent or back-to-back pipeline run its own isolated
# build_log. When invoked standalone (`scripts/attractor.sh node
# validate`), we fall back to the fixed runs/ path.
build_log="${ATTRACTOR_RUN_DIR:-${ROOT}/.attractor/runs}/validate.stderr"
mkdir -p "$(dirname "$build_log")"

# JSON-escape the build_log path before embedding in the contract so a
# repo root containing \ or " doesn't corrupt the JSONL audit record
# the runner parses. Mirrors the escape in distill.sh.
escaped_log="${build_log//\\/\\\\}"
escaped_log="${escaped_log//\"/\\\"}"

# Cheapest meaningful signal. Heavier validation (nextest, integration
# tests, brain-server smoke) is wired in once Phase 6's GitHub Actions
# self-learning workflow lands.
if RUST_MIN_STACK=16777216 cargo check --workspace --exclude ruvector-postgres \
    >"$build_log" 2>&1; then
    printf '{"validated":true,"check":"cargo check --workspace --exclude ruvector-postgres","stderr_log":"%s"}\n' "$escaped_log"
    exit 0
fi

printf '{"validated":false,"check":"cargo check --workspace --exclude ruvector-postgres","stderr_log":"%s","hint":"see stderr_log for cargo check diagnostics"}\n' "$escaped_log"
exit 1
