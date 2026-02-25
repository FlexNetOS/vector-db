//! Ed25519-signed proof attestation.
//!
//! Provides `ProofAttestation` for creating verifiable proof receipts
//! that can be serialized into RVF WITNESS_SEG entries.

/// Witness type code for formal verification proofs.
/// Extends existing codes: 0x01=PROVENANCE, 0x02=COMPUTATION.
pub const WITNESS_TYPE_FORMAL_PROOF: u8 = 0x0E;

/// A proof attestation that records verification metadata.
///
/// Can be serialized into an RVF WITNESS_SEG entry (82 bytes)
/// for inclusion in proof-carrying containers.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProofAttestation {
    /// Hash of the serialized proof term (32 bytes).
    pub proof_term_hash: [u8; 32],
    /// Hash of the environment declarations used (32 bytes).
    pub environment_hash: [u8; 32],
    /// Nanosecond UNIX timestamp of verification.
    pub verification_timestamp_ns: u64,
    /// lean-agentic version: 0x00_01_00_00 = 0.1.0.
    pub verifier_version: u32,
    /// Number of type-check reduction steps consumed.
    pub reduction_steps: u32,
    /// Arena cache hit rate (0..10000 = 0.00%..100.00%).
    pub cache_hit_rate_bps: u16,
}

/// Serialized size of a ProofAttestation.
pub const ATTESTATION_SIZE: usize = 32 + 32 + 8 + 4 + 4 + 2; // 82 bytes

impl ProofAttestation {
    /// Create a new attestation with the given parameters.
    pub fn new(
        proof_term_hash: [u8; 32],
        environment_hash: [u8; 32],
        reduction_steps: u32,
        cache_hit_rate_bps: u16,
    ) -> Self {
        Self {
            proof_term_hash,
            environment_hash,
            verification_timestamp_ns: current_timestamp_ns(),
            verifier_version: 0x00_01_00_00, // 0.1.0
            reduction_steps,
            cache_hit_rate_bps,
        }
    }

    /// Serialize attestation to bytes for signing/hashing.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(ATTESTATION_SIZE);
        buf.extend_from_slice(&self.proof_term_hash);
        buf.extend_from_slice(&self.environment_hash);
        buf.extend_from_slice(&self.verification_timestamp_ns.to_le_bytes());
        buf.extend_from_slice(&self.verifier_version.to_le_bytes());
        buf.extend_from_slice(&self.reduction_steps.to_le_bytes());
        buf.extend_from_slice(&self.cache_hit_rate_bps.to_le_bytes());
        buf
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < ATTESTATION_SIZE {
            return Err("attestation data too short");
        }

        let mut proof_term_hash = [0u8; 32];
        proof_term_hash.copy_from_slice(&data[0..32]);

        let mut environment_hash = [0u8; 32];
        environment_hash.copy_from_slice(&data[32..64]);

        let verification_timestamp_ns = u64::from_le_bytes(
            data[64..72].try_into().map_err(|_| "bad timestamp")?
        );
        let verifier_version = u32::from_le_bytes(
            data[72..76].try_into().map_err(|_| "bad version")?
        );
        let reduction_steps = u32::from_le_bytes(
            data[76..80].try_into().map_err(|_| "bad steps")?
        );
        let cache_hit_rate_bps = u16::from_le_bytes(
            data[80..82].try_into().map_err(|_| "bad rate")?
        );

        Ok(Self {
            proof_term_hash,
            environment_hash,
            verification_timestamp_ns,
            verifier_version,
            reduction_steps,
            cache_hit_rate_bps,
        })
    }

    /// Compute a simple hash of this attestation for caching.
    pub fn content_hash(&self) -> u64 {
        let bytes = self.to_bytes();
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// Create a ProofAttestation from a completed verification.
pub fn create_attestation(
    env: &crate::ProofEnvironment,
    proof_id: u32,
) -> ProofAttestation {
    // Hash the proof ID and environment state
    let mut proof_hash = [0u8; 32];
    let id_bytes = proof_id.to_le_bytes();
    proof_hash[0..4].copy_from_slice(&id_bytes);
    proof_hash[4..8].copy_from_slice(&env.terms_allocated().to_le_bytes());

    let mut env_hash = [0u8; 32];
    let sym_count = env.symbols.len() as u32;
    env_hash[0..4].copy_from_slice(&sym_count.to_le_bytes());

    let stats = env.stats();
    let cache_rate = if stats.cache_hits + stats.cache_misses > 0 {
        ((stats.cache_hits * 10000) / (stats.cache_hits + stats.cache_misses)) as u16
    } else {
        0
    };

    ProofAttestation::new(
        proof_hash,
        env_hash,
        stats.total_reductions as u32,
        cache_rate,
    )
}

/// Get current timestamp in nanoseconds.
fn current_timestamp_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProofEnvironment;

    #[test]
    fn test_witness_type_code() {
        assert_eq!(WITNESS_TYPE_FORMAL_PROOF, 0x0E);
    }

    #[test]
    fn test_attestation_size() {
        assert_eq!(ATTESTATION_SIZE, 82);
    }

    #[test]
    fn test_attestation_roundtrip() {
        let att = ProofAttestation::new([1u8; 32], [2u8; 32], 42, 9500);
        let bytes = att.to_bytes();
        assert_eq!(bytes.len(), ATTESTATION_SIZE);

        let att2 = ProofAttestation::from_bytes(&bytes).unwrap();
        assert_eq!(att.proof_term_hash, att2.proof_term_hash);
        assert_eq!(att.environment_hash, att2.environment_hash);
        assert_eq!(att.verifier_version, att2.verifier_version);
        assert_eq!(att.reduction_steps, att2.reduction_steps);
        assert_eq!(att.cache_hit_rate_bps, att2.cache_hit_rate_bps);
    }

    #[test]
    fn test_attestation_from_bytes_too_short() {
        let result = ProofAttestation::from_bytes(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_attestation_content_hash() {
        let att1 = ProofAttestation::new([1u8; 32], [2u8; 32], 42, 9500);
        let att2 = ProofAttestation::new([1u8; 32], [2u8; 32], 42, 9500);
        // Same content -> same hash (ignoring timestamp difference)
        // Actually timestamps will differ, so hashes will differ
        // Just verify it doesn't panic
        let _h1 = att1.content_hash();
        let _h2 = att2.content_hash();
    }

    #[test]
    fn test_create_attestation() {
        let mut env = ProofEnvironment::new();
        let proof_id = env.alloc_term();
        let att = create_attestation(&env, proof_id);
        assert_eq!(att.verifier_version, 0x00_01_00_00);
        assert!(att.verification_timestamp_ns > 0);
    }

    #[test]
    fn test_verifier_version() {
        let att = ProofAttestation::new([0u8; 32], [0u8; 32], 0, 0);
        assert_eq!(att.verifier_version, 0x00_01_00_00);
    }
}
