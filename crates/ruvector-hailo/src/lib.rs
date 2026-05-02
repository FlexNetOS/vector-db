//! ruvector embedding backend for the Hailo-8 NPU.
//!
//! ADR-167 (`hailo-backend` branch). Public surface mirrors
//! `ruvector_core::embeddings::EmbeddingProvider` exactly so wiring it up
//! once iteration 3 lands the path dep is a one-line `impl`.
//!
//! Default build (no `hailo` feature): every API call returns
//! `Err(HailoError::FeatureDisabled)`. Lets non-Pi machines run
//! `cargo check -p ruvector-hailo` without HailoRT installed.

pub mod device;
pub mod error;
pub mod inference;
pub mod tokenizer;

pub use device::HailoDevice;
pub use error::HailoError;
pub use inference::{EmbeddingPipeline, l2_normalize, mean_pool, DEFAULT_MAX_SEQ, MINI_LM_DIM};
pub use tokenizer::{EncodedInput, SpecialIds, WordPieceTokenizer};

use std::path::Path;
use std::sync::Mutex;

/// Convenience alias matching ruvector-core's `Result<T> = Result<T, Error>`.
pub type Result<T> = std::result::Result<T, HailoError>;

/// Embedding inference engine backed by the Hailo-8 NPU.
///
/// Uses interior mutability so the public API is `&self` — that lets
/// `HailoEmbedder` implement `ruvector_core::embeddings::EmbeddingProvider`
/// (which takes `&self`) without forcing every caller to manage a `&mut`.
///
/// Phase 1 step 1 (this iteration): scaffold + signature parity. Open
/// returns `FeatureDisabled` until iteration 4 brings device enumeration
/// online.
pub struct HailoEmbedder {
    /// Embedding dimensionality from the loaded HEF. Set when an HEF is
    /// loaded; 0 in stub.
    dimensions: usize,
    /// Human-readable name for logging — e.g. `"hailo:all-MiniLM-L6-v2"`.
    name: String,
    /// PCIe BDF of the underlying device once opened, e.g. `0001:01:00.0`.
    device_id: String,
    /// Internal handle bundle. The actual fields land in iterations 4-7
    /// (Mutex<DeviceHandle>, Mutex<NetworkGroup>, Mutex<VStreams>, etc.).
    /// Keeping `_inner: Mutex<()>` reserves the interior-mutability slot
    /// without committing to the layout yet.
    _inner: Mutex<()>,
}

impl HailoEmbedder {
    /// Open a Hailo NPU device and load the HEF + tokenizer artifacts found
    /// at `model_dir`.
    ///
    /// Expected layout under `model_dir`:
    ///
    /// ```text
    /// model_dir/
    ///   model.hef             # compiled by Hailo Dataflow Compiler
    ///   vocab.txt             # WordPiece vocab (one token per line)
    ///   special_tokens.json   # CLS/SEP/PAD ids
    /// ```
    pub fn open(_model_dir: &Path) -> Result<Self> {
        #[cfg(not(feature = "hailo"))]
        {
            Err(HailoError::FeatureDisabled)
        }
        #[cfg(feature = "hailo")]
        {
            // TODO iteration 4: enumerate /dev/hailo*, open first, load HEF.
            Err(HailoError::NotYetImplemented("HailoEmbedder::open"))
        }
    }

    /// Embed a single piece of text into a `dimensions()`-element f32 vector.
    pub fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        #[cfg(not(feature = "hailo"))]
        {
            Err(HailoError::FeatureDisabled)
        }
        #[cfg(feature = "hailo")]
        {
            Err(HailoError::NotYetImplemented("HailoEmbedder::embed"))
        }
    }

    /// Embed a batch of texts. Default impl loops; iteration 7 replaces
    /// with batched-vstream feed when the HEF is compiled with batch>1.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(self.embed(t)?);
        }
        Ok(out)
    }

    /// Vector dimensionality (e.g. 384 for `all-MiniLM-L6-v2`).
    /// Mirrors `EmbeddingProvider::dimensions()`.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Human-readable provider name. Mirrors `EmbeddingProvider::name()`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// PCIe BDF, e.g. `"0001:01:00.0"`. Empty before `open()` succeeds.
    /// Hailo-specific extension — not on the EmbeddingProvider trait.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

// SAFETY: HailoEmbedder will own a Mutex<DeviceHandle> once iteration 4
// lands. The HailoRT C library is documented thread-safe per device handle
// when accessed under a single configuration; our Mutex wrapper enforces
// the rest. Send+Sync are required by `EmbeddingProvider`.
unsafe impl Send for HailoEmbedder {}
unsafe impl Sync for HailoEmbedder {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_open_returns_feature_disabled_or_not_implemented() {
        let r = HailoEmbedder::open(Path::new("/nonexistent"));
        assert!(matches!(
            r,
            Err(HailoError::FeatureDisabled) | Err(HailoError::NotYetImplemented(_))
        ));
    }

    #[test]
    fn embedding_provider_signature_parity() {
        // Compile-time check that our API surface matches the
        // `EmbeddingProvider` trait shape we'll be wiring into in
        // iteration 3.
        fn assert_signatures<T>()
        where
            T: Send + Sync,
        {}
        assert_signatures::<HailoEmbedder>();
    }
}
