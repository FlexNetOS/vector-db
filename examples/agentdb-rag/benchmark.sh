#!/usr/bin/env bash
# benchmark.sh — Capture latency for the RAG pipeline against the seeded corpus.
#
# Records:
#   - Seed throughput (docs/sec)
#   - Per-query end-to-end latency (embed + vector-search + prompt assembly)
#   - Raw vector-search latency (agentdb-only, no embed overhead)
#   - DB stats (size, pattern count)
#
# Output: BENCHMARK_RESULTS.json + appended row in BENCHMARKS.md

set -euo pipefail
cd "$(dirname "$0")"

# Bench uses a dedicated DB so it never clobbers the user's seeded vectors.db.
# Caller can still override with AGENTDB_PATH if they want bench == workload.
: "${AGENTDB_PATH:=./bench.db}"
: "${EMBED_BACKEND:=hash}"
RUNS="${RUNS:-10}"

export AGENTDB_PATH EMBED_BACKEND
echo "==> Benchmarking with EMBED_BACKEND=$EMBED_BACKEND, RUNS=$RUNS, DB=$AGENTDB_PATH"

# mktemp prevents the symlink-attack class where a local attacker pre-creates
# /tmp/agentdb-*.log as a symlink to an arbitrary file. Cleaned up on exit.
INIT_LOG=$(mktemp -t agentdb-init.XXXXXX.log)
SEED_LOG=$(mktemp -t agentdb-seed.XXXXXX.log)
trap 'rm -f "$INIT_LOG" "$SEED_LOG"' EXIT

./init.sh > "$INIT_LOG" 2>&1

echo "==> Measuring seed throughput"
SEED_START=$(date +%s%N)
node src/seed-docs.mjs > "$SEED_LOG" 2>&1
SEED_END=$(date +%s%N)
SEED_MS=$(( (SEED_END - SEED_START) / 1000000 ))
DOC_COUNT=$(wc -l < data/corpus.jsonl)
# Guard against SEED_MS=0 (would yield `inf` and break the JSON output).
if [[ "$SEED_MS" -le 0 ]]; then SEED_RATE="0.00"; else
  SEED_RATE=$(awk -v ms="$SEED_MS" -v n="$DOC_COUNT" 'BEGIN{printf "%.2f", n*1000/ms}')
fi

QUESTIONS=(
  "How does HNSW indexing work?"
  "What is product quantization?"
  "Compare cosine vs euclidean distance"
  "Which vector database does AgentDB use as backend?"
  "When should I use MMR reranking?"
)

echo "==> Measuring end-to-end query latency ($RUNS runs per question)"
TOTAL_LAT_NS=0
COUNT=0
declare -a PER_Q_RESULTS
for q in "${QUESTIONS[@]}"; do
  Q_TOTAL=0
  for ((i = 0; i < RUNS; i++)); do
    T0=$(date +%s%N)
    node src/query.mjs "$q" -k 5 > /dev/null 2>&1
    T1=$(date +%s%N)
    DT=$(( T1 - T0 ))
    Q_TOTAL=$(( Q_TOTAL + DT ))
    TOTAL_LAT_NS=$(( TOTAL_LAT_NS + DT ))
    COUNT=$(( COUNT + 1 ))
  done
  Q_AVG_MS=$(( Q_TOTAL / RUNS / 1000000 ))
  PER_Q_RESULTS+=("${Q_AVG_MS} ${q}")
  echo "    ${Q_AVG_MS} ms avg  —  $q"
done
AVG_LAT_MS=$(( TOTAL_LAT_NS / COUNT / 1000000 ))

echo "==> Measuring raw query latency (no Node wrapper)"
RAW_START=$(date +%s%N)
for ((i = 0; i < RUNS; i++)); do
  AGENTDB_PATH="$AGENTDB_PATH" npx --yes agentdb@3.0.0-alpha.14 query \
    --query "HNSW search?" --domain rag-corpus --k 5 --format json > /dev/null 2>&1
done
RAW_END=$(date +%s%N)
RAW_AVG_MS=$(( (RAW_END - RAW_START) / RUNS / 1000000 ))

DB_SIZE_BYTES=$(stat -c %s "$AGENTDB_PATH" 2>/dev/null || echo 0)

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
HOST=$(uname -nm)
NODE_VER=$(node --version)
AGENTDB_VER=$(npx --yes agentdb@3.0.0-alpha.14 --version 2>&1 | grep -oE 'v?[0-9]+\.[0-9]+\.[0-9]+[-a-z0-9.]*' | head -1 || echo "unknown")

# Build BENCHMARK_RESULTS.json via jq so any quote/newline/special character in
# $HOST or version strings is properly escaped. Falls back to a here-doc only
# if jq is unavailable, with the same warning.
if command -v jq >/dev/null 2>&1; then
  jq -n \
    --arg timestamp "$TIMESTAMP" \
    --arg host "$HOST" \
    --arg node "$NODE_VER" \
    --arg agentdb "$AGENTDB_VER" \
    --arg embed_backend "$EMBED_BACKEND" \
    --argjson runs "$RUNS" \
    --argjson doc_count "$DOC_COUNT" \
    --argjson db_size_bytes "$DB_SIZE_BYTES" \
    --argjson seed_total_ms "$SEED_MS" \
    --arg     seed_docs_per_sec "$SEED_RATE" \
    --argjson e2e_query_avg_ms "$AVG_LAT_MS" \
    --argjson raw_query_avg_ms "$RAW_AVG_MS" \
    '{schema:"ruvector.rag.bench/v1", timestamp:$timestamp, host:$host, node:$node,
      agentdb:$agentdb, embed_backend:$embed_backend, runs_per_question:$runs,
      doc_count:$doc_count, db_size_bytes:$db_size_bytes,
      seed_total_ms:$seed_total_ms, seed_docs_per_sec:($seed_docs_per_sec|tonumber),
      e2e_query_avg_ms:$e2e_query_avg_ms, raw_query_avg_ms:$raw_query_avg_ms}' \
    > BENCHMARK_RESULTS.json
else
  echo "warn: jq not found; JSON output may be malformed if host contains special chars" >&2
  cat > BENCHMARK_RESULTS.json <<EOF
{
  "schema": "ruvector.rag.bench/v1",
  "timestamp": "$TIMESTAMP",
  "host": "$HOST",
  "node": "$NODE_VER",
  "agentdb": "$AGENTDB_VER",
  "embed_backend": "$EMBED_BACKEND",
  "runs_per_question": $RUNS,
  "doc_count": $DOC_COUNT,
  "db_size_bytes": $DB_SIZE_BYTES,
  "seed_total_ms": $SEED_MS,
  "seed_docs_per_sec": $SEED_RATE,
  "e2e_query_avg_ms": $AVG_LAT_MS,
  "raw_query_avg_ms": $RAW_AVG_MS
}
EOF
fi

{
  echo
  echo "## Run @ $TIMESTAMP"
  echo
  echo "- Host: \`$HOST\`"
  echo "- Node: \`$NODE_VER\`"
  echo "- AgentDB: \`$AGENTDB_VER\`"
  echo "- Embed backend: \`$EMBED_BACKEND\`"
  echo "- Docs in corpus: \`$DOC_COUNT\`"
  echo "- DB size on disk: \`$DB_SIZE_BYTES\` bytes"
  echo
  echo "| Metric | Value |"
  echo "|---|---|"
  echo "| Seed time (total) | ${SEED_MS} ms |"
  echo "| Seed throughput | ${SEED_RATE} docs/sec |"
  echo "| End-to-end query latency (avg over $((RUNS * ${#QUESTIONS[@]}))) | ${AVG_LAT_MS} ms |"
  echo "| Raw query latency (avg over $RUNS, agentdb only) | ${RAW_AVG_MS} ms |"
  echo
  echo "**Per-question end-to-end latency:**"
  echo
  for line in "${PER_Q_RESULTS[@]}"; do
    Q_MS=$(echo "$line" | cut -d' ' -f1)
    Q_TXT=$(echo "$line" | cut -d' ' -f2-)
    echo "- \`${Q_MS} ms\` — $Q_TXT"
  done
} >> BENCHMARKS.md

echo
echo "==> Wrote BENCHMARK_RESULTS.json"
echo "==> Appended results to BENCHMARKS.md"
cat BENCHMARK_RESULTS.json
