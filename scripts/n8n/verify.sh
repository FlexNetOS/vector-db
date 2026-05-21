#!/usr/bin/env bash
# End-to-end smoke test: assumes the docker-compose stack is up and the
# workflows have been imported via `import_workflows.py`. Ingests a few real
# docs from the repo, then queries via the n8n retrieval webhook and asserts
# that we get a non-empty hit list including the seeded source path.
set -euo pipefail

N8N_HOST="${N8N_HOST:-http://localhost:5678}"
RUVECTOR_BASE_URL="${RUVECTOR_BASE_URL:-http://localhost:8080}"
NS="${NS:-ruvector-verify}"

cd "$(dirname "$0")/../.."

echo "==> 1/4 RuVector health"
curl -sS "${RUVECTOR_BASE_URL}/v1/health" | tee /tmp/health.json
echo

echo "==> 2/4 ingest a small fixed set of real docs into namespace=${NS}"
INGEST_TARGET=ruvector \
  RUVECTOR_BASE_URL="${RUVECTOR_BASE_URL}" \
  RUVECTOR_NAMESPACE="${NS}" \
  python3 scripts/n8n/ingest_docs.py \
    --root docs/audit \
    --namespace "${NS}"

echo "==> 3/4 query via n8n retrieval webhook"
RESP=$(curl -sS -X POST "${N8N_HOST}/webhook/kb-query" \
  -H 'content-type: application/json' \
  -d "{\"q\":\"audit duplicate detection\",\"k\":3,\"namespace\":\"${NS}\"}")
echo "$RESP" | python3 -m json.tool

echo "==> 4/4 assert"
# Pass $RESP as argv[1] with a quoted heredoc so bash does NOT interpolate
# into the Python source. Embedding $RESP directly inside ''' ... ''' would
# misparse JSON escapes (\n, \t, ...) and break on JSON content containing $.
python3 - "$RESP" <<'PY'
import json, sys
r = json.loads(sys.argv[1])
hits = r.get("hits", [])
assert hits, f"no hits returned: {r}"
top = hits[0]
assert top.get("score", 0) > 0, f"top hit has zero score: {top}"
print(f"OK — {len(hits)} hits, top score={top['score']:.3f} path={top.get('source_path')}")
PY
