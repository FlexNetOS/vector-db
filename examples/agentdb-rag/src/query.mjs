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
    const p = spawn('npx', ['--yes', 'agentdb@3.0.0-alpha.14', ...args], {
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

// Single-pass bracket-depth scanner. Finds the first `[`/`{` in the cleaned
// stdout, walks forward tracking depth + string state, and returns the
// substring spanning the matched outer container. O(n) instead of the
// previous O(n^2) shrink-loop, which was both slow and a CPU-DoS vector
// against attacker-influenced CLI output.
function extractJsonPayload(stdout) {
  const clean = stdout.replace(ANSI_RE, '');
  const n = clean.length;
  let i = 0;
  // Find the first opening bracket of a JSON value
  while (i < n && clean[i] !== '[' && clean[i] !== '{') i++;
  if (i >= n) return null;

  const open = clean[i];
  const close = open === '[' ? ']' : '}';
  let depth = 0;
  let inString = false;
  let escape = false;

  for (let j = i; j < n; j++) {
    const c = clean[j];
    if (inString) {
      if (escape) { escape = false; continue; }
      if (c === '\\') { escape = true; continue; }
      if (c === '"') inString = false;
      continue;
    }
    if (c === '"') { inString = true; continue; }
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) {
        try { return JSON.parse(clean.slice(i, j + 1)); } catch { return null; }
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

// Neutralize delimiter strings ("=== CONTEXT ===" etc.) inside retrieved doc
// text so a malicious document cannot inject fake sections that override the
// system instructions. This is a minimum-viable defense — production RAG
// systems should also validate ingest sources and consider structured prompts.
function sanitizeContext(text) {
  if (text == null) return '[no text]';
  return String(text).replace(/={3,}/g, '===');
}

function buildPrompt(question, hits) {
  const ctxBlocks = hits.map((h, i) => {
    const text = sanitizeContext(h.text || '[no text]');
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
    sanitizeContext(question),
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
    // Stable output contract — agents that consume this JSON should pin to
    // a major version and treat unknown minor-version fields as additive.
    schema: 'ruvector.rag.query/v1',
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
