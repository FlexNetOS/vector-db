#!/usr/bin/env bash
# Guardrail for PRs: source stays in git; generated binaries go to
# GitHub Releases, GHCR, or Actions artifacts.
set -euo pipefail

base_ref="${1:-${GITHUB_BASE_REF:-}}"
if [[ -n "$base_ref" ]]; then
  git fetch --no-tags --depth=1 origin "$base_ref" >/dev/null 2>&1 || true
  range="origin/${base_ref}...HEAD"
else
  range="HEAD^...HEAD"
fi

forbidden_regex='\.(bin|elf|uf2|hex|hef|onnx|safetensors|pt|pth|ckpt|dylib|so|dll|a)$'
allowed_binary_regex='^npm/core/(native|platforms)/.*\.node$'

violations=()
while IFS= read -r path; do
  [[ -z "$path" ]] && continue
  if [[ "$path" =~ $forbidden_regex ]]; then
    violations+=("$path")
  fi
  if [[ "$path" =~ \.node$ && ! "$path" =~ $allowed_binary_regex ]]; then
    violations+=("$path")
  fi
done < <(git diff --name-only "$range")

if (( ${#violations[@]} > 0 )); then
  printf 'Generated or large binary artifacts must not be committed. Publish them via GitHub Releases, GHCR, or Actions artifacts instead:\n' >&2
  printf '  - %s\n' "${violations[@]}" >&2
  exit 1
fi

python3 - <<'PY'
import json
from pathlib import Path
manifest = Path('docs/hailo/models.manifest.json')
data = json.loads(manifest.read_text())
for model in data.get('models', []):
    if not model.get('id') or not model.get('license') or not model.get('default_install_dir'):
        raise SystemExit(f"model entry missing id/license/default_install_dir: {model!r}")
    for artifact in model.get('artifacts', []):
        for key in ('file', 'url', 'sha256'):
            if not artifact.get(key):
                raise SystemExit(f"artifact missing {key}: {artifact!r}")
        if len(artifact['sha256']) != 64:
            raise SystemExit(f"artifact sha256 must be 64 hex chars: {artifact!r}")
print('artifact policy OK')
PY
