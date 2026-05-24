#!/usr/bin/env bash
# init.sh — Initialize the AgentDB vector store for this example.
#
# Honors:
#   AGENTDB_PATH   — db file (default: ./vectors.db, in this example's dir)
#   EMBED_BACKEND  — hash | xenova | openai (default: hash)
#                    The chosen backend determines vector dimensionality.

set -euo pipefail

cd "$(dirname "$0")"

: "${AGENTDB_PATH:=./vectors.db}"
: "${EMBED_BACKEND:=hash}"

case "$EMBED_BACKEND" in
  hash)    DIM=384  ;;
  xenova)  DIM=384  ;;   # Xenova/all-MiniLM-L6-v2
  openai)  DIM=1536 ;;   # text-embedding-3-small
  *)       echo "unknown EMBED_BACKEND=$EMBED_BACKEND" >&2; exit 2 ;;
esac

# Wipe any prior DB so re-runs are idempotent (the file is .gitignore'd).
rm -f "$AGENTDB_PATH"

echo "==> agentdb init  path=$AGENTDB_PATH  dim=$DIM  backend=auto (ruvector preferred)"
npx --yes agentdb@latest init "$AGENTDB_PATH" --dimension "$DIM" --preset small

echo "==> agentdb status"
npx --yes agentdb@latest status --db "$AGENTDB_PATH" --verbose 2>&1 | sed 's/^/    /'

echo
echo "OK. Database ready at: $AGENTDB_PATH"
echo "Next: node src/seed-docs.mjs"
