// query.mjs — End-to-end RAG query.
//
// 1. Embed the query LOCALLY (for caller-side analysis / future reranking).
// 2. agentdb query --query <text> → top-k similar patterns (internal embed).
// 3. Assemble a context-injected prompt and print it.
//
// Why we use `agentdb query` (text) instead of `agentdb vector-search`:
//   - `query` embeds with the same model used at insert time, so dimension
//     and embedder always match.
//   - `vector-search` requires the caller's vector to match the index dim
//     exactly, and AgentDB v3.0.0-alpha.14 currently stores patterns through
//     its internal embedder (Xenova/all-MiniLM-L6-v2, 384-dim) which can
//     differ from a BYO embedder unless carefully aligned.
//   - For OFFLINE vector-only workflows (no agentdb at query time), use
//     embed.mjs + your own ANN library. embed.mjs is exported for that use.
//
// Usage:
//   node src/query.mjs "your question here" [-k N] [--threshold 0.5] [--domain rag-corpus]
//
// Output: JSON object {query, results, prompt} on stdout. Pipe `prompt` into any LLM.

import { spawn } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync } from 'node:fs';
import { embed, EMBED_BACKEND } from './embed.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const DB = process.env.AGENTDB_PATH || resolve(ROOT, 'vectors.db');

function parseArgs(argv) {
  const args = { k: 5, threshold: 0, domain: 'rag-corpus', positional: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '-k') args.k = parseInt(argv[++i], 10);
    else if (a === '--threshold' || a === '-t') args.threshold = parseFloat(argv[++i]);
    else if (a === '--domain' || a === '-d') args.domain = argv[++i];
    else args.positional.push(a);
  }
  return args;
}

function runCli(args, env = {}) {
  return new Promise((res, rej) => {
    const p = spawn('npx', ['--yes', 'agentdb@latest', ...args], {
      env: { ...process.env, AGENTDB_PATH: DB, ...env },
    });
    let out = '', err = '';
    p.stdout.on('data', (b) => { out += b.toString(); });
    p.stderr.on('data', (b) => { err += b.toString(); });
    p.on('close', (code) => {
      if (code === 0) res({ out, err });
      else rej(new Error(`agentdb ${args.join(' ')} exited ${code}\n${err || out}`));
    });
  });
}

// ANSI escape codes: ESC [ <params> <intermediate> <final>
const ANSI_RE = /\x1b\[[0-9;]*[a-zA-Z]/g;

function extractJsonPayload(stdout) {
  // Strip ANSI color codes (their `[` sequences confuse bracket scanning).
  const clean = stdout.replace(ANSI_RE, '');
  // Walk lines until one starts with `[` or `{`; parse from there to end.
  const lines = clean.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const first = lines[i].trimStart()[0];
    if (first !== '[' && first !== '{') continue;
    const candidate = lines.slice(i).join('\n').trim();
    try {
      return JSON.parse(candidate);
    } catch {
      // try shrinking from the end — handles trailing CLI epilogues
      for (let j = candidate.length; j > 0; j--) {
        try { return JSON.parse(candidate.slice(0, j)); } catch { /* shrink */ }
      }
    }
  }
  return null;
}

function summarizeHit(h) {
  // AgentDB v3 `query` returns episode-shaped objects: the original pattern
  // body is wrapped in `critique` (string) or `pattern_data` (object), depending
  // on which storage path created it. Try both, plus a few legacy field names.
  let body = h.pattern_data ?? h.data;
  if (typeof body === 'string') {
    try { body = JSON.parse(body); } catch { body = { text: body }; }
  }
  if (!body && typeof h.critique === 'string') {
    try { body = JSON.parse(h.critique); } catch { body = { text: h.critique }; }
  }
  body = body || {};
  // store-pattern wraps the user pattern inside body.pattern; reflexion store
  // puts fields at top level. Accept both.
  const inner = body.pattern && typeof body.pattern === 'object' ? body.pattern : body;
  return {
    id: h.id,
    similarity: h.similarity ?? h.score ?? null,
    confidence: h.confidence ?? null,
    doc_id: inner.doc_id ?? null,
    text: inner.text ?? body.task ?? h.task ?? null,
    metadata: inner.metadata ?? null,
  };
}

function buildPrompt(question, hits) {
  const ctxBlocks = hits.map((h, i) => {
    const text = h.text || '[no text]';
    const id = h.doc_id || h.id || `hit-${i}`;
    return `[${i + 1}] (${id}) ${text}`;
  });
  return [
    'You are a precise technical assistant. Answer ONLY from the context below.',
    'If the context does not contain the answer, say "not in the provided context".',
    'Cite sources inline as [1], [2], etc.',
    '',
    '=== CONTEXT ===',
    ctxBlocks.join('\n'),
    '',
    '=== QUESTION ===',
    question,
    '',
    '=== ANSWER ===',
  ].join('\n');
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const question = args.positional.join(' ').trim();
  if (!question) {
    console.error('usage: node src/query.mjs "your question" [-k N] [--threshold X] [--domain D]');
    process.exit(2);
  }
  if (!existsSync(DB)) {
    console.error(`vector DB not found at ${DB}. Run ./init.sh && node src/seed-docs.mjs first.`);
    process.exit(2);
  }

  // Embed locally for caller-side use (analysis, reranking, offline workflows).
  // The actual retrieval below uses agentdb's internal embedder for consistency
  // with how documents were stored.
  const localVec = await embed(question);

  const cliArgs = [
    'query',
    '--query', question,
    '--domain', args.domain,
    '--k', String(args.k),
    '--format', 'json',
  ];
  if (args.threshold > 0) cliArgs.push('--min-confidence', String(args.threshold));

  const t0 = Date.now();
  const { out } = await runCli(cliArgs);
  const dt = Date.now() - t0;

  const parsed = extractJsonPayload(out);
  let hits = Array.isArray(parsed) ? parsed : parsed?.results || parsed?.matches || parsed?.patterns || [];
  hits = hits.map(summarizeHit);

  const prompt = buildPrompt(question, hits);
  console.log(JSON.stringify({
    query: question,
    local_embed_backend: EMBED_BACKEND,
    local_embed_dim: localVec.length,
    db: DB,
    domain: args.domain,
    search_ms: dt,
    k: args.k,
    threshold: args.threshold,
    results: hits,
    prompt,
  }, null, 2));
}

main().catch((e) => {
  console.error('query failed:', e.message);
  process.exit(1);
});
