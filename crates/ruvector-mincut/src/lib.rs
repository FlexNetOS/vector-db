//! # RuVector MinCut
//!
//! Subpolynomial-time dynamic minimum cut algorithm with real-time monitoring.
//!
//! This crate provides efficient algorithms for maintaining minimum cuts in
//! dynamic graphs with edge insertions and deletions.
//!
//! ## Features
//!
//! - **Exact Algorithm**: O(n^{o(1)}) amortized update time for cuts up to 2^{O((log n)^{3/4})}
//! - **Approximate Algorithm**: (1+ε)-approximate cuts via graph sparsification
//! - **Real-Time Monitoring**: Event-driven notifications with configurable thresholds
//! - **Thread-Safe**: Concurrent reads with exclusive writes
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ruvector_mincut::{MinCutBuilder, DynamicMinCut};
//!
//! // Create a dynamic minimum cut structure
//! let mut mincut = MinCutBuilder::new()
//!     .exact()
//!     .with_edges(vec![(1, 2, 1.0), (2, 3, 1.0), (3, 1, 1.0)])
//!     .build()
//!     .expect("Failed to build");
//!
//! // Query the minimum cut
//! println!("Min cut: {}", mincut.min_cut_value());
//!
//! // Insert a new edge
//! mincut.insert_edge(3, 4, 1.0).expect("Insert failed");
//!
//! // Delete an existing edge
//! let _ = mincut.delete_edge(2, 3);
//! ```
//!
//! ## Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`graph`]: Dynamic graph representation with efficient operations
//! - [`algorithm`]: Core minimum cut algorithms (exact and approximate)
//! - [`tree`]: Hierarchical decomposition for subpolynomial updates
//! - [`linkcut`]: Link-cut trees for dynamic connectivity
//! - [`euler`]: Euler tour trees for tree operations
//! - [`sparsify`]: Graph sparsification for approximate cuts
//! - [`expander`]: Expander decomposition for subpolynomial updates
//! - `monitoring`: Real-time event monitoring (feature-gated)
//!
//! ## Feature Flags
//!
//! - `exact` - Exact minimum cut algorithm (enabled by default)
//! - `approximate` - (1+ε)-approximate algorithm (enabled by default)
//! - `monitoring` - Real-time monitoring with callbacks (optional)
//! - `integration` - GraphDB integration (optional)
//! - `simd` - SIMD optimizations (optional)
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```rust
//! use ruvector_mincut::prelude::*;
//!
//! let mut mincut = MinCutBuilder::new()
//!     .with_edges(vec![
//!         (1, 2, 1.0),
//!         (2, 3, 1.0),
//!         (3, 1, 1.0),
//!     ])
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(mincut.min_cut_value(), 2.0);
//! ```
//!
//! ### Approximate Algorithm
//!
//! ```rust
//! use ruvector_mincut::prelude::*;
//!
//! let mincut = MinCutBuilder::new()
//!     .approximate(0.1) // 10% approximation
//!     .with_edges(vec![(1, 2, 1.0), (2, 3, 1.0)])
//!     .build()
//!     .unwrap();
//!
//! let result = mincut.min_cut();
//! assert!(!result.is_exact);
//! assert_eq!(result.approximation_ratio, 1.1);
//! ```
//!
//! ### Real-Time Monitoring
//!
//! ```rust,ignore
//! #[cfg(feature = "monitoring")]
//! use ruvector_mincut::{MinCutBuilder, MonitorBuilder, EventType};
//!
//! let mut mincut = MinCutBuilder::new()
//!     .with_edges(vec![(1, 2, 1.0)])
//!     .build()
//!     .unwrap();
//!
//! let monitor = MonitorBuilder::new()
//!     .threshold("low", 0.5, true)
//!     .on_event(|event| {
//!         println!("Cut changed: {:?}", event.event_type);
//!     })
//!     .build();
//!
//! // Monitor will fire callbacks on updates
//! mincut.insert_edge(2, 3, 1.0).unwrap();
//! ```

#![deny(missing_docs)]
#![cfg_attr(not(feature = "wasm"), deny(unsafe_code))]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

// Core modules
pub mod error;
pub mod graph;
pub mod linkcut;
pub mod euler;
pub mod tree;
pub mod witness;
pub mod algorithm;
pub mod sparsify;
pub mod expander;
pub mod localkcut;
pub mod connectivity;
pub mod instance;
pub mod wrapper;
pub mod certificate;
pub mod fragment;
pub mod fragmentation;
pub mod cluster;
pub mod compact;
pub mod parallel;
pub mod pool;
pub mod integration;

/// Spiking Neural Network integration for deep MinCut optimization.
///
/// This module implements a six-layer integration architecture combining
/// neuromorphic computing with subpolynomial graph algorithms:
///
/// 1. **Temporal Attractors**: Energy landscapes for graph optimization
/// 2. **Strange Loop**: Self-modifying meta-cognitive protocols
/// 3. **Causal Discovery**: Spike-timing based inference
/// 4. **Time Crystal CPG**: Central pattern generators for coordination
/// 5. **Morphogenetic Networks**: Bio-inspired self-organizing growth
/// 6. **Neural Optimizer**: Reinforcement learning on graph structures
///
/// ## Quick Start
///
/// ```rust,no_run
/// use ruvector_mincut::snn::{CognitiveMinCutEngine, EngineConfig, OperationMode};
/// use ruvector_mincut::graph::DynamicGraph;
///
/// // Create a graph
/// let graph = DynamicGraph::new();
/// graph.insert_edge(0, 1, 1.0).unwrap();
/// graph.insert_edge(1, 2, 1.0).unwrap();
///
/// // Create the cognitive engine
/// let config = EngineConfig::default();
/// let mut engine = CognitiveMinCutEngine::new(graph, config);
///
/// // Run optimization
/// engine.set_mode(OperationMode::Optimize);
/// let spikes = engine.run(100);
/// ```
pub mod snn;

/// Subpolynomial-time dynamic minimum cut algorithm.
///
/// This module implements the December 2024 breakthrough achieving n^{o(1)} update time.
/// Integrates multi-level hierarchy, deterministic LocalKCut, and fragmenting algorithm.
pub mod subpolynomial;

// Internal modules
mod core;

// Optional feature-gated modules
#[cfg(feature = "monitoring")]
pub mod monitoring;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-exports for convenient access
pub use error::{MinCutError, Result};
pub use graph::{DynamicGraph, Edge, GraphStats, VertexId, EdgeId, Weight};
pub use algorithm::{DynamicMinCut, MinCutBuilder, MinCutConfig, MinCutResult, AlgorithmStats};
pub use algorithm::approximate::{ApproxMinCut, ApproxMinCutConfig, ApproxMinCutResult, ApproxMinCutStats};
pub use tree::{HierarchicalDecomposition, DecompositionNode, LevelInfo};
pub use witness::{WitnessTree, LazyWitnessTree, EdgeWitness};
pub use linkcut::LinkCutTree;
pub use euler::EulerTourTree;
pub use sparsify::{SparseGraph, SparsifyConfig};
pub use expander::{ExpanderDecomposition, ExpanderComponent, Conductance};
pub use localkcut::{
    LocalKCut, LocalCutResult, EdgeColor, ColorMask, ForestPacking,
    LocalKCutQuery, LocalKCutResult as PaperLocalKCutResult, LocalKCutOracle,
    DeterministicLocalKCut, DeterministicFamilyGenerator,
};
pub use connectivity::DynamicConnectivity;
pub use connectivity::polylog::{PolylogConnectivity, PolylogStats};
pub use instance::{ProperCutInstance, InstanceResult, WitnessHandle, StubInstance, BoundedInstance};
pub use wrapper::MinCutWrapper;
pub use certificate::{
    CutCertificate, CertificateError, CertLocalKCutQuery, LocalKCutResponse,
    LocalKCutResultSummary, UpdateTrigger, UpdateType, AuditLogger,
    AuditEntry, AuditEntryType, AuditData,
};
pub use cluster::{ClusterHierarchy, Cluster};
pub use cluster::hierarchy::{
    ThreeLevelHierarchy, Expander, Precluster, HierarchyCluster,
    MirrorCut, HierarchyConfig, HierarchyStats,
};
pub use fragment::{Fragment, FragmentResult, FragmentingAlgorithm};
pub use fragmentation::{
    Fragmentation, FragmentationConfig, TrimResult,
    Fragment as FragmentationFragment,
};
pub use compact::{
    BitSet256, CompactEdge, CompactWitness, CompactAdjacency, CompactCoreState,
    CoreResult, CompactVertexId, CompactEdgeId, MAX_VERTICES_PER_CORE, MAX_EDGES_PER_CORE,
};
pub use parallel::{
    NUM_CORES, RANGES_PER_CORE, TOTAL_RANGES, RANGE_FACTOR,
    CoreStrategy, CoreMessage, WorkItem, SharedCoordinator,
    CoreDistributor, CoreExecutor, ResultAggregator,
    compute_core_range,
};
pub use integration::{
    RuVectorGraphAnalyzer, CommunityDetector, GraphPartitioner,
};
pub use subpolynomial::{
    SubpolynomialMinCut, SubpolyConfig, RecourseStats,
    MinCutQueryResult, HierarchyStatistics, LevelExpander, HierarchyLevel,
};

// SNN Integration re-exports
pub use snn::{
    // Core SNN types
    LIFNeuron, NeuronState, NeuronConfig, SpikeTrain,
    Synapse, STDPConfig, SynapseMatrix,
    SpikingNetwork, NetworkConfig, LayerConfig,
    // Layer 1: Attractors
    AttractorDynamics, EnergyLandscape, AttractorConfig,
    // Layer 2: Strange Loop
    MetaCognitiveMinCut, MetaAction, MetaLevel, StrangeLoopConfig,
    // Layer 3: Causal Discovery
    CausalDiscoverySNN, CausalGraph, CausalRelation, CausalConfig,
    // Layer 4: Time Crystal
    TimeCrystalCPG, OscillatorNeuron, PhaseTopology, CPGConfig,
    // Layer 5: Morphogenetic
    MorphogeneticSNN, GrowthRules, TuringPattern, MorphConfig,
    // Layer 6: Neural Optimizer
    NeuralGraphOptimizer, PolicySNN, ValueNetwork, OptimizerConfig, OptimizationResult,
    // Unified Engine
    CognitiveMinCutEngine, EngineConfig, EngineMetrics,
    // Utilities
    Spike, SimTime, SNNMinCutConfig,
};

#[cfg(feature = "agentic")]
pub use integration::AgenticAnalyzer;

#[cfg(feature = "monitoring")]
pub use monitoring::{
    MinCutMonitor, MonitorBuilder, MonitorConfig, MinCutEvent,
    EventType, Threshold, MonitorMetrics
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Prelude module for convenient imports
///
/// Import this module to get all the commonly used types and traits:
///
/// ```rust
/// use ruvector_mincut::prelude::*;
///
/// let mincut = MinCutBuilder::new()
///     .exact()
///     .build()
///     .unwrap();
/// ```
pub mod prelude {
    //! Prelude module with commonly used types

    pub use crate::{
        DynamicMinCut, MinCutBuilder, MinCutConfig, MinCutResult, ApproxMinCut, ApproxMinCutConfig,
        DynamicGraph, Edge, VertexId, EdgeId, Weight,
        MinCutError, Result,
        AlgorithmStats,
        ExpanderDecomposition, ExpanderComponent, Conductance,
        LocalKCut, LocalCutResult, EdgeColor, ColorMask, ForestPacking,
        LocalKCutQuery, PaperLocalKCutResult, LocalKCutOracle,
        DeterministicLocalKCut, DeterministicFamilyGenerator,
        CutCertificate, CertificateError, AuditLogger,
        DynamicConnectivity, PolylogConnectivity, PolylogStats,
        ProperCutInstance, InstanceResult, WitnessHandle, StubInstance, BoundedInstance,
        MinCutWrapper,
        ClusterHierarchy, Cluster,
        Fragment, FragmentResult, FragmentingAlgorithm,
        BitSet256, CompactEdge, CompactWitness, CompactAdjacency, CompactCoreState,
        CoreResult, CompactVertexId, CompactEdgeId, MAX_VERTICES_PER_CORE, MAX_EDGES_PER_CORE,
        NUM_CORES, RANGES_PER_CORE, CoreStrategy, SharedCoordinator,
        CoreDistributor, CoreExecutor, ResultAggregator, compute_core_range,
        RuVectorGraphAnalyzer, CommunityDetector, GraphPartitioner,
        // Subpolynomial min-cut
        SubpolynomialMinCut, SubpolyConfig, RecourseStats,
        // SNN Integration types
        CognitiveMinCutEngine, EngineConfig, EngineMetrics,
        AttractorDynamics, AttractorConfig,
        TimeCrystalCPG, CPGConfig,
        NeuralGraphOptimizer, OptimizerConfig,
        Spike, SimTime,
    };

    #[cfg(feature = "agentic")]
    pub use crate::AgenticAnalyzer;

    #[cfg(feature = "monitoring")]
    pub use crate::{MinCutMonitor, MonitorBuilder, MinCutEvent, EventType};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
        assert!(!NAME.is_empty());
        assert_eq!(NAME, "ruvector-mincut");
    }

    #[test]
    fn test_basic_workflow() {
        // Test the main API works correctly
        let mut mincut = MinCutBuilder::new()
            .exact()
            .with_edges(vec![
                (1, 2, 1.0),
                (2, 3, 1.0),
                (3, 1, 1.0),
            ])
            .build()
            .unwrap();

        // Verify initial state
        assert_eq!(mincut.num_vertices(), 3);
        assert_eq!(mincut.num_edges(), 3);
        assert_eq!(mincut.min_cut_value(), 2.0);

        // Test insert
        let new_cut = mincut.insert_edge(1, 4, 2.0).unwrap();
        assert!(new_cut.is_finite());
        assert_eq!(mincut.num_edges(), 4);

        // Test delete
        let new_cut = mincut.delete_edge(1, 2).unwrap();
        assert!(new_cut.is_finite());
        assert_eq!(mincut.num_edges(), 3);
    }

    #[test]
    fn test_prelude_imports() {
        use crate::prelude::*;

        // Ensure all prelude items are accessible
        let mincut = MinCutBuilder::new()
            .build()
            .unwrap();

        assert_eq!(mincut.min_cut_value(), f64::INFINITY);
    }

    #[test]
    fn test_approximate_mode() {
        let mincut = MinCutBuilder::new()
            .approximate(0.1)
            .with_edges(vec![(1, 2, 1.0)])
            .build()
            .unwrap();

        let result = mincut.min_cut();
        assert!(!result.is_exact);
        assert_eq!(result.approximation_ratio, 1.1);
    }

    #[test]
    fn test_exact_mode() {
        let mincut = MinCutBuilder::new()
            .exact()
            .with_edges(vec![(1, 2, 1.0)])
            .build()
            .unwrap();

        let result = mincut.min_cut();
        assert!(result.is_exact);
        assert_eq!(result.approximation_ratio, 1.0);
    }

    #[test]
    fn test_min_cut_result() {
        let mincut = MinCutBuilder::new()
            .with_edges(vec![
                (1, 2, 1.0),
                (2, 3, 1.0),
                (3, 1, 1.0),
            ])
            .build()
            .unwrap();

        let result = mincut.min_cut();
        assert_eq!(result.value, 2.0);
        assert!(result.cut_edges.is_some());
        assert!(result.partition.is_some());

        if let Some((s, t)) = result.partition {
            assert_eq!(s.len() + t.len(), 3);
            assert!(!s.is_empty());
            assert!(!t.is_empty());
        }
    }

    #[test]
    fn test_graph_stats() {
        let mincut = MinCutBuilder::new()
            .with_edges(vec![
                (1, 2, 2.0),
                (2, 3, 3.0),
                (3, 1, 1.0),
            ])
            .build()
            .unwrap();

        let graph = mincut.graph();
        let stats = graph.read().stats();

        assert_eq!(stats.num_vertices, 3);
        assert_eq!(stats.num_edges, 3);
        assert_eq!(stats.total_weight, 6.0);
        assert_eq!(stats.min_degree, 2);
        assert_eq!(stats.max_degree, 2);
    }

    #[test]
    fn test_algorithm_stats() {
        let mut mincut = MinCutBuilder::new()
            .with_edges(vec![(1, 2, 1.0)])
            .build()
            .unwrap();

        mincut.insert_edge(2, 3, 1.0).unwrap();
        mincut.delete_edge(1, 2).unwrap();
        let _ = mincut.min_cut_value();

        let stats = mincut.stats();
        assert_eq!(stats.insertions, 1);
        assert_eq!(stats.deletions, 1);
        assert_eq!(stats.queries, 1);
        assert!(stats.avg_update_time_us > 0.0);
    }

    #[test]
    fn test_dynamic_updates() {
        let mut mincut = MinCutBuilder::new().build().unwrap();

        // Start empty
        assert_eq!(mincut.min_cut_value(), f64::INFINITY);

        // Add edges dynamically
        mincut.insert_edge(1, 2, 1.0).unwrap();
        assert_eq!(mincut.min_cut_value(), 1.0);

        mincut.insert_edge(2, 3, 1.0).unwrap();
        assert_eq!(mincut.min_cut_value(), 1.0);

        mincut.insert_edge(3, 1, 1.0).unwrap();
        assert_eq!(mincut.min_cut_value(), 2.0);
    }

    #[test]
    fn test_disconnected_graph() {
        let mincut = MinCutBuilder::new()
            .with_edges(vec![
                (1, 2, 1.0),
                (3, 4, 1.0),
            ])
            .build()
            .unwrap();

        assert!(!mincut.is_connected());
        assert_eq!(mincut.min_cut_value(), 0.0);
    }

    #[test]
    fn test_builder_pattern() {
        let mincut = MinCutBuilder::new()
            .exact()
            .max_cut_size(500)
            .parallel(true)
            .with_edges(vec![(1, 2, 1.0)])
            .build()
            .unwrap();

        assert!(!mincut.config().approximate);
        assert_eq!(mincut.config().max_exact_cut_size, 500);
        assert!(mincut.config().parallel);
    }

    #[test]
    fn test_weighted_graph() {
        let mincut = MinCutBuilder::new()
            .with_edges(vec![
                (1, 2, 5.0),
                (2, 3, 3.0),
                (3, 1, 2.0),
            ])
            .build()
            .unwrap();

        assert_eq!(mincut.min_cut_value(), 5.0);
    }

    #[test]
    fn test_error_handling() {
        let mut mincut = MinCutBuilder::new()
            .with_edges(vec![(1, 2, 1.0)])
            .build()
            .unwrap();

        // Try to insert duplicate edge
        let result = mincut.insert_edge(1, 2, 2.0);
        assert!(result.is_err());
        assert!(matches!(result, Err(MinCutError::EdgeExists(1, 2))));

        // Try to delete non-existent edge
        let result = mincut.delete_edge(3, 4);
        assert!(result.is_err());
        assert!(matches!(result, Err(MinCutError::EdgeNotFound(3, 4))));
    }

    #[test]
    fn test_large_graph() {
        let mut edges = Vec::new();
        for i in 0..99 {
            edges.push((i, i + 1, 1.0));
        }

        let mincut = MinCutBuilder::new()
            .with_edges(edges)
            .build()
            .unwrap();

        assert_eq!(mincut.num_vertices(), 100);
        assert_eq!(mincut.num_edges(), 99);
        assert_eq!(mincut.min_cut_value(), 1.0);
    }

    #[cfg(feature = "monitoring")]
    #[test]
    fn test_monitoring_feature() {
        use crate::monitoring::EventType;

        // Ensure monitoring types are accessible when feature is enabled
        let _ = EventType::CutIncreased;
        let _ = EventType::CutDecreased;
    }
}
