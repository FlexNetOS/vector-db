#!/usr/bin/env bash
# Re-vendor the cognitum-one Rust SDK from crates.io into vendor/cognitum-one/.
#
# Usage:
#   scripts/vendor-cognitum-one.sh <version> <sha256>
#
# Example:
#   scripts/vendor-cognitum-one.sh 0.2.1 \
#     0db3dccd4aa8ffbe593dc76ace747e163e549215a68325ccb09bfac21ac8b3aa
#
# After running, update vendor/cognitum-one/VENDORED.md (version, SHA, commit
# from UPSTREAM_VCS_INFO.json) and the workspace Cargo.toml comment block.
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <version> <sha256>" >&2
  exit 64
fi

VERSION="$1"
EXPECTED_SHA256="$2"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$REPO_ROOT/vendor/cognitum-one"
TARBALL="$(mktemp -t cognitum-one.XXXXXX.crate)"
trap 'rm -f "$TARBALL"' EXIT

echo "==> Downloading cognitum-one v${VERSION} from crates.io..."
curl -fsSL -o "$TARBALL" \
  "https://crates.io/api/v1/crates/cognitum-one/${VERSION}/download"

echo "==> Verifying SHA-256..."
echo "${EXPECTED_SHA256}  ${TARBALL}" | sha256sum -c -

echo "==> Replacing $DEST..."
rm -rf "$DEST"
mkdir -p "$DEST"
tar xzf "$TARBALL" -C "$DEST" --strip-components=1

# Use the upstream-authored Cargo.toml, not the cargo-rewritten one.
mv "$DEST/Cargo.toml.orig" "$DEST/Cargo.toml"
# Lockfile resolution is owned by the host workspace.
rm -f "$DEST/Cargo.lock"
# Preserve VCS metadata under a clearly labelled name.
mv "$DEST/.cargo_vcs_info.json" "$DEST/UPSTREAM_VCS_INFO.json"
# Drop any stray build artefacts.
rm -rf "$DEST/target"

echo "==> Done. New file count: $(find "$DEST" -type f | wc -l)"
echo "    Upstream commit:    $(grep -o '\"sha1\": \"[^\"]*\"' "$DEST/UPSTREAM_VCS_INFO.json")"
echo
echo "Next steps:"
echo "  1. Edit vendor/cognitum-one/VENDORED.md (version, SHA, commit)."
echo "  2. Edit Cargo.toml [workspace.dependencies] comment block."
echo "  3. Run cargo metadata --no-deps --format-version 1 >/dev/null to verify."
