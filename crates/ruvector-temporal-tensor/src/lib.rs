//! Temporal Tensor Compression with Tiered Quantization
//!
//! Implements ADR-017: groupwise symmetric quantization with temporal segment
//! reuse and access-pattern-driven tier selection (8/7/5/3 bit).
//!
//! # Architecture
//!
//! ```text
//! f32 frame → tier_policy → quantizer → bitpack → segment
//! segment → bitpack → quantizer → f32 output
//! ```
//!
//! # Compression Ratios
//!
//! | Tier | Bits | Ratio vs f32 | Use Case |
//! |------|------|-------------|----------|
//! | Hot  | 8    | ~4.0x       | Frequently accessed tensors |
//! | Warm | 7    | ~4.57x      | Moderately accessed |
//! | Warm | 5    | ~6.4x       | Aggressively compressed warm |
//! | Cold | 3    | ~10.67x     | Rarely accessed |
//!
//! # Zero Dependencies
//!
//! This crate has no external dependencies, making it fully WASM-compatible.

pub mod bitpack;
pub mod compressor;
pub mod f16;
pub mod quantizer;
pub mod segment;
pub mod tier_policy;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use compressor::TemporalTensorCompressor;
pub use tier_policy::TierPolicy;
