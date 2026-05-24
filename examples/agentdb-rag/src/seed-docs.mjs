// seed-docs.mjs — Load the sample corpus into AgentDB.
//
// Reads ../data/corpus.jsonl (one {id, text, metadata} per line),
// embeds each with the configured backend, and inserts via the AgentDB CLI.

import { readFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { embed, EMBED_BACKEND, EMBED_DIM } from './embed.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const DB = process.env.AGENTDB_PATH || resolve(ROOT, 'vectors.db');
const CORPUS = resolve(ROOT, 'data', 'corpus.jsonl');

// data/corpus.jsonl is the single source of truth. Fail loudly if it's
// missing instead of regenerating from an embedded literal — duplicated
// seed data was a documented drift hazard.
function assertCorpus() {
  if (!existsSync(CORPUS)) {
    throw new Error(
      `corpus not found at ${CORPUS}\n` +
      `Restore it from git: git checkout HEAD -- data/corpus.jsonl`
    );
  }
}

function runCli(args, input) {
  return new Promise((res, rej) => {
    const p = spawn('npx', ['--yes', 'agentdb@3.0.0-alpha.14', ...args], {
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
  assertCorpus();
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
