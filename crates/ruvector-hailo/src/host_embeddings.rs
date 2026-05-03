//! Host-side BERT embeddings lookup — feeds the HEF encoder pipeline.
//!
//! ADR-176 P2 (`hailo-backend`, iter 160). The iter-156b ONNX export
//! deliberately strips the embedding `Gather` op (Hailo can't
//! represent it). This module replaces it host-side, computing
//! `word_embeddings + position_embeddings + token_type_embeddings`
//! followed by `LayerNorm(γ, β, ε)` from `model.safetensors`.
//!
//! The output `[1, seq, hidden]` FP32 tensor feeds directly into
//! `HefPipeline::forward`.
//!
//! candle's own `BertEmbeddings` is private to candle-transformers;
//! we reimplement using its public `Embedding` + `LayerNorm` building
//! blocks. ~80 LOC; no new deps beyond what `cpu-fallback` already
//! pulls in.
//!
//! **Lifetime**: load once at worker startup; clone-free per-embed
//! call. `Send + Sync` because the underlying tensors are immutable
//! after load.

#![cfg(feature = "cpu-fallback")]

use crate::error::HailoError;
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::{embedding, layer_norm, Embedding, LayerNorm, VarBuilder};
use candle_transformers::models::bert::Config;
use std::path::Path;

/// Host-side BERT embedding lookup. Drop-in replacement for the
/// `Gather + Add + LayerNorm` block at the start of the HF BERT graph
/// that we strip from the ONNX export so Hailo can compile the encoder.
pub struct HostEmbeddings {
    word_embeddings: Embedding,
    position_embeddings: Embedding,
    token_type_embeddings: Embedding,
    layer_norm: LayerNorm,
    device: Device,
}

impl HostEmbeddings {
    /// Load the three embedding tables + LayerNorm γ/β from
    /// `model.safetensors` in `model_dir`. Reads `config.json` for
    /// vocab sizes + hidden_size + eps.
    pub fn open(model_dir: &Path) -> Result<Self, HailoError> {
        let weights_path = model_dir.join("model.safetensors");
        let config_path = model_dir.join("config.json");

        if !weights_path.exists() {
            return Err(HailoError::BadModelDir {
                path: model_dir.display().to_string(),
                what: "model.safetensors",
            });
        }
        if !config_path.exists() {
            return Err(HailoError::BadModelDir {
                path: model_dir.display().to_string(),
                what: "config.json",
            });
        }

        let config_str = std::fs::read_to_string(&config_path).map_err(|e| {
            HailoError::Tokenizer(format!("read config.json: {}", e))
        })?;
        let config: Config = serde_json::from_str(&config_str).map_err(|e| {
            HailoError::Tokenizer(format!("parse config.json: {}", e))
        })?;

        let device = Device::Cpu;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[&weights_path], DType::F32, &device)
                .map_err(|e| HailoError::Tokenizer(format!("load safetensors: {}", e)))?
        };

        // BERT puts the embedding tables under `embeddings.*` in
        // sentence-transformers' safetensors layout. The same path
        // candle's BertEmbeddings::load uses internally — verified by
        // grepping the cpu-fallback path that loads BertModel cleanly.
        let emb_vb = vb.pp("embeddings");

        let word_embeddings = embedding(
            config.vocab_size,
            config.hidden_size,
            emb_vb.pp("word_embeddings"),
        )
        .map_err(|e| {
            HailoError::Tokenizer(format!("load word_embeddings: {}", e))
        })?;
        let position_embeddings = embedding(
            config.max_position_embeddings,
            config.hidden_size,
            emb_vb.pp("position_embeddings"),
        )
        .map_err(|e| {
            HailoError::Tokenizer(format!("load position_embeddings: {}", e))
        })?;
        let token_type_embeddings = embedding(
            config.type_vocab_size,
            config.hidden_size,
            emb_vb.pp("token_type_embeddings"),
        )
        .map_err(|e| {
            HailoError::Tokenizer(format!("load token_type_embeddings: {}", e))
        })?;
        let layer_norm = layer_norm(
            config.hidden_size,
            config.layer_norm_eps,
            emb_vb.pp("LayerNorm"),
        )
        .map_err(|e| HailoError::Tokenizer(format!("load LayerNorm: {}", e)))?;

        Ok(Self {
            word_embeddings,
            position_embeddings,
            token_type_embeddings,
            layer_norm,
            device,
        })
    }

    /// Run `input_ids` (Vec<i64>, length `seq_len`) through the embedding
    /// lookup. `token_type_ids` defaults to all-zeros for the
    /// single-segment case. Returns flat FP32 of length
    /// `seq_len * hidden_size` in row-major `[seq, hidden]` order —
    /// directly feedable into `HefPipeline::forward`.
    pub fn forward(&self, input_ids: &[i64]) -> Result<Vec<f32>, HailoError> {
        let seq_len = input_ids.len();
        if seq_len == 0 {
            return Err(HailoError::Tokenizer("empty input_ids".to_string()));
        }

        let input_t = Tensor::new(input_ids, &self.device)
            .map_err(|e| HailoError::Tokenizer(format!("input tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| HailoError::Tokenizer(format!("unsqueeze: {}", e)))?;
        let type_t = Tensor::zeros((1, seq_len), DType::I64, &self.device)
            .map_err(|e| HailoError::Tokenizer(format!("type tensor: {}", e)))?;
        let position_ids: Vec<i64> = (0..seq_len as i64).collect();
        let pos_t = Tensor::new(position_ids.as_slice(), &self.device)
            .map_err(|e| HailoError::Tokenizer(format!("pos tensor: {}", e)))?
            .unsqueeze(0)
            .map_err(|e| HailoError::Tokenizer(format!("pos unsqueeze: {}", e)))?;

        let word = self.word_embeddings.forward(&input_t).map_err(|e| {
            HailoError::Tokenizer(format!("word_embeddings forward: {}", e))
        })?;
        let pos = self.position_embeddings.forward(&pos_t).map_err(|e| {
            HailoError::Tokenizer(format!("position_embeddings forward: {}", e))
        })?;
        let typ = self.token_type_embeddings.forward(&type_t).map_err(|e| {
            HailoError::Tokenizer(format!("token_type_embeddings forward: {}", e))
        })?;

        let summed = (&word + &pos)
            .and_then(|s| s + &typ)
            .map_err(|e| HailoError::Tokenizer(format!("emb sum: {}", e)))?;
        let normed = self
            .layer_norm
            .forward(&summed)
            .map_err(|e| HailoError::Tokenizer(format!("LayerNorm: {}", e)))?;

        // Squeeze batch dim, flatten to row-major [seq * hidden].
        let flat = normed
            .squeeze(0)
            .and_then(|t| t.flatten_all())
            .map_err(|e| HailoError::Tokenizer(format!("flatten: {}", e)))?
            .to_vec1::<f32>()
            .map_err(|e| HailoError::Tokenizer(format!("to_vec1: {}", e)))?;

        Ok(flat)
    }
}

// SAFETY: candle Tensors hold immutable refs into the mmap'd
// safetensors. forward() creates new tensors but never mutates the
// loaded weights. Send + Sync hold for the same reasons CpuEmbedder's
// Inner is Send (after we wrapped it in Mutex per Pool slot).
unsafe impl Send for HostEmbeddings {}
unsafe impl Sync for HostEmbeddings {}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_dir() -> Option<std::path::PathBuf> {
        std::env::var_os("RUVECTOR_CPU_FALLBACK_MODEL_DIR")
            .map(std::path::PathBuf::from)
    }

    #[test]
    fn host_embeddings_load_and_forward_match_shape() {
        let Some(dir) = model_dir() else {
            return; // skip when no model available
        };
        let emb = HostEmbeddings::open(&dir).unwrap();
        let input_ids: Vec<i64> = (100..228).collect(); // 128 tokens
        let out = emb.forward(&input_ids).unwrap();
        assert_eq!(out.len(), 128 * 384, "expected [seq * hidden] = 128*384");
        assert!(out.iter().all(|x| x.is_finite()));
    }
}
