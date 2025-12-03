# @ruvector/postgres-cli

[![npm version](https://img.shields.io/npm/v/@ruvector/postgres-cli.svg)](https://www.npmjs.com/package/@ruvector/postgres-cli)
[![npm downloads](https://img.shields.io/npm/dm/@ruvector/postgres-cli.svg)](https://www.npmjs.com/package/@ruvector/postgres-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Node.js](https://img.shields.io/badge/Node.js-18+-green.svg)](https://nodejs.org/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-14--17-blue.svg)](https://www.postgresql.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-blue.svg)](https://www.typescriptlang.org/)

**The most advanced AI vector database CLI for PostgreSQL.** A drop-in pgvector replacement with 53+ SQL functions, 39 attention mechanisms, GNN layers, hyperbolic embeddings, and self-learning capabilities.

## Why RuVector?

| Feature | pgvector | RuVector |
|---------|----------|----------|
| Vector Search | HNSW, IVFFlat | HNSW, IVFFlat |
| Distance Metrics | 3 | 8+ (including hyperbolic) |
| Attention Mechanisms | - | 39 types |
| Graph Neural Networks | - | GCN, GraphSAGE, GAT |
| Hyperbolic Embeddings | - | Poincare, Lorentz |
| Sparse Vectors / BM25 | - | Full support |
| Self-Learning | - | ReasoningBank |
| Agent Routing | - | Tiny Dancer |

## Installation

```bash
# Global installation
npm install -g @ruvector/postgres-cli

# Or use npx directly
npx @ruvector/postgres-cli info
```

## Quick Start

### 1. Connect to PostgreSQL

```bash
# Set connection string
export DATABASE_URL="postgresql://user:pass@localhost:5432/mydb"

# Or use -c flag
ruvector-pg -c "postgresql://user:pass@localhost:5432/mydb" info
```

### 2. Install Extension

```bash
# Install ruvector extension
ruvector-pg install

# Verify installation
ruvector-pg info
```

### 3. Create & Search Vectors

```bash
# Create a vector table with HNSW index
ruvector-pg vector create embeddings --dim 384 --index hnsw

# Insert vectors from file
ruvector-pg vector insert embeddings --file vectors.json

# Search similar vectors
ruvector-pg vector search embeddings --query "[0.1, 0.2, 0.3, ...]" --top-k 10

# Compute distance between vectors
ruvector-pg vector distance --a "[0.1, 0.2]" --b "[0.3, 0.4]" --metric cosine
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    @ruvector/postgres-cli                          │
├─────────────────────────────────────────────────────────────────────┤
│  CLI Layer (Commander.js)                                          │
│    ├── vector    - CRUD & search operations                        │
│    ├── attention - 39 attention mechanism types                    │
│    ├── gnn       - Graph Neural Network layers                     │
│    ├── graph     - Cypher queries & traversal                      │
│    ├── hyperbolic- Poincare/Lorentz embeddings                     │
│    ├── sparse    - BM25/SPLADE scoring                             │
│    ├── routing   - Tiny Dancer agent routing                       │
│    ├── learning  - ReasoningBank self-learning                     │
│    ├── bench     - Performance benchmarking                        │
│    └── quant     - Quantization (scalar/product/binary)            │
├─────────────────────────────────────────────────────────────────────┤
│  Client Layer (pg with connection pooling)                         │
│    ├── Connection pooling (max 10, idle timeout 30s)               │
│    ├── Automatic retry (3 attempts, exponential backoff)           │
│    ├── Batch operations (1000 vectors/batch)                       │
│    ├── SQL injection protection                                    │
│    └── Input validation                                            │
├─────────────────────────────────────────────────────────────────────┤
│  PostgreSQL Extension (ruvector-postgres crate)                    │
│    └── 53 SQL functions exposed via pgrx                           │
└─────────────────────────────────────────────────────────────────────┘
```

## Commands Reference

### Vector Operations

```bash
# Create table with HNSW or IVFFlat index
ruvector-pg vector create <table> --dim <n> --index <hnsw|ivfflat>

# Insert from JSON file
ruvector-pg vector insert <table> --file data.json

# Semantic search
ruvector-pg vector search <table> --query "[...]" --top-k 10 --metric cosine

# Distance calculation
ruvector-pg vector distance --a "[...]" --b "[...]" --metric <cosine|l2|ip>

# Vector normalization
ruvector-pg vector normalize --vector "[0.5, 0.3, 0.2]"
```

### Hyperbolic Geometry

Perfect for hierarchical data like taxonomies and knowledge graphs:

```bash
# Poincare ball distance
ruvector-pg hyperbolic poincare-distance --a "[0.1, 0.2]" --b "[0.3, 0.4]" --curvature -1.0

# Lorentz hyperboloid distance
ruvector-pg hyperbolic lorentz-distance --a "[1.1, 0.1, 0.2]" --b "[1.2, 0.3, 0.4]"

# Mobius addition (hyperbolic translation)
ruvector-pg hyperbolic mobius-add --a "[0.1, 0.2]" --b "[0.05, 0.1]"

# Exponential map (tangent to manifold)
ruvector-pg hyperbolic exp-map --base "[0.0, 0.0]" --tangent "[0.1, 0.2]"

# Convert between models
ruvector-pg hyperbolic poincare-to-lorentz --vector "[0.3, 0.4]"
ruvector-pg hyperbolic lorentz-to-poincare --vector "[1.5, 0.3, 0.4]"
```

### Attention Mechanisms

```bash
# Compute attention (39 types available)
ruvector-pg attention compute \
  --query "[0.1, 0.2, ...]" \
  --keys "[[...], [...]]" \
  --values "[[...], [...]]" \
  --type scaled_dot

# List all 39 attention types
ruvector-pg attention list-types
```

### Graph Neural Networks

```bash
# GCN layer
ruvector-pg gnn gcn --features "[[...]]" --adj "[[...]]" --weights "[[...]]"

# GraphSAGE layer
ruvector-pg gnn graphsage --features "[[...]]" --neighbors "[[...]]"

# GAT (Graph Attention) layer
ruvector-pg gnn gat --features "[[...]]" --adj "[[...]]"
```

### Graph & Cypher

```bash
# Execute Cypher query
ruvector-pg graph query "MATCH (n:Person)-[:KNOWS]->(m) RETURN n, m"

# Create nodes and edges
ruvector-pg graph create-node --labels "Person,Developer" --properties '{"name": "Alice"}'
ruvector-pg graph create-edge --from node1 --to node2 --type KNOWS

# Graph traversal
ruvector-pg graph traverse --start node123 --depth 3 --type bfs
```

### Sparse Vectors & BM25

```bash
# Create sparse vector
ruvector-pg sparse create --indices "[0, 5, 10]" --values "[0.5, 0.3, 0.2]" --dim 100

# BM25 scoring
ruvector-pg sparse bm25 --query-terms "[1, 5, 10]" --doc-freqs "[100, 50, 10]"

# Sparse dot product
ruvector-pg sparse dot --a "0:0.5,5:0.3" --b "0:0.2,5:0.8"
```

### Agent Routing (Tiny Dancer)

```bash
# Route query to best agent
ruvector-pg routing route --query "[0.1, 0.2, ...]" --agents agents.json

# Register new agent
ruvector-pg routing register --name "summarizer" --capabilities "[0.8, 0.2, ...]"

# Multi-agent routing
ruvector-pg routing multi-route --query "[...]" --top-k 3
```

### Self-Learning (ReasoningBank)

```bash
# Record learning trajectory
ruvector-pg learning record --input "[...]" --output "[...]" --success true

# Get adaptive search parameters
ruvector-pg learning adaptive-search --context "[0.1, 0.2, ...]"

# Train from trajectories
ruvector-pg learning train --file trajectories.json --epochs 10
```

### Benchmarking

```bash
# Run full benchmark suite
ruvector-pg bench run --type all --size 10000 --dim 384

# Benchmark specific operation
ruvector-pg bench run --type search --size 100000 --dim 768

# Generate report
ruvector-pg bench report --format table
```

## Benchmarks

Performance measured on AMD EPYC 7763 (64 cores), 256GB RAM:

| Operation | 10K vectors | 100K vectors | 1M vectors |
|-----------|-------------|--------------|------------|
| HNSW Build | 0.8s | 8.2s | 95s |
| HNSW Search (top-10) | 0.3ms | 0.5ms | 1.2ms |
| Cosine Distance | 0.01ms | 0.01ms | 0.01ms |
| Poincare Distance | 0.02ms | 0.02ms | 0.02ms |
| GCN Forward | 2.1ms | 18ms | 180ms |
| BM25 Score | 0.05ms | 0.08ms | 0.15ms |

*Dimensions: 384 for vector ops, 128 for GNN*

## Docker Quick Start

```bash
# Pull and run the RuVector PostgreSQL image
docker run -d --name ruvector-pg \
  -e POSTGRES_PASSWORD=secret \
  -p 5432:5432 \
  ruvector/postgres:latest

# Connect with CLI
ruvector-pg -c "postgresql://postgres:secret@localhost:5432/postgres" install
```

## Usage Tutorial: Building a Semantic Search Engine

### Step 1: Setup

```bash
# Create database
createdb semantic_search
ruvector-pg -c "postgresql://localhost/semantic_search" install
```

### Step 2: Create Embeddings Table

```bash
ruvector-pg vector create documents --dim 384 --index hnsw
```

### Step 3: Insert Documents (from JSON)

```json
// documents.json
[
  {"vector": [0.1, 0.2, ...], "metadata": {"title": "AI Overview", "category": "tech"}},
  {"vector": [0.3, 0.1, ...], "metadata": {"title": "ML Basics", "category": "tech"}}
]
```

```bash
ruvector-pg vector insert documents --file documents.json
```

### Step 4: Semantic Search

```bash
# Find similar documents
ruvector-pg vector search documents \
  --query "[0.15, 0.18, ...]" \
  --top-k 5 \
  --metric cosine
```

### Step 5: Add Hybrid Search with BM25

```bash
# Create sparse representation for text search
ruvector-pg sparse create --indices "[10, 25, 42]" --values "[2.5, 1.8, 3.2]" --dim 10000
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://localhost:5432/ruvector` |
| `RUVECTOR_POOL_SIZE` | Connection pool size | `10` |
| `RUVECTOR_TIMEOUT` | Query timeout (ms) | `30000` |
| `RUVECTOR_RETRIES` | Max retry attempts | `3` |

## Global Options

```bash
-c, --connection <string>  PostgreSQL connection string
-v, --verbose              Enable verbose output
-h, --help                 Display help
--version                  Display version
```

## Features Summary

- **Vector Search**: HNSW and IVFFlat indexes with cosine, L2, inner product, and hyperbolic metrics
- **39 Attention Mechanisms**: Scaled dot-product, multi-head, flash, sparse, linear, causal, and more
- **Graph Neural Networks**: GCN, GraphSAGE, GAT, GIN layers with message passing
- **Graph Operations**: Full Cypher query support, BFS/DFS traversal, PageRank
- **Self-Learning**: ReasoningBank-based trajectory learning and adaptive search
- **Hyperbolic Embeddings**: Poincare ball and Lorentz hyperboloid models for hierarchies
- **Sparse Vectors**: BM25, TF-IDF, and SPLADE for hybrid search
- **Agent Routing**: Tiny Dancer routing with FastGRNN acceleration
- **Quantization**: Scalar, product, and binary quantization for memory efficiency
- **Performance**: Connection pooling, batch operations, automatic retries

## Related Packages

- [`ruvector-postgres`](https://crates.io/crates/ruvector-postgres) - Rust PostgreSQL extension
- [`ruvector-core`](https://crates.io/crates/ruvector-core) - Core vector operations library

## Contributing

Contributions welcome! See [CONTRIBUTING.md](https://github.com/ruvnet/ruvector/blob/main/CONTRIBUTING.md).

## License

MIT - see [LICENSE](https://github.com/ruvnet/ruvector/blob/main/LICENSE)
