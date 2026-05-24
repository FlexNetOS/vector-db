// embed.mjs — text → vector embeddings with three swappable backends.
//
// Default `hash` backend is dependency-free, fully deterministic, and good enough
// for a working RAG demo. For semantic quality, switch to `xenova` (local ML)
// or `openai` (text-embedding-3-small) via the EMBED_BACKEND env var.

const BACKEND = (process.env.EMBED_BACKEND || 'hash').toLowerCase();

export const EMBED_DIM = BACKEND === 'openai' ? 1536 : 384;
export const EMBED_BACKEND = BACKEND;

const FNV_OFFSET = 2166136261n;
const FNV_PRIME  = 16777619n;
const MASK_32    = 0xffffffffn;

function fnv1a(str) {
  let h = FNV_OFFSET;
  for (let i = 0; i < str.length; i++) {
    h ^= BigInt(str.charCodeAt(i));
    h = (h * FNV_PRIME) & MASK_32;
  }
  return Number(h);
}

function tokenize(text) {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, ' ')
    .split(/\s+/)
    .filter((t) => t.length > 1);
}

function hashEmbed(text) {
  const v = new Float32Array(EMBED_DIM);
  const tokens = tokenize(text);
  if (tokens.length === 0) return Array.from(v);
  for (const tok of tokens) {
    const a = fnv1a(tok) % EMBED_DIM;
    const b = fnv1a('::' + tok) % EMBED_DIM;
    v[a] += 1;
    v[b] += 0.5;
  }
  let norm = 0;
  for (let i = 0; i < EMBED_DIM; i++) norm += v[i] * v[i];
  norm = Math.sqrt(norm) || 1;
  for (let i = 0; i < EMBED_DIM; i++) v[i] /= norm;
  return Array.from(v);
}

let _xenovaPipe = null;
async function xenovaEmbed(text) {
  if (!_xenovaPipe) {
    const mod = await import('@xenova/transformers').catch(() => {
      throw new Error(
        "EMBED_BACKEND=xenova but @xenova/transformers is not installed.\n" +
        "Run: npx agentdb@latest install-embeddings"
      );
    });
    _xenovaPipe = await mod.pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2');
  }
  const out = await _xenovaPipe(text, { pooling: 'mean', normalize: true });
  return Array.from(out.data);
}

async function openaiEmbed(text) {
  const key = process.env.OPENAI_API_KEY;
  if (!key) throw new Error('EMBED_BACKEND=openai but OPENAI_API_KEY is unset.');
  const res = await fetch('https://api.openai.com/v1/embeddings', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${key}`,
    },
    body: JSON.stringify({ model: 'text-embedding-3-small', input: text }),
  });
  if (!res.ok) throw new Error(`OpenAI embeddings failed: ${res.status} ${await res.text()}`);
  const json = await res.json();
  return json.data[0].embedding;
}

export async function embed(text) {
  switch (BACKEND) {
    case 'hash':   return hashEmbed(text);
    case 'xenova': return await xenovaEmbed(text);
    case 'openai': return await openaiEmbed(text);
    default:       throw new Error(`unknown EMBED_BACKEND=${BACKEND}`);
  }
}

export async function embedBatch(texts) {
  const out = new Array(texts.length);
  for (let i = 0; i < texts.length; i++) out[i] = await embed(texts[i]);
  return out;
}
