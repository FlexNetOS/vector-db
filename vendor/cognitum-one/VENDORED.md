# Vendored: cognitum-one v0.2.1

This directory contains a vendored copy of the **`cognitum-one`** Rust SDK,
imported into the FlexNetOS/ruvector workspace so the codebase has a single
source of truth that does not depend on crates.io being reachable at build
time.

| Field | Value |
|---|---|
| Crate name | `cognitum-one` |
| Version | `0.2.1` |
| Upstream repository | `https://github.com/cognitum-one/sdks` (path `sdks/rust`) |
| Upstream commit | `a9e1c073ef017bc35c70a620579a1a4947213f10` (per `UPSTREAM_VCS_INFO.json`) |
| Source tarball | `https://crates.io/api/v1/crates/cognitum-one/0.2.1/download` |
| Source tarball SHA-256 | `0db3dccd4aa8ffbe593dc76ace747e163e549215a68325ccb09bfac21ac8b3aa` |
| Imported on | 2026-04-29 |
| License | MIT (see `LICENSE`) |

## What was changed during vendoring

1. The crates.io tarball was extracted in place.
2. `Cargo.toml.orig` (the human-authored manifest from the upstream repo) was
   renamed to `Cargo.toml`. The cargo-rewritten manifest that ships in the
   tarball as `Cargo.toml` was discarded.
3. The crate-local `Cargo.lock` was removed — lockfile resolution is owned by
   the host workspace.
4. `.cargo_vcs_info.json` was renamed to `UPSTREAM_VCS_INFO.json` so it remains
   discoverable but does not look like a build-tool file.

## In-tree deviations from upstream v0.2.1

The original vendoring was **byte-for-byte from the crates.io tarball**.
The list below tracks every subsequent edit applied in-tree, with the rationale.
Each deviation is also annotated in the source with a `FlexNetOS deviation:`
comment so re-vendoring catches drift. When upstream ships a fix, the matching
deviation should be removed (see "Updating the vendored copy" below).

| File | Symbol | Bug | Fix | Upstream status |
|---|---|---|---|---|
| `src/seed/peers.rs` | `PeerSet::pick_random` | `Instant::now().elapsed()` returns ~`Duration::ZERO` so `seed % candidates.len()` was always `0` — the function was deterministic on `candidates[0]`. | Switched seed source to `SystemTime::now().duration_since(UNIX_EPOCH).subsec_nanos()`. | Not yet filed upstream. |
| `src/mcp/resource.rs` | `McpResource::call_tool`, `McpResource::initialize` | `err.code as u16` wraps negative JSON-RPC error codes (e.g. `-32601` -> `32935`), losing the spec-defined sign. | Changed to `err.code.unsigned_abs().min(u16::MAX as u64) as u16` — same idiom already in use at `src/mcp/transport.rs::From<McpError>`. | Not yet filed upstream. |
| `src/seed/error.rs` | `from_response` | On HTTP 429 the function hardcoded `retry_after_ms = 1000`, ignoring the `Retry-After` header and any body hint. | Added `from_response_with_headers(status, headers, body, path)` which parses via `seed::retry::parse_retry_after`, falling back to 1000ms only when no hint is present. The original `from_response` is preserved as a header-less wrapper for backward compatibility (notably the in-crate tests). All call sites in `src/seed/client.rs` were updated to pass the response headers. | Not yet filed upstream. |

Verification commands (run from `vendor/cognitum-one/`):

```bash
cargo check --no-default-features --features "rustls,seed"
cargo test  --no-default-features --features "rustls,seed" --lib seed::error
cargo test  --no-default-features --features "rustls,seed" --lib seed::peers
```

The pre-existing `client::tests::invalid_pem_is_surfaced_as_validation_error`
upstream-test failure is unrelated to the FlexNetOS deviations and is reproducible
on a clean v0.2.1 checkout.

## How the workspace consumes this crate

The root `Cargo.toml` declares it as a workspace dependency:

```toml
[workspace.dependencies]
cognitum-one = { path = "vendor/cognitum-one", default-features = false, features = ["rustls", "seed"] }
```

Member crates opt in by writing `cognitum-one.workspace = true` in their own
`Cargo.toml`. The vendored crate is **not** added to `[workspace.members]` —
it lives outside the workspace as a path-resolved dependency. This keeps the
vendored sources out of `cargo check --workspace`, `cargo clippy
--workspace`, and `cargo test --workspace` runs (the host workspace's lint
policy is much stricter than what we want to enforce on vendored upstream
code).

## Updating the vendored copy

Use the `scripts/vendor-cognitum-one.sh` helper at the repo root, or:

```bash
VERSION=0.2.x
SHA256=...
curl -L -o /tmp/cognitum-one.crate \
  "https://crates.io/api/v1/crates/cognitum-one/${VERSION}/download"
echo "${SHA256}  /tmp/cognitum-one.crate" | sha256sum -c -
rm -rf vendor/cognitum-one
mkdir -p vendor/cognitum-one
tar xzf /tmp/cognitum-one.crate -C vendor/cognitum-one --strip-components=1
mv vendor/cognitum-one/Cargo.toml.orig vendor/cognitum-one/Cargo.toml
mv vendor/cognitum-one/.cargo_vcs_info.json vendor/cognitum-one/UPSTREAM_VCS_INFO.json
rm -f vendor/cognitum-one/Cargo.lock vendor/cognitum-one/target -rf
# Update this file (version, SHA, commit) and bump the version field above.
```

Always commit the `UPSTREAM_VCS_INFO.json` SHA1 alongside the source bump so
provenance is auditable.
