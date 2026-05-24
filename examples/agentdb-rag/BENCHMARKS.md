# RAG Pipeline Benchmarks

This file captures benchmark runs from `benchmark.sh`. Each run appends a new
section. Numbers are end-to-end timings (Node spawn → CLI → search → output)
unless explicitly labeled "raw query" (which strips the Node wrapper).

## What's measured

1. **Seed throughput** — `store-pattern` × N (each spawns a fresh `npx agentdb` process).
2. **End-to-end query latency** — `node src/query.mjs` → spawn `npx agentdb query` → parse → assemble prompt.
3. **Raw query latency** — `npx agentdb query` only, no Node wrapper, no JSON assembly.

## How to run

```bash
cd examples/agentdb-rag
./benchmark.sh                      # default: hash backend, 10 runs/question
EMBED_BACKEND=xenova ./benchmark.sh # local ML embeddings (needs install-embeddings)
RUNS=50 ./benchmark.sh              # more samples
```

## Caveats

- Each query spawns a fresh `npx agentdb` subprocess. Most of the wall-clock latency
  is Node.js startup + `npx` resolution. A long-lived process (or the MCP server
  in `MCP.md`) is the right shape for production — these numbers reflect the
  CLI-driven dev workflow, not the lower bound.
- The hash embedder is deterministic but semantically weak; swap to xenova or
  openai for production-grade recall.

---

## Run @ 2026-05-24T18:52:03Z

- Host: `drdave-TRX50-AI-TOP x86_64`
- Node: `v24.15.0`
- AgentDB: `v3.0.0-alpha.14`
- Embed backend: `hash`
- Docs in corpus: `24`
- DB size on disk: `491520` bytes

| Metric | Value |
|---|---|
| Seed time (total) | 18248 ms |
| Seed throughput | 1.32 docs/sec |
| End-to-end query latency (avg over 25) | 725 ms |
| Raw query latency (avg over 5, agentdb only) | 678 ms |

**Per-question end-to-end latency:**

- `724 ms` — How does HNSW indexing work?
- `724 ms` — What is product quantization?
- `725 ms` — Compare cosine vs euclidean distance
- `720 ms` — Which vector database does AgentDB use as backend?
- `731 ms` — When should I use MMR reranking?
