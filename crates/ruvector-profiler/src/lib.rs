//! Memory, power, and latency profiling for attention-mechanism benchmarks.

pub mod config_hash;
pub mod csv_emitter;
pub mod latency;
pub mod memory;
pub mod power;

pub use config_hash::{BenchConfig, config_hash};
pub use csv_emitter::{ResultRow, write_latency_csv, write_memory_csv, write_results_csv};
pub use latency::{LatencyRecord, LatencyStats, compute_latency_stats};
pub use memory::{MemoryReport, MemorySnapshot, MemoryTracker, capture_memory};
pub use power::{EnergyResult, MockPowerSource, PowerSample, PowerSource, PowerTracker};
