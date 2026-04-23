//! The `table.rulake.json` bundle sidecar.
//!
//! This is the portable unit that defines the reproducibility and
//! governance scope of a ruLake-compatible table. It does not contain
//! the vectors themselves; it *points at* them (via `data_ref`) and
//! carries the cryptographically-linked metadata needed to prove that
//! a given cache entry was built from exactly those bytes.
//!
//! ## Why this matters more than the UDF
//!
//! The remote function, the BigQuery integration, the cache service —
//! all of them are implementation details that can be swapped. The
//! bundle is what travels: between teams, between clouds, between
//! backups-and-restores. If the bundle format is wrong, every
//! layer on top of it leaks.
//!
//! ## Contract
//!
//! A ruLake bundle MUST carry:
//!
//! - **`data_ref`** — URI pointing at the authoritative byte stream
//!   (Parquet on GCS, an Iceberg table, an RVF segment, etc). ruLake
//!   does not own these bytes.
//! - **`dim`** — vector dimensionality. Immutable once set.
//! - **`rotation_seed`** — the Haar-uniform rotation seed used by the
//!   RaBitQ compressor. Deterministic — two caches built with the
//!   same seed produce byte-identical codes.
//! - **`rerank_factor`** — RaBitQ rerank factor. Declared so consumers
//!   can reason about the recall guarantee without re-running the
//!   sweep.
//! - **`generation`** — the backend's own coherence token at the time
//!   the bundle was produced (mtime, snapshot id, etc). Opaque `Bytes`
//!   so we're not locked to u64 when Iceberg hands us a UUID.
//! - **`rvf_witness`** — SHAKE-256 digest over `(data_ref, dim,
//!   rotation_seed, rerank_factor, generation)`. The cache-key anchor.
//!   Two bundles with the same witness are interchangeable for any
//!   query.
//! - **`pii_policy`** — opaque per-lake classifier reference. ruLake
//!   doesn't interpret it in v1; it passes it through so governance
//!   layers can enforce.
//! - **`lineage_id`** — OpenLineage job id that produced this bundle.
//!
//! The witness makes bundles equality-testable without comparing
//! payloads — two instances of ruLake observing the same bundle from
//! different backends can cache-share safely.

use serde::{Deserialize, Serialize};

/// Opaque coherence token. Large enough to hold a Parquet mtime
/// (u64 seconds), an Iceberg snapshot id (i64 positive), or a
/// Snowflake change-stream offset (u64 nanos). For longer tokens —
/// most notably multi-part Delta transaction logs or raw UUIDs — use
/// the `Opaque` variant and store the serialized bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Generation {
    /// Numeric token (mtime, version, snapshot id).
    Num(u64),
    /// Opaque string (UUID, hash, base64 blob).
    Opaque(String),
}

impl Generation {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Num(n) => Some(*n),
            Self::Opaque(_) => None,
        }
    }

    /// Concatenate into a byte stream for witness digest input.
    pub fn hash_bytes(&self) -> Vec<u8> {
        match self {
            Self::Num(n) => n.to_le_bytes().to_vec(),
            Self::Opaque(s) => s.as_bytes().to_vec(),
        }
    }
}

impl From<u64> for Generation {
    fn from(n: u64) -> Self {
        Self::Num(n)
    }
}

impl From<String> for Generation {
    fn from(s: String) -> Self {
        Self::Opaque(s)
    }
}

/// The bundle payload. Serializes to `table.rulake.json` verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuLakeBundle {
    /// Semantic version of the bundle format. Bump on breaking changes.
    pub format_version: u32,

    /// URI of the authoritative byte stream. Not owned by ruLake.
    pub data_ref: String,

    /// Vector dimensionality.
    pub dim: usize,

    /// RaBitQ rotation seed. Determines the cache's compressed codes.
    pub rotation_seed: u64,

    /// RaBitQ rerank factor. Determines the recall guarantee.
    pub rerank_factor: usize,

    /// Backend-reported coherence token at bundle-emission time.
    pub generation: Generation,

    /// Anchored digest over the other fields — the cache-key anchor.
    /// SHAKE-256(32) hex-encoded.
    pub rvf_witness: String,

    /// Opaque PII-policy handle — passed to the governance layer.
    pub pii_policy: Option<String>,

    /// OpenLineage job id that produced this bundle.
    pub lineage_id: Option<String>,
}

impl RuLakeBundle {
    pub const FORMAT_VERSION: u32 = 1;

    /// Construct a bundle, computing the witness from the other fields.
    pub fn new(
        data_ref: impl Into<String>,
        dim: usize,
        rotation_seed: u64,
        rerank_factor: usize,
        generation: Generation,
    ) -> Self {
        let data_ref = data_ref.into();
        let witness = compute_witness(&data_ref, dim, rotation_seed, rerank_factor, &generation);
        Self {
            format_version: Self::FORMAT_VERSION,
            data_ref,
            dim,
            rotation_seed,
            rerank_factor,
            generation,
            rvf_witness: witness,
            pii_policy: None,
            lineage_id: None,
        }
    }

    /// Recompute the witness from the other fields. Useful after
    /// deserialization to verify a bundle hasn't been tampered with.
    pub fn verify_witness(&self) -> bool {
        let expected = compute_witness(
            &self.data_ref,
            self.dim,
            self.rotation_seed,
            self.rerank_factor,
            &self.generation,
        );
        expected == self.rvf_witness
    }

    /// Serialize to JSON (the on-disk `table.rulake.json` form).
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::RuLakeError::InvalidParameter(format!("bundle serialize: {e}")))
    }

    /// Deserialize from JSON. Fails if the format version is newer than
    /// this binary supports.
    pub fn from_json(s: &str) -> crate::Result<Self> {
        let b: Self = serde_json::from_str(s)
            .map_err(|e| crate::RuLakeError::InvalidParameter(format!("bundle parse: {e}")))?;
        if b.format_version > Self::FORMAT_VERSION {
            return Err(crate::RuLakeError::InvalidParameter(format!(
                "bundle format_version={} newer than this binary supports ({})",
                b.format_version,
                Self::FORMAT_VERSION
            )));
        }
        Ok(b)
    }

    pub fn with_pii_policy(mut self, p: impl Into<String>) -> Self {
        self.pii_policy = Some(p.into());
        self
    }

    pub fn with_lineage_id(mut self, id: impl Into<String>) -> Self {
        self.lineage_id = Some(id.into());
        self
    }
}

/// SHAKE-256(32) over a stable concatenation of the bundle fields.
/// Deliberately domain-separated by a fixed prefix so witnesses can't
/// collide with other uses of SHAKE.
fn compute_witness(
    data_ref: &str,
    dim: usize,
    rotation_seed: u64,
    rerank_factor: usize,
    generation: &Generation,
) -> String {
    use sha3::{
        digest::{ExtendableOutput, Update},
        Shake256,
    };
    let mut h = Shake256::default();
    h.update(b"rulake-bundle-witness-v1|");
    h.update(&(data_ref.len() as u64).to_le_bytes());
    h.update(data_ref.as_bytes());
    h.update(b"|");
    h.update(&(dim as u64).to_le_bytes());
    h.update(&rotation_seed.to_le_bytes());
    h.update(&(rerank_factor as u64).to_le_bytes());
    h.update(b"|");
    let g = generation.hash_bytes();
    h.update(&(g.len() as u64).to_le_bytes());
    h.update(&g);
    let mut reader = h.finalize_xof();
    let mut out = [0u8; 32];
    use sha3::digest::XofReader;
    reader.read(&mut out);
    hex::encode(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn witness_is_deterministic() {
        let a = RuLakeBundle::new("gs://bucket/x.parquet", 128, 42, 20, Generation::Num(7));
        let b = RuLakeBundle::new("gs://bucket/x.parquet", 128, 42, 20, Generation::Num(7));
        assert_eq!(a.rvf_witness, b.rvf_witness);
    }

    #[test]
    fn witness_changes_on_any_field() {
        let a = RuLakeBundle::new("gs://bucket/x.parquet", 128, 42, 20, Generation::Num(7));
        // Different data_ref.
        let b = RuLakeBundle::new("gs://bucket/y.parquet", 128, 42, 20, Generation::Num(7));
        assert_ne!(a.rvf_witness, b.rvf_witness);
        // Different dim.
        let c = RuLakeBundle::new("gs://bucket/x.parquet", 256, 42, 20, Generation::Num(7));
        assert_ne!(a.rvf_witness, c.rvf_witness);
        // Different seed.
        let d = RuLakeBundle::new("gs://bucket/x.parquet", 128, 43, 20, Generation::Num(7));
        assert_ne!(a.rvf_witness, d.rvf_witness);
        // Different rerank factor.
        let e = RuLakeBundle::new("gs://bucket/x.parquet", 128, 42, 5, Generation::Num(7));
        assert_ne!(a.rvf_witness, e.rvf_witness);
        // Different generation.
        let f = RuLakeBundle::new("gs://bucket/x.parquet", 128, 42, 20, Generation::Num(8));
        assert_ne!(a.rvf_witness, f.rvf_witness);
    }

    #[test]
    fn witness_is_length_prefixed() {
        // Regression against "a|b" colliding with "ab|" on concat-only
        // witness schemes. Length prefixes should prevent that.
        let g1 = Generation::Opaque("a|b".to_string());
        let g2 = Generation::Opaque("ab|".to_string());
        let a = RuLakeBundle::new("x", 1, 0, 1, g1);
        let b = RuLakeBundle::new("x", 1, 0, 1, g2);
        assert_ne!(a.rvf_witness, b.rvf_witness);
    }

    #[test]
    fn opaque_generation_serialization_roundtrip() {
        let bundle = RuLakeBundle::new(
            "iceberg://catalog/db/table",
            384,
            0xdead_beef,
            5,
            Generation::Opaque("01JCX7NK6G5R9G1YZ7QH".to_string()),
        );
        let s = bundle.to_json().unwrap();
        let parsed = RuLakeBundle::from_json(&s).unwrap();
        assert_eq!(parsed.rvf_witness, bundle.rvf_witness);
        assert!(parsed.verify_witness());
    }

    #[test]
    fn verify_witness_catches_tamper() {
        let mut b = RuLakeBundle::new("x", 128, 1, 20, Generation::Num(1));
        assert!(b.verify_witness());
        // Tamper with a field without recomputing the witness.
        b.dim = 256;
        assert!(!b.verify_witness(), "witness verify must catch dim tamper");
    }

    #[test]
    fn format_version_downgrade_rejected() {
        let good = RuLakeBundle::new("x", 128, 1, 20, Generation::Num(1));
        let mut s = good.to_json().unwrap();
        // Simulate a future bundle with format_version: 99.
        s = s.replace("\"format_version\": 1", "\"format_version\": 99");
        let err = RuLakeBundle::from_json(&s).unwrap_err();
        assert!(matches!(err, crate::RuLakeError::InvalidParameter(_)));
    }
}
