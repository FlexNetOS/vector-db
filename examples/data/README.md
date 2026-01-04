# RuVector Dataset Discovery Framework

Comprehensive examples demonstrating RuVector's capabilities for novel discovery across world-scale datasets.

## What's New

- **SIMD-Accelerated Vectors** - 2.9x faster cosine similarity
- **Parallel Batch Processing** - 8.8x faster vector insertion
- **Statistical Significance** - P-values, effect sizes, confidence intervals
- **Temporal Causality** - Granger-style cross-domain prediction
- **Cross-Domain Bridges** - Automatic detection of hidden connections

## Quick Start

```bash
# Run the optimized benchmark
cargo run --example optimized_benchmark -p ruvector-data-framework --features parallel --release

# Run the discovery hunter
cargo run --example discovery_hunter -p ruvector-data-framework --features parallel --release

# Run cross-domain discovery
cargo run --example cross_domain_discovery -p ruvector-data-framework --release

# Run climate regime detector
cargo run --example regime_detector -p ruvector-data-climate

# Run financial coherence watch
cargo run --example coherence_watch -p ruvector-data-edgar
```

## The Discovery Thesis

RuVector's unique combination of **vector memory**, **graph structures**, and **dynamic minimum cut algorithms** enables discoveries that most analysis tools miss:

- **Emerging patterns before they have names**: Detect topic splits and merges as cut boundaries shift over time
- **Non-obvious cross-domain bridges**: Find small "connector" subgraphs where disciplines quietly start citing each other
- **Causal leverage maps**: Link funders, labs, venues, and downstream citations to spot high-impact intervention points
- **Regime shifts in time series**: Use coherence breaks to flag fundamental changes in system behavior

## Tutorial

### 1. Creating the Engine

```rust
use ruvector_data_framework::optimized::{
    OptimizedDiscoveryEngine, OptimizedConfig,
};
use ruvector_data_framework::ruvector_native::{
    Domain, SemanticVector,
};

let config = OptimizedConfig {
    similarity_threshold: 0.55,   // Minimum cosine similarity
    mincut_sensitivity: 0.10,     // Coherence change threshold
    cross_domain: true,           // Enable cross-domain discovery
    use_simd: true,               // SIMD acceleration
    significance_threshold: 0.05, // P-value threshold
    causality_lookback: 12,       // Temporal lookback periods
    ..Default::default()
};

let mut engine = OptimizedDiscoveryEngine::new(config);
```

### 2. Adding Data

```rust
use std::collections::HashMap;
use chrono::Utc;

// Single vector
let vector = SemanticVector {
    id: "climate_drought_2024".to_string(),
    embedding: generate_embedding(), // 128-dim vector
    domain: Domain::Climate,
    timestamp: Utc::now(),
    metadata: HashMap::from([
        ("region".to_string(), "sahel".to_string()),
        ("severity".to_string(), "extreme".to_string()),
    ]),
};
let node_id = engine.add_vector(vector);

// Batch insertion (8.8x faster)
#[cfg(feature = "parallel")]
{
    let vectors: Vec<SemanticVector> = load_vectors();
    let node_ids = engine.add_vectors_batch(vectors);
}
```

### 3. Computing Coherence

```rust
let snapshot = engine.compute_coherence();

println!("Min-cut value: {:.3}", snapshot.mincut_value);
println!("Partition sizes: {:?}", snapshot.partition_sizes);
println!("Boundary nodes: {:?}", snapshot.boundary_nodes);
```

**Interpretation:**
| Min-cut Trend | Meaning |
|---------------|---------|
| Rising | Network consolidating, stronger connections |
| Falling | Fragmentation, potential regime change |
| Stable | Steady state, consistent structure |

### 4. Pattern Detection

```rust
let patterns = engine.detect_patterns_with_significance();

for pattern in patterns.iter().filter(|p| p.is_significant) {
    println!("{}", pattern.pattern.description);
    println!("  P-value: {:.4}", pattern.p_value);
    println!("  Effect size: {:.3}", pattern.effect_size);
}
```

**Pattern Types:**
| Type | Description | Example |
|------|-------------|---------|
| `CoherenceBreak` | Min-cut dropped significantly | Network fragmentation crisis |
| `Consolidation` | Min-cut increased | Market convergence |
| `BridgeFormation` | Cross-domain connections | Climate-finance link |
| `Cascade` | Temporal causality | Climate → Finance lag-3 |
| `EmergingCluster` | New dense subgraph | Research topic emerging |

### 5. Cross-Domain Analysis

```rust
// Check coupling strength
let stats = engine.stats();
let coupling = stats.cross_domain_edges as f64 / stats.total_edges as f64;
println!("Cross-domain coupling: {:.1}%", coupling * 100.0);

// Domain coherence scores
for domain in [Domain::Climate, Domain::Finance, Domain::Research] {
    if let Some(coh) = engine.domain_coherence(domain) {
        println!("{:?}: {:.3}", domain, coh);
    }
}
```

## Performance Benchmarks

| Operation | Baseline | Optimized | Speedup |
|-----------|----------|-----------|---------|
| Vector Insertion | 133ms | 15ms | **8.84x** |
| SIMD Cosine | 432ms | 148ms | **2.91x** |
| Pattern Detection | 524ms | 655ms | - |

## Datasets

### 1. OpenAlex (Research Intelligence)
**Best for**: Emerging field detection, cross-discipline bridges

- 250M+ works, 90M+ authors
- Native graph structure
- Bulk download + API access

```rust
use ruvector_data_openalex::{OpenAlexConfig, FrontierRadar};

let radar = FrontierRadar::new(OpenAlexConfig::default());
let frontiers = radar.detect_emerging_topics(papers);
```

### 2. NOAA + NASA (Climate Intelligence)
**Best for**: Regime shift detection, anomaly prediction

- Weather observations, satellite imagery
- Time series → graph transformation
- Economic risk modeling

```rust
use ruvector_data_climate::{ClimateConfig, RegimeDetector};

let detector = RegimeDetector::new(config);
let shifts = detector.detect_shifts();
```

### 3. SEC EDGAR (Financial Intelligence)
**Best for**: Corporate risk signals, peer divergence

- XBRL financial statements
- 10-K/10-Q filings
- Narrative + fundamental analysis

```rust
use ruvector_data_edgar::{EdgarConfig, CoherenceMonitor};

let monitor = CoherenceMonitor::new(config);
let alerts = monitor.analyze_filing(filing);
```

## Directory Structure

```
examples/data/
├── README.md                 # This file
├── Cargo.toml               # Workspace manifest
├── framework/               # Core discovery framework
│   ├── src/
│   │   ├── lib.rs              # Framework exports
│   │   ├── ruvector_native.rs  # Native engine with Stoer-Wagner
│   │   ├── optimized.rs        # SIMD + parallel optimizations
│   │   ├── coherence.rs        # Coherence signal computation
│   │   ├── discovery.rs        # Pattern detection
│   │   └── ingester.rs         # Data ingestion
│   └── examples/
│       ├── cross_domain_discovery.rs  # Cross-domain patterns
│       ├── optimized_benchmark.rs     # Performance comparison
│       └── discovery_hunter.rs        # Novel pattern search
├── openalex/               # OpenAlex integration
├── climate/                # NOAA/NASA integration
└── edgar/                  # SEC EDGAR integration
```

## Configuration Reference

### OptimizedConfig

| Parameter | Default | Description |
|-----------|---------|-------------|
| `similarity_threshold` | 0.65 | Minimum cosine similarity for edges |
| `mincut_sensitivity` | 0.12 | Sensitivity to coherence changes |
| `cross_domain` | true | Enable cross-domain discovery |
| `batch_size` | 256 | Parallel batch size |
| `use_simd` | true | Enable SIMD acceleration |
| `significance_threshold` | 0.05 | P-value threshold |
| `causality_lookback` | 10 | Temporal lookback periods |
| `causality_min_correlation` | 0.6 | Minimum correlation for causality |

## Discovery Examples

### Climate-Finance Bridge

```
Detected: Climate ↔ Finance bridge
  Strength: 0.73
  Connections: 197

Hypothesis: Drought indices may predict
  utility sector performance with lag-2
```

### Regime Shift Detection

```
Min-cut trajectory:
  t=0: 72.5 (baseline)
  t=1: 73.3 (+1.1%)
  t=2: 74.5 (+1.6%) ← Consolidation

Effect size: 2.99 (large)
P-value: 0.042 (significant)
```

### Causality Pattern

```
Climate → Finance causality detected
  F-statistic: 4.23
  Optimal lag: 3 periods
  Correlation: 0.67
  P-value: 0.031
```

## Algorithms

### Stoer-Wagner Min-Cut
Computes minimum cut of weighted undirected graph.
- **Complexity**: O(VE + V² log V)
- **Use**: Network coherence measurement

### SIMD Cosine Similarity
Processes 8 floats per iteration using AVX2.
- **Speedup**: 2.9x vs scalar
- **Fallback**: Chunked scalar (4 floats)

### Granger Causality
Tests if past values of X predict Y.
1. Compute cross-correlation at lags 1..k
2. Find optimal lag with max |correlation|
3. Calculate F-statistic
4. Convert to p-value

## Best Practices

1. **Start with low thresholds** - Use `similarity_threshold: 0.45` for exploration
2. **Use batch insertion** - `add_vectors_batch()` is 8x faster
3. **Monitor coherence trends** - Min-cut trajectory predicts regime changes
4. **Filter by significance** - Focus on `p_value < 0.05`
5. **Validate causality** - Temporal patterns need domain expertise

## Troubleshooting

| Problem | Solution |
|---------|----------|
| No patterns detected | Lower `mincut_sensitivity` to 0.05 |
| Too many edges | Raise `similarity_threshold` to 0.70 |
| Slow performance | Use `--features parallel --release` |
| Memory issues | Reduce `batch_size` |

## References

- [OpenAlex Documentation](https://docs.openalex.org/)
- [NOAA Open Data](https://www.noaa.gov/information-technology/open-data-dissemination)
- [NASA Earthdata](https://earthdata.nasa.gov/)
- [SEC EDGAR](https://www.sec.gov/edgar)

## License

MIT OR Apache-2.0
