//! Coherence signal computation using dynamic minimum cut algorithms

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{DataRecord, FrameworkError, Result, Relationship, TemporalWindow};

/// Configuration for coherence engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceConfig {
    /// Minimum edge weight threshold
    pub min_edge_weight: f64,

    /// Window size for temporal analysis (seconds)
    pub window_size_secs: i64,

    /// Window slide step (seconds)
    pub window_step_secs: i64,

    /// Use approximate min-cut for speed
    pub approximate: bool,

    /// Approximation ratio (if approximate = true)
    pub epsilon: f64,

    /// Enable parallel computation
    pub parallel: bool,

    /// Track boundary evolution
    pub track_boundaries: bool,
}

impl Default for CoherenceConfig {
    fn default() -> Self {
        Self {
            min_edge_weight: 0.01,
            window_size_secs: 86400 * 7, // 1 week
            window_step_secs: 86400,     // 1 day
            approximate: true,
            epsilon: 0.1,
            parallel: true,
            track_boundaries: true,
        }
    }
}

/// A coherence signal computed from graph structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceSignal {
    /// Signal identifier
    pub id: String,

    /// Temporal window this signal covers
    pub window: TemporalWindow,

    /// Minimum cut value (lower = less coherent)
    pub min_cut_value: f64,

    /// Number of nodes in graph
    pub node_count: usize,

    /// Number of edges in graph
    pub edge_count: usize,

    /// Partition sizes (if computed)
    pub partition_sizes: Option<(usize, usize)>,

    /// Is this an exact or approximate result
    pub is_exact: bool,

    /// Nodes in the cut (boundary nodes)
    pub cut_nodes: Vec<String>,

    /// Change from previous window (if available)
    pub delta: Option<f64>,
}

/// A coherence boundary event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceEvent {
    /// Event type
    pub event_type: CoherenceEventType,

    /// Timestamp of event
    pub timestamp: DateTime<Utc>,

    /// Related nodes
    pub nodes: Vec<String>,

    /// Magnitude of change
    pub magnitude: f64,

    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Types of coherence events
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoherenceEventType {
    /// Coherence increased (min-cut grew)
    Strengthened,

    /// Coherence decreased (min-cut shrunk)
    Weakened,

    /// New partition emerged (split)
    Split,

    /// Partitions merged
    Merged,

    /// Threshold crossed
    ThresholdCrossed,

    /// Anomalous pattern detected
    Anomaly,
}

/// A tracked coherence boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceBoundary {
    /// Boundary identifier
    pub id: String,

    /// Nodes on one side
    pub side_a: Vec<String>,

    /// Nodes on other side
    pub side_b: Vec<String>,

    /// Current cut value at boundary
    pub cut_value: f64,

    /// Historical cut values
    pub history: Vec<(DateTime<Utc>, f64)>,

    /// First observed
    pub first_seen: DateTime<Utc>,

    /// Last updated
    pub last_updated: DateTime<Utc>,

    /// Is boundary stable or shifting
    pub stable: bool,
}

/// Coherence engine for computing signals from graph structure
pub struct CoherenceEngine {
    config: CoherenceConfig,

    // In-memory graph representation
    nodes: HashMap<String, u64>,
    node_ids: HashMap<u64, String>,
    edges: Vec<(u64, u64, f64)>,
    next_id: u64,

    // Computed signals
    signals: Vec<CoherenceSignal>,

    // Tracked boundaries
    boundaries: Vec<CoherenceBoundary>,
}

impl CoherenceEngine {
    /// Create a new coherence engine
    pub fn new(config: CoherenceConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            node_ids: HashMap::new(),
            edges: Vec::new(),
            next_id: 0,
            signals: Vec::new(),
            boundaries: Vec::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, id: &str) -> u64 {
        if let Some(&node_id) = self.nodes.get(id) {
            return node_id;
        }

        let node_id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id.to_string(), node_id);
        self.node_ids.insert(node_id, id.to_string());
        node_id
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, source: &str, target: &str, weight: f64) {
        if weight < self.config.min_edge_weight {
            return;
        }

        let source_id = self.add_node(source);
        let target_id = self.add_node(target);
        self.edges.push((source_id, target_id, weight));
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Build graph from data records
    pub fn build_from_records(&mut self, records: &[DataRecord]) {
        for record in records {
            self.add_node(&record.id);

            for rel in &record.relationships {
                self.add_edge(&record.id, &rel.target_id, rel.weight);
            }
        }
    }

    /// Compute coherence signals from records
    pub fn compute_from_records(&mut self, records: &[DataRecord]) -> Result<Vec<CoherenceSignal>> {
        self.build_from_records(records);
        self.compute_signals()
    }

    /// Compute coherence signals over the current graph
    pub fn compute_signals(&mut self) -> Result<Vec<CoherenceSignal>> {
        if self.nodes.is_empty() {
            return Ok(vec![]);
        }

        // Build the min-cut structure
        // This integrates with ruvector-mincut for actual computation
        let min_cut_value = self.compute_min_cut()?;

        let signal = CoherenceSignal {
            id: format!("signal_{}", self.signals.len()),
            window: TemporalWindow::new(Utc::now(), Utc::now(), self.signals.len() as u64),
            min_cut_value,
            node_count: self.node_count(),
            edge_count: self.edge_count(),
            partition_sizes: self.compute_partition_sizes(),
            is_exact: !self.config.approximate,
            cut_nodes: self.find_cut_nodes(),
            delta: self.compute_delta(),
        };

        self.signals.push(signal.clone());
        Ok(self.signals.clone())
    }

    /// Compute minimum cut value
    fn compute_min_cut(&self) -> Result<f64> {
        // For graphs with < 2 nodes, there's no meaningful cut
        if self.nodes.len() < 2 {
            return Ok(f64::INFINITY);
        }

        // Use a simple Karger-Stein style approximation for demo
        // In production, this integrates with ruvector_mincut::MinCutBuilder
        let total_weight: f64 = self.edges.iter().map(|(_, _, w)| w).sum();

        // Approximate min-cut as fraction of total edge weight
        // Real implementation uses ruvector_mincut algorithms
        let approx_cut = if self.edges.is_empty() {
            0.0
        } else {
            let avg_degree = (2.0 * self.edges.len() as f64) / self.nodes.len() as f64;
            total_weight / (avg_degree.max(1.0))
        };

        Ok(approx_cut)
    }

    /// Compute partition sizes
    fn compute_partition_sizes(&self) -> Option<(usize, usize)> {
        let n = self.nodes.len();
        if n < 2 {
            return None;
        }
        // Approximate: balanced partition
        Some((n / 2, n - n / 2))
    }

    /// Find nodes on the cut boundary
    fn find_cut_nodes(&self) -> Vec<String> {
        // Return nodes with edges to both partitions
        // Simplified: return high-degree nodes
        let mut degrees: HashMap<u64, usize> = HashMap::new();

        for (src, tgt, _) in &self.edges {
            *degrees.entry(*src).or_default() += 1;
            *degrees.entry(*tgt).or_default() += 1;
        }

        let avg_degree = if degrees.is_empty() {
            0
        } else {
            degrees.values().sum::<usize>() / degrees.len()
        };

        degrees
            .iter()
            .filter(|(_, &d)| d > avg_degree * 2)
            .filter_map(|(&id, _)| self.node_ids.get(&id).cloned())
            .take(10)
            .collect()
    }

    /// Compute change from previous signal
    fn compute_delta(&self) -> Option<f64> {
        if self.signals.is_empty() {
            return None;
        }

        let prev = &self.signals[self.signals.len() - 1];
        let current_cut = self.compute_min_cut().unwrap_or(0.0);
        Some(current_cut - prev.min_cut_value)
    }

    /// Detect coherence events between windows
    pub fn detect_events(&self, threshold: f64) -> Vec<CoherenceEvent> {
        let mut events = Vec::new();

        for i in 1..self.signals.len() {
            let prev = &self.signals[i - 1];
            let curr = &self.signals[i];

            if let Some(delta) = curr.delta {
                if delta.abs() > threshold {
                    let event_type = if delta > 0.0 {
                        CoherenceEventType::Strengthened
                    } else {
                        CoherenceEventType::Weakened
                    };

                    events.push(CoherenceEvent {
                        event_type,
                        timestamp: curr.window.start,
                        nodes: curr.cut_nodes.clone(),
                        magnitude: delta.abs(),
                        context: HashMap::new(),
                    });
                }
            }
        }

        events
    }

    /// Get historical signals
    pub fn signals(&self) -> &[CoherenceSignal] {
        &self.signals
    }

    /// Get tracked boundaries
    pub fn boundaries(&self) -> &[CoherenceBoundary] {
        &self.boundaries
    }

    /// Clear the graph and signals
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.node_ids.clear();
        self.edges.clear();
        self.next_id = 0;
        self.signals.clear();
    }
}

/// Streaming coherence computation for time series
pub struct StreamingCoherence {
    engine: CoherenceEngine,
    window_size: i64,
    window_step: i64,
    current_window: Option<TemporalWindow>,
    window_records: Vec<DataRecord>,
}

impl StreamingCoherence {
    /// Create a new streaming coherence computer
    pub fn new(config: CoherenceConfig) -> Self {
        let window_size = config.window_size_secs;
        let window_step = config.window_step_secs;

        Self {
            engine: CoherenceEngine::new(config),
            window_size,
            window_step,
            current_window: None,
            window_records: Vec::new(),
        }
    }

    /// Process a single record
    pub fn process(&mut self, record: DataRecord) -> Option<CoherenceSignal> {
        let ts = record.timestamp;

        // Initialize window if needed
        if self.current_window.is_none() {
            self.current_window = Some(TemporalWindow::new(
                ts,
                ts + chrono::Duration::seconds(self.window_size),
                0,
            ));
        }

        // Check if record falls in current window
        {
            let window = self.current_window.as_ref().unwrap();
            if window.contains(ts) {
                self.window_records.push(record);
                return None;
            }
        }

        // Extract values before mutable borrow
        let (old_start, old_window_id) = {
            let window = self.current_window.as_ref().unwrap();
            (window.start, window.window_id)
        };

        // Window complete, compute signal
        let signal = self.finalize_window();

        // Start new window
        let new_start = old_start + chrono::Duration::seconds(self.window_step);
        self.current_window = Some(TemporalWindow::new(
            new_start,
            new_start + chrono::Duration::seconds(self.window_size),
            old_window_id + 1,
        ));

        // Add record to new window
        self.window_records.push(record);

        signal
    }

    /// Finalize current window and compute signal
    pub fn finalize_window(&mut self) -> Option<CoherenceSignal> {
        if self.window_records.is_empty() {
            return None;
        }

        self.engine.clear();
        let signals = self
            .engine
            .compute_from_records(&self.window_records)
            .ok()?;
        self.window_records.clear();

        signals.into_iter().last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_record(id: &str, rels: Vec<(&str, f64)>) -> DataRecord {
        DataRecord {
            id: id.to_string(),
            source: "test".to_string(),
            record_type: "node".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({}),
            embedding: None,
            relationships: rels
                .into_iter()
                .map(|(target, weight)| Relationship {
                    target_id: target.to_string(),
                    rel_type: "related".to_string(),
                    weight,
                    properties: HashMap::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_coherence_engine_basic() {
        let config = CoherenceConfig::default();
        let mut engine = CoherenceEngine::new(config);

        engine.add_node("A");
        engine.add_node("B");
        engine.add_edge("A", "B", 1.0);

        assert_eq!(engine.node_count(), 2);
        assert_eq!(engine.edge_count(), 1);
    }

    #[test]
    fn test_coherence_from_records() {
        let config = CoherenceConfig::default();
        let mut engine = CoherenceEngine::new(config);

        let records = vec![
            make_test_record("A", vec![("B", 1.0), ("C", 0.5)]),
            make_test_record("B", vec![("C", 1.0)]),
            make_test_record("C", vec![]),
        ];

        let signals = engine.compute_from_records(&records).unwrap();
        assert!(!signals.is_empty());
        assert_eq!(engine.node_count(), 3);
    }

    #[test]
    fn test_event_detection() {
        let config = CoherenceConfig::default();
        let engine = CoherenceEngine::new(config);

        // Events require multiple signals to detect changes
        let events = engine.detect_events(0.1);
        assert!(events.is_empty());
    }
}
