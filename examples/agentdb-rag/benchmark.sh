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

: "${AGENTDB_PATH:=./vectors.db}"
: "${EMBED_BACKEND:=hash}"
RUNS="${RUNS:-10}"

echo "==> Benchmarking with EMBED_BACKEND=$EMBED_BACKEND, RUNS=$RUNS"

./init.sh > /tmp/agentdb-init.log 2>&1

echo "==> Measuring seed throughput"
SEED_START=$(date +%s%N)
node src/seed-docs.mjs > /tmp/agentdb-seed.log 2>&1
SEED_END=$(date +%s%N)
SEED_MS=$(( (SEED_END - SEED_START) / 1000000 ))
DOC_COUNT=$(wc -l < data/corpus.jsonl)
SEED_RATE=$(awk -v ms="$SEED_MS" -v n="$DOC_COUNT" 'BEGIN{printf "%.2f", n*1000/ms}')

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
  AGENTDB_PATH="$AGENTDB_PATH" npx --yes agentdb@latest query \
    --query "HNSW search?" --domain rag-corpus --k 5 --format json > /dev/null 2>&1
done
RAW_END=$(date +%s%N)
RAW_AVG_MS=$(( (RAW_END - RAW_START) / RUNS / 1000000 ))

DB_SIZE_BYTES=$(stat -c %s "$AGENTDB_PATH" 2>/dev/null || echo 0)

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
HOST=$(uname -nm)
NODE_VER=$(node --version)
AGENTDB_VER=$(npx --yes agentdb@latest --version 2>&1 | grep -oE 'v?[0-9]+\.[0-9]+\.[0-9]+[-a-z0-9.]*' | head -1 || echo "unknown")

cat > BENCHMARK_RESULTS.json <<EOF
{
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
