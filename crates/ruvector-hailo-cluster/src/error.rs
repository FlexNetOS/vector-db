//! Cluster-side errors. Maps cleanly onto `ruvector_core::EmbeddingError`
//! once iteration 14 brings the path dep.

use thiserror::Error;

/// Errors surfaced by `HailoClusterEmbedder` and its supporting modules.
/// Distinguishes worker-level failures (`Transport`, `FingerprintMismatch`,
/// `DimMismatch`) from coordinator-level failures (`NoWorkers`,
/// `AllWorkersFailed`) so callers can map to user-facing categories.
#[derive(Debug, Error)]
pub enum ClusterError {
    /// Coordinator built with zero workers.
    #[error("HailoClusterEmbedder requires at least one worker")]
    NoWorkers,

    /// Iteration N hasn't landed for this code path.
    #[error("not yet implemented: {0}")]
    NotYetImplemented(/// Description of the missing functionality.
                      &'static str),

    /// Every worker we tried failed (after retry budget exhausted).
    #[error("all workers failed: {0}")]
    AllWorkersFailed(/// Aggregated reason — typically the last seen error.
                     String),

    /// Worker refused due to model fingerprint mismatch — never silently
    /// fan out across a heterogeneous fleet.
    #[error("worker {worker} fingerprint {actual} != expected {expected}")]
    FingerprintMismatch {
        /// Name of the worker (per `WorkerEndpoint::name`).
        worker: String,
        /// Fingerprint string the worker reports.
        actual: String,
        /// Fingerprint string the coordinator was configured to require.
        expected: String,
    },

    /// Transport-layer failure (gRPC connect / RPC error).
    #[error("transport error to {worker}: {reason}")]
    Transport {
        /// Worker name from the transport call site.
        worker: String,
        /// Free-form failure reason — gRPC status, IO error, etc.
        reason: String,
    },

    /// Worker returned a vector with the wrong dimensionality.
    #[error("worker {worker}: expected dim {expected}, got {actual}")]
    DimMismatch {
        /// Worker name from the dispatch call site.
        worker: String,
        /// Dimensionality the coordinator was configured to accept.
        expected: usize,
        /// Dimensionality the worker actually returned.
        actual: usize,
    },
}
