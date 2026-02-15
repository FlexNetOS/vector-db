//! Publishable RVF Acceptance Test — CLI entry point.
//!
//! Generates or verifies a deterministic acceptance test manifest with
//! SHA-256 witness chain. Same seed → same outcomes → same root hash.
//!
//! ```bash
//! # Generate manifest (default config)
//! cargo run --bin acceptance-rvf -- generate -o manifest.json
//!
//! # Generate with custom config
//! cargo run --bin acceptance-rvf -- generate -o manifest.json \
//!     --holdout 200 --training 200 --cycles 5
//!
//! # Verify a manifest (re-runs and compares root hash)
//! cargo run --bin acceptance-rvf -- verify -i manifest.json
//! ```

use clap::{Parser, Subcommand};
use ruvector_benchmarks::acceptance_test::HoldoutConfig;
use ruvector_benchmarks::publishable_rvf::{generate_manifest, verify_manifest};

#[derive(Parser)]
#[command(name = "acceptance-rvf")]
#[command(about = "Publishable RVF acceptance test with witness chain verification")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new acceptance test manifest
    Generate {
        /// Output JSON file path
        #[arg(short, long, default_value = "acceptance_manifest.json")]
        output: String,

        /// Holdout set size
        #[arg(long, default_value_t = 200)]
        holdout: usize,

        /// Training puzzles per cycle
        #[arg(long, default_value_t = 200)]
        training: usize,

        /// Number of training cycles
        #[arg(long, default_value_t = 5)]
        cycles: usize,

        /// Step budget per puzzle
        #[arg(long, default_value_t = 400)]
        budget: usize,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Verify an existing manifest by replaying and comparing root hash
    Verify {
        /// Input JSON file path
        #[arg(short, long)]
        input: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            output,
            holdout,
            training,
            cycles,
            budget,
            verbose,
        } => {
            let config = HoldoutConfig {
                holdout_size: holdout,
                training_per_cycle: training,
                cycles,
                step_budget: budget,
                min_accuracy: 0.50,
                min_dimensions_improved: 1,
                verbose,
                ..Default::default()
            };

            println!("Generating acceptance test manifest...");
            println!("  holdout={}, training={}, cycles={}, budget={}",
                holdout, training, cycles, budget);
            println!();

            let manifest = generate_manifest(&config)?;
            manifest.print_summary();

            let json = serde_json::to_string_pretty(&manifest)?;
            std::fs::write(&output, &json)?;
            println!("  Manifest written to: {}", output);
            println!("  Chain root hash: {}", manifest.chain_root_hash);
            println!();

            if manifest.all_passed {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
        Commands::Verify { input } => {
            println!("Loading manifest from: {}", input);
            let json = std::fs::read_to_string(&input)?;
            let manifest: ruvector_benchmarks::publishable_rvf::RvfManifest =
                serde_json::from_str(&json)?;

            println!("  Chain length: {}", manifest.chain_length);
            println!("  Expected root: {}", &manifest.chain_root_hash[..32.min(manifest.chain_root_hash.len())]);
            println!();
            println!("Re-running acceptance test with same config...");

            let result = verify_manifest(&manifest)?;
            result.print();

            if result.passed() {
                println!("  VERIFICATION: PASSED — outcomes are identical");
                std::process::exit(0);
            } else {
                println!("  VERIFICATION: FAILED — outcomes differ");
                std::process::exit(1);
            }
        }
    }
}
