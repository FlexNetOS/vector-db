// seed-docs.mjs — Load the sample corpus into AgentDB.
//
// Reads ../data/corpus.jsonl (one {id, text, metadata} per line),
// embeds each with the configured backend, and inserts via the AgentDB CLI.

import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { embed, EMBED_BACKEND, EMBED_DIM } from './embed.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const DB = process.env.AGENTDB_PATH || resolve(ROOT, 'vectors.db');
const CORPUS = resolve(ROOT, 'data', 'corpus.jsonl');

async function ensureCorpus() {
  if (existsSync(CORPUS)) return;
  await mkdir(dirname(CORPUS), { recursive: true });
  const docs = [
    { id: 'hnsw-01', text: 'HNSW (Hierarchical Navigable Small World) is a graph-based ANN index with O(log n) search.', meta: { topic: 'hnsw' } },
    { id: 'hnsw-02', text: 'HNSW layers form a probability-skipping hierarchy that approximates greedy nearest-neighbor descent.', meta: { topic: 'hnsw' } },
    { id: 'hnsw-03', text: 'In HNSW, the parameter M controls graph connectivity and ef_search controls recall at query time.', meta: { topic: 'hnsw' } },
    { id: 'pq-01', text: 'Product quantization splits a vector into subvectors and quantizes each via k-means codebooks.', meta: { topic: 'quantization' } },
    { id: 'pq-02', text: 'PQ reduces memory by 8-16x and enables asymmetric distance computation via precomputed tables.', meta: { topic: 'quantization' } },
    { id: 'bq-01', text: 'Binary quantization stores each dimension as a single bit, yielding 32x memory reduction.', meta: { topic: 'quantization' } },
    { id: 'sq-01', text: 'Scalar quantization maps float32 to int8 per dimension with 4x memory reduction and small recall loss.', meta: { topic: 'quantization' } },
    { id: 'dist-01', text: 'Cosine similarity measures angle, ignoring magnitude — best for normalized embeddings.', meta: { topic: 'distance' } },
    { id: 'dist-02', text: 'Euclidean (L2) distance includes magnitude and is appropriate for un-normalized vectors.', meta: { topic: 'distance' } },
    { id: 'dist-03', text: 'Dot product equals cosine on unit-norm vectors and is the fastest similarity to compute.', meta: { topic: 'distance' } },
    { id: 'rag-01', text: 'RAG injects retrieved documents into the prompt so the LLM can ground answers in your data.', meta: { topic: 'rag' } },
    { id: 'rag-02', text: 'A RAG pipeline chunks documents, embeds them, stores vectors, and retrieves top-k by similarity at query time.', meta: { topic: 'rag' } },
    { id: 'rag-03', text: 'Hybrid retrieval combines dense vector search with BM25 or keyword filters for higher recall.', meta: { topic: 'rag' } },
    { id: 'rag-04', text: 'MMR (Maximal Marginal Relevance) reranks results to balance relevance with diversity.', meta: { topic: 'rag' } },
    { id: 'chunk-01', text: 'Chunk size 500-1000 tokens with 10-20% overlap is a common starting point for RAG ingestion.', meta: { topic: 'chunking' } },
    { id: 'chunk-02', text: 'Semantic chunking splits on paragraph or sentence boundaries instead of fixed token counts.', meta: { topic: 'chunking' } },
    { id: 'embed-01', text: 'OpenAI text-embedding-3-small produces 1536-dimensional embeddings and is cheap to run.', meta: { topic: 'embeddings' } },
    { id: 'embed-02', text: 'sentence-transformers all-MiniLM-L6-v2 outputs 384-dim embeddings and runs locally without API costs.', meta: { topic: 'embeddings' } },
    { id: 'embed-03', text: 'BGE (BAAI General Embedding) models lead public retrieval leaderboards for English.', meta: { topic: 'embeddings' } },
    { id: 'db-01', text: 'Qdrant supports payload filters fused into HNSW search for filtered vector retrieval.', meta: { topic: 'vector-db' } },
    { id: 'db-02', text: 'Pinecone is a managed vector database with per-namespace isolation and serverless tiers.', meta: { topic: 'vector-db' } },
    { id: 'db-03', text: 'Faiss is a research-oriented C++ library with GPU support for IVF and PQ indexes.', meta: { topic: 'vector-db' } },
    { id: 'db-04', text: 'AgentDB combines vector search with reflexion episodes, causal edges, and skills under one DB.', meta: { topic: 'vector-db' } },
    { id: 'db-05', text: 'ruvector is a Rust-native vector index that AgentDB v3 auto-detects as its preferred backend.', meta: { topic: 'vector-db' } },
  ];
  const lines = docs.map((d) => JSON.stringify(d)).join('\n') + '\n';
  await writeFile(CORPUS, lines);
}

function runCli(args, input) {
  return new Promise((res, rej) => {
    const p = spawn('npx', ['--yes', 'agentdb@latest', ...args], {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, AGENTDB_PATH: DB },
    });
    let out = '', err = '';
    p.stdout.on('data', (b) => { out += b.toString(); });
    p.stderr.on('data', (b) => { err += b.toString(); });
    p.on('close', (code) => {
      if (code === 0) res({ out, err });
      else rej(new Error(`agentdb ${args.join(' ')} exited ${code}\n${err}`));
    });
    if (input != null) p.stdin.end(input);
    else p.stdin.end();
  });
}

async function main() {
  await ensureCorpus();
  const raw = await readFile(CORPUS, 'utf8');
  const docs = raw.split('\n').filter(Boolean).map((l) => JSON.parse(l));

  console.log(`==> embedding ${docs.length} documents with backend=${EMBED_BACKEND} dim=${EMBED_DIM}`);

  const t0 = Date.now();
  for (const doc of docs) {
    const vec = await embed(doc.text);
    const patternBody = {
      embedding: vec,
      text: doc.text,
      doc_id: doc.id,
      metadata: doc.meta || {},
    };
    await runCli([
      'store-pattern',
      '--type', 'document',
      '--domain', 'rag-corpus',
      '--pattern', JSON.stringify(patternBody),
      '--confidence', '1.0',
    ]);
    process.stdout.write('.');
  }
  process.stdout.write('\n');
  const dt = Date.now() - t0;
  console.log(`OK. Seeded ${docs.length} docs into ${DB} in ${dt} ms (${(dt / docs.length).toFixed(1)} ms/doc).`);
}

main().catch((e) => {
  console.error('seed-docs failed:', e.message);
  process.exit(1);
});
