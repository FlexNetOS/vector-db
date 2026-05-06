# n8n ⇄ RuVector Knowledge-Base Automation

This directory wires the FlexNetOS knowledge base into [n8n](https://n8n.io)
so changes in the docs tree can trigger automation workflows backed by
RuVector's vector-search and memory APIs.

## Architecture

```
                      ┌───────────────────────────────┐
                      │  Markdown / Wiki / crate docs │
                      └──────────────┬────────────────┘
                                     │ (1) walk + chunk
                                     ▼
            ┌──────────────────────────────────────────────┐
            │  scripts/n8n/ingest_docs.py                  │
            │   • normalises markdown                      │
            │   • POST /v1/memories  →  RuVector brain     │
            │   • OR  POST $N8N_INGEST_URL  →  n8n webhook │
            └──────────────┬───────────────────────────────┘
                           │ (2) HTTP/JSON
                           ▼
            ┌──────────────────────────────────────────────┐
            │  n8n workflow: knowledge-base-ingest         │
            │   • Webhook trigger (POST /webhook/kb-ingest)│
            │   • HTTP Request → RuVector /memories        │
            │   • Set/Function nodes for tagging           │
            │   • Respond with stored memory id            │
            └──────────────┬───────────────────────────────┘
                           │
                           ▼
            ┌──────────────────────────────────────────────┐
            │  n8n workflow: knowledge-retrieval           │
            │   • Webhook trigger (POST /webhook/kb-query) │
            │   • HTTP Request → RuVector /memories/search │
            │   • Function node for reranking              │
            │   • Respond with top-k JSON                  │
            └──────────────────────────────────────────────┘
```

The ingest path treats RuVector as the **single source of truth for vector
storage** (per the user's request). The retrieval path can be invoked from
any automation downstream — chat bot, ADR-validator, doc-bot, etc.

## Quick start

```bash
# 1. Bring up n8n + a local RuVector-compatible memory shim
docker compose -f scripts/n8n/docker-compose.yml up -d

# 2. Import the workflows into n8n
n8n_HOST=http://localhost:5678 \
RUVECTOR_BASE_URL=http://localhost:8080 \
python3 scripts/n8n/import_workflows.py

# 3. Ingest the knowledge base
RUVECTOR_BASE_URL=http://localhost:8080 \
python3 scripts/n8n/ingest_docs.py --root . --namespace ruvector-docs

# 4. Query
curl -sS -X POST http://localhost:5678/webhook/kb-query \
  -H 'content-type: application/json' \
  -d '{"q":"diskann vamana","k":3}' | jq .
```

## Files

| File | Purpose |
| ---  | ---     |
| `docker-compose.yml`             | Brings up n8n + the local RuVector memory shim. |
| `workflows/knowledge-base-ingest.json` | n8n workflow that accepts ingest webhooks and forwards to RuVector. |
| `workflows/knowledge-retrieval.json`   | n8n workflow that performs RAG-style retrieval over RuVector. |
| `ingest_docs.py`                 | Walks the repo, chunks markdown, and POSTs to either the n8n webhook or directly to RuVector. |
| `ruvector_shim.py`               | Tiny in-memory FastAPI shim implementing the same `/v1/memories*` surface as `mcp-brain-server-local`. Useful for local n8n testing without needing the full Rust build. |
| `import_workflows.py`            | Imports the workflow JSON files into a running n8n via its REST API. |
| `verify.sh`                      | End-to-end smoke test: ingest 3 docs, query, assert non-empty results. |

## RuVector backends

The same workflows work against three RuVector-compatible backends:

1. **`ruvector_shim.py`** (default for local dev) — Python in-memory shim
   on port 8080. Zero build time, used by the verify suite below.
2. **`mcp-brain-server-local`** — the Rust standalone with SQLite-backed
   storage (`crates/mcp-brain-server`, `--features local`). Long-running and
   crash-safe; recommended for local development beyond ad-hoc tests.
3. **`mcp-brain-server`** (Cloud Run / pi.ruv.io) — the production REST API
   under `/v1/memories*`. Switch by setting `RUVECTOR_BASE_URL` to
   `https://pi.ruv.io` and adding an `Authorization: Bearer <key>` header.

The workflows route through `RUVECTOR_BASE_URL` (an n8n environment variable),
so swapping backends is a config change with no workflow edit.

## Trigger surface

| Webhook                                 | Method | Purpose |
| ---                                     | ---    | ---     |
| `POST /webhook/kb-ingest`               | POST   | Ingest a single document (`{path, content, namespace, tags}`). |
| `POST /webhook/kb-ingest/batch`         | POST   | Ingest a batch (`{docs: [...]}`). |
| `POST /webhook/kb-query`                | POST   | Search (`{q, k, namespace}`). |
| `POST /webhook/kb-on-doc-change`        | POST   | Triggered by a CI hook on doc commits (re-ingests changed paths). |

The `kb-on-doc-change` workflow is wired to the audit pipeline in
[`scripts/audit/`](../audit/) — when a PR lands that touches `docs/**` or
`crates/**/README.md`, CI calls this webhook to keep the vector store in
sync, then triggers the duplicate scan for early warning of new conflicts.

The expected payload is:

```json
{
  "repo": "FlexNetOS/ruvector",
  "sha":  "abc123…",
  "changed_paths": [
    "docs/audit/AUDIT_REPORT.md",
    { "path": "docs/architecture/overview.md", "content": "<full file body>" }
  ]
}
```

Each `changed_paths` entry may be a bare string (the workflow then fetches the
file body from `https://raw.githubusercontent.com/<repo>/<sha>/<path>`, with
`Authorization: Bearer $GITHUB_TOKEN` for private repos) or an object that
carries the body inline (skipping the GitHub round-trip). Entries whose fetch
returns a 404 are forwarded to `kb-ingest` with `metadata.fetch_failed = true`
so the shim can decide whether to drop them or store a stub.

## See also

* [`docs/audit/AUDIT_REPORT.md`](../../docs/audit/AUDIT_REPORT.md) — the
  knowledge-base audit summary that produced the items currently flagged
  for manual review.
* [`docs/audit/MANUAL_REVIEW.md`](../../docs/audit/MANUAL_REVIEW.md) — the
  triage queue for items that need a human decision.
