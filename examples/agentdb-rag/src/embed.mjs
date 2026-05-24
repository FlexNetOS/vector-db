// embed.mjs — text → vector embeddings with three swappable backends.
//
// Default `hash` backend is dependency-free, fully deterministic, and good enough
// for a working RAG demo. For semantic quality, switch to `xenova` (local ML)
// or `openai` (text-embedding-3-small) via the EMBED_BACKEND env var.

const BACKEND = (process.env.EMBED_BACKEND || 'hash').toLowerCase();

// Named dimensions per backend (Xenova/all-MiniLM-L6-v2 = 384,
// OpenAI text-embedding-3-small = 1536). Kept here as the single source
// of truth — init.sh's $DIM case must match.
const DIM_BY_BACKEND = Object.freeze({
  hash:   384,
  xenova: 384,
  openai: 1536,
});

/** Vector dimension produced by the current EMBED_BACKEND. @type {number} */
export const EMBED_DIM = DIM_BY_BACKEND[BACKEND] ?? 384;
/** Active embedding backend name. @type {'hash'|'xenova'|'openai'} */
export const EMBED_BACKEND = BACKEND;

// FNV-1a 32-bit constants. Held in JS Number space (int32) so the inner
// loop can use Math.imul instead of BigInt — ~10x faster on the hot path.
const FNV_OFFSET_32 = 0x811c9dc5;     // 2166136261
const FNV_PRIME_32  = 0x01000193;     // 16777619
const HASH_PRIMARY_WEIGHT   = 1.0;    // tok itself
const HASH_SECONDARY_WEIGHT = 0.5;    // '::tok' (collision-mitigation hash)

function fnv1a(str) {
  let h = FNV_OFFSET_32 | 0;
  for (let i = 0; i < str.length; i++) {
    h = (h ^ str.charCodeAt(i)) | 0;
    h = Math.imul(h, FNV_PRIME_32);
  }
  return h >>> 0; // coerce to uint32
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
    v[a] += HASH_PRIMARY_WEIGHT;
    v[b] += HASH_SECONDARY_WEIGHT;
  }
  let norm = 0;
  for (let i = 0; i < EMBED_DIM; i++) norm += v[i] * v[i];
  norm = Math.sqrt(norm) || 1;
  for (let i = 0; i < EMBED_DIM; i++) v[i] /= norm;
  return Array.from(v); // agentdb CLI expects plain-array JSON
}

let _xenovaPipe = null;
async function xenovaEmbed(text) {
  if (!_xenovaPipe) {
    const mod = await import('@xenova/transformers').catch(() => {
      throw new Error(
        "EMBED_BACKEND=xenova but @xenova/transformers is not installed.\n" +
        "Run: npx agentdb@3.0.0-alpha.14 install-embeddings"
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
  if (!res.ok) {
    // Deliberately do NOT include the response body — upstream error payloads
    // can echo the masked key, org id, or other sensitive headers. Status code
    // is enough to debug from logs.
    throw new Error(`OpenAI embeddings failed: HTTP ${res.status} ${res.statusText}`);
  }
  const json = await res.json();
  return json.data[0].embedding;
}

/**
 * Embed a single text string with the configured backend.
 * @param {string} text
 * @returns {Promise<number[]>} length === EMBED_DIM
 */
export async function embed(text) {
  switch (BACKEND) {
    case 'hash':   return hashEmbed(text);
    case 'xenova': return await xenovaEmbed(text);
    case 'openai': return await openaiEmbed(text);
    default:       throw new Error(`unknown EMBED_BACKEND=${BACKEND}`);
  }
}

/**
 * Embed many texts. Sequential by design — xenova batches internally,
 * openai is network-bound and benefits more from a real queue than naive
 * Promise.all, and the hash backend is already CPU-fast.
 * @param {string[]} texts
 * @returns {Promise<number[][]>}
 */
export async function embedBatch(texts) {
  const out = new Array(texts.length);
  for (let i = 0; i < texts.length; i++) out[i] = await embed(texts[i]);
  return out;
}
