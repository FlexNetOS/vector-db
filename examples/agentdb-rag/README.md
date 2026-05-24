# AgentDB RAG — Self-Contained Vector Search + RAG Example

End-to-end RAG (Retrieval-Augmented Generation) pipeline using **AgentDB v3** on top of
ruvector's HNSW backend. No external API keys required by default — uses a deterministic
hashed embedder so the example is reproducible. Drop-in upgrade paths to
`@xenova/transformers` (local ML) or OpenAI are documented below.

## What this example demonstrates

| # | Feature | File |
|---|---------|------|
| 1 | Initialize an AgentDB vector store with ruvector backend | `init.sh` |
| 2 | Deterministic & ML-backed text embeddings | `src/embed.mjs` |
| 3 | Seed a knowledge base from `data/corpus.jsonl` | `src/seed-docs.mjs` |
| 4 | Query: embed → vector-search → context-injected prompt | `src/query.mjs` |
| 5 | Wire AgentDB MCP server into Claude Code | `MCP.md` |
| 6 | Benchmark vs. baselines (latency, throughput, recall) | `benchmark.sh`, `BENCHMARKS.md` |

## Prerequisites

- Node.js 18+ (verified on v24.15.0)
- `npx` (bundled with npm)
- ~50MB free disk for the test corpus

No global installs. Everything runs through `npx agentdb@latest`.

## Quick start

```bash
cd examples/agentdb-rag

# 1) Initialize the vector store
./init.sh

# 2) Seed the knowledge base (writes 24 documents)
node src/seed-docs.mjs

# 3) Ask questions
node src/query.mjs "How does HNSW indexing work?"
node src/query.mjs "What is product quantization?" -k 3
node src/query.mjs "Compare cosine vs euclidean distance" --threshold 0.3
```

## Architecture

```
INGEST (one-time)                    QUERY (per question)
─────────────────                    ────────────────────
text + embedding                     question text
       │                                    │
       ▼                                    ▼
agentdb store-pattern            agentdb query --query
   (domain=rag-corpus)              (auto-embeds + ANN)
       │                                    │
       ▼                                    ▼
   vectors.db ◀───────HNSW──────── top-k pattern hits
                                            │
                                            ▼
                                    buildPrompt(question, hits)
                                            │
                                            ▼
                                    LLM of your choice
```

Two embedders coexist on purpose:

1. **AgentDB's internal embedder** (Xenova/all-MiniLM-L6-v2, 384-dim) handles
   both write and read paths — guarantees no dimension mismatch.
2. **`src/embed.mjs`** (hash / xenova / openai, swappable via `EMBED_BACKEND`)
   is exposed for offline analysis, reranking, or BYO-vector workflows.

The pipeline is provider-agnostic: `query.mjs` emits a context-injected prompt
as plain text. Pipe it to `claude`, `gpt-4`, or any local LLM.

## Upgrade paths

### Real ML embeddings (Xenova/transformers)

```bash
npx agentdb@latest install-embeddings
EMBED_BACKEND=xenova ./init.sh && EMBED_BACKEND=xenova node src/seed-docs.mjs
EMBED_BACKEND=xenova node src/query.mjs "your question"
```

### OpenAI embeddings (text-embedding-3-small, 1536 dim)

```bash
export OPENAI_API_KEY=sk-...
EMBED_BACKEND=openai ./init.sh && EMBED_BACKEND=openai node src/seed-docs.mjs
EMBED_BACKEND=openai node src/query.mjs "your question"
```

## MCP integration

See [`MCP.md`](./MCP.md) — adds AgentDB tools (vector_search, store_pattern, query) to
Claude Code via `claude mcp add`.

## Benchmarks

See [`BENCHMARKS.md`](./BENCHMARKS.md) — captured on the host that ran this example.

## License

Same as the parent repo (FlexNetOS/ruvector).
