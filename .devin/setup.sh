#!/usr/bin/env bash
# Devin workspace setup for FlexNetOS/ruvector.
#
# This script bootstraps a fresh Devin VM (or any Ubuntu/Debian host) so that
# `cargo check --workspace --exclude ruvector-postgres` succeeds. It mirrors
# the steps in `.github/workflows/ci.yml` so local checks match CI.
#
# Notes:
# - `ruvector-postgres` is a pgrx-based PostgreSQL extension and is intentionally
#   excluded from workspace builds. Build it separately via `cargo build -p
#   ruvector-postgres` after running `cargo install cargo-pgrx --version 0.12.9
#   --locked` and `cargo pgrx init --pg17=$(which pg_config)`.
# - `hnsw_rs` is patched in-tree at `patches/hnsw_rs/` for WASM compatibility.
#   Do not bypass this patch.
# - `RUST_MIN_STACK = 16777216` is set workspace-wide via `.cargo/config.toml`
#   to avoid trait-resolution stack overflows in `ruvector-filter`.

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

log() { printf '\033[1;34m[devin-setup]\033[0m %s\n' "$*"; }

# 1. System dependencies. `libfontconfig1-dev` is required by the fontconfig
#    crate transitively pulled in by ruvector-cnn / plotters; CI installs it at
#    `.github/workflows/ci.yml:39`.
log "Installing system dependencies (libfontconfig1-dev)"
sudo apt-get update
sudo apt-get install -y libfontconfig1-dev

# 2. Rust toolchain. The workspace declares `rust-version = "1.77"` as a
#    minimum (edition 2021); CI uses stable. Add rustfmt and clippy components.
if ! command -v rustup >/dev/null 2>&1; then
  log "Installing rustup (stable)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    -y --default-toolchain stable --profile minimal --component rustfmt,clippy
  # shellcheck disable=SC1091
  . "${HOME}/.cargo/env"
else
  log "Ensuring stable toolchain with rustfmt + clippy is installed"
  rustup toolchain install stable --component rustfmt,clippy
  rustup default stable
fi

# 3. cargo-nextest is used by CI test shards (.github/workflows/ci.yml:238-241).
#    Install it once so local `cargo nextest run -p <crate>` matches CI.
if ! command -v cargo-nextest >/dev/null 2>&1; then
  log "Installing cargo-nextest"
  cargo install cargo-nextest --locked
else
  log "cargo-nextest already installed: $(cargo nextest --version | head -n1)"
fi

# 4. Node.js + npm workspaces. `npm/` hosts NAPI-RS / wasm-bindgen packages
#    (see `package.json` workspaces: `npm/core`, `npm/packages/*`).
if ! command -v node >/dev/null 2>&1 || ! node --version | grep -qE '^v20\.'; then
  log "Installing Node.js 20.x"
  curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
  sudo apt-get install -y nodejs
else
  log "Node.js already installed: $(node --version)"
fi

log "Installing npm dependencies (cd npm && npm ci)"
( cd "${REPO_ROOT}/npm" && npm ci )

# 5. Verification. `cargo check --workspace --exclude ruvector-postgres` is the
#    canonical sanity check; CI runs the same command at `.github/workflows/ci.yml:48`.
log "Verifying workspace compiles (cargo check)"
cargo check --workspace --exclude ruvector-postgres

log "Setup complete."
