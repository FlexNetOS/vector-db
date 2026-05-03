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
5. No source code was modified.

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
