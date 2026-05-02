# ruvector-hailo-cluster

Multi-Pi cluster coordinator for ruvector's Hailo-8 NPU embedding workers.
Implements P2C+EWMA load balancing, fingerprint enforcement, optional
in-process caching, and Tailscale-tag-based discovery.

> **Status:** library + 3 CLI binaries production-shaped; **131 tests**
> passing across lib unit / cluster integration / 3 CLI integration / 7
> doctest suites. End-to-end NPU inference gated on HEF compilation
> (see [ADR-167][adr167] §6).

[adr167]: ../../docs/adr/ADR-167-ruvector-hailo-npu-embedding-backend.md
[adr168]: ../../docs/adr/ADR-168-ruvector-hailo-cluster-cli-surface.md
[adr169]: ../../docs/adr/ADR-169-ruvector-hailo-cluster-cache-architecture.md
[adr170]: ../../docs/adr/ADR-170-ruvector-hailo-cluster-tracing-correlation.md

## What it ships

### Library

`HailoClusterEmbedder` — the coordinator. Distributes embed requests
across N Pi 5 + Hailo-8 workers via a transport-agnostic dispatch loop.

```rust
use std::sync::Arc;
use ruvector_hailo_cluster::{
    GrpcTransport, HailoClusterEmbedder, WorkerEndpoint,
    transport::EmbeddingTransport,
};

let workers = vec![
    WorkerEndpoint::new("pi-a", "100.77.59.83:50051"),
    WorkerEndpoint::new("pi-b", "100.77.59.84:50051"),
];
let transport: Arc<dyn EmbeddingTransport + Send + Sync> =
    Arc::new(GrpcTransport::new()?);

let cluster = HailoClusterEmbedder::new(workers, transport, 384, "fp:abc")?
    .with_cache(4096);

cluster.validate_fleet()?;                         // boot-time integrity check
let v = cluster.embed_one_blocking("hello world")?;
```

8 entry-point methods cover the full sync × async × single × batch ×
random-id × caller-id matrix:

```rust
embed_one_blocking(text)                            -> Vec<f32>
embed_one(self, text).await                         -> Vec<f32>
embed_batch_blocking(texts)                         -> Vec<Vec<f32>>
embed_batch(self, texts).await                      -> Vec<Vec<f32>>
embed_one_blocking_with_request_id(text, id)        -> Vec<f32>
embed_one_with_request_id(self, text, id).await     -> Vec<f32>
embed_batch_blocking_with_request_id(texts, id)     -> Vec<Vec<f32>>
embed_batch_with_request_id(self, texts, id).await  -> Vec<Vec<f32>>
```

### CLI binaries

| Binary | Purpose |
|---|---|
| `ruvector-hailo-worker` | Hailo NPU server (runs on each Pi) |
| `ruvector-hailo-fakeworker` | Deterministic mock for demos / tests / dev |
| `ruvector-hailo-embed` | stdin / `--text` → JSONL embeddings |
| `ruvector-hailo-stats` | Fleet observability (TSV / JSON / Prom) |
| `ruvector-hailo-cluster-bench` | Sustained-load harness |

The 3 user-facing binaries (`embed`, `stats`, `cluster-bench`) share a
common flag vocabulary documented in [ADR-168][adr168].

## Quick start

### Local demo (no Pi required)

```bash
# Terminal 1 — fakeworker
RUVECTOR_FAKE_BIND=127.0.0.1:50051 ruvector-hailo-fakeworker

# Terminal 2 — embed via stdin
echo "hello world" | ruvector-hailo-embed --workers 127.0.0.1:50051 --dim 384

# Terminal 2 — bench
ruvector-hailo-cluster-bench --workers 127.0.0.1:50051 --concurrency 4 --duration-secs 10

# Terminal 2 — fleet stats
ruvector-hailo-stats --workers 127.0.0.1:50051
```

The cluster-bench against a single fakeworker on loopback sustains
**~94k req/s** (p99 153µs). With `--cache 2000 --cache-keyspace 100`
the same bench hits **~1.09M req/s** (p99 8µs, 99.98% hit rate).

### Real Pi fleet

```bash
# 1) Install worker binary + systemd unit on each Pi
deploy/install.sh

# 2) Tag each Pi in tailscale (one-time)
sudo tailscale up --advertise-tags=tag:ruvector-hailo-worker

# 3) From any tailnet member, embed via tag-based discovery
ruvector-hailo-embed --tailscale-tag tag:ruvector-hailo-worker --port 50051 \
    --auto-fingerprint --validate-fleet --health-check 30 \
    --batch 32 --cache 4096 --cache-ttl 600 \
    --output full --quiet \
    < corpus.jsonl > embeddings.jsonl
```

## Discovery

Three discovery sources, mutually exclusive:

```bash
# Inline CSV (auto-named static-N)
--workers pi-a-host:50051,pi-b-host:50051,pi-c-host:50051

# File manifest (named workers, comments + blank lines OK)
--workers-file deploy/production.manifest

# Tailscale tag query (resolves at boot)
--tailscale-tag tag:ruvector-hailo-worker --port 50051
```

Manifest format:
```
# Production fleet — fingerprint fp:abc, dim 384.
pi-a = 100.77.59.83:50051
pi-b = 100.77.59.84:50051
pi-c = 100.77.59.85:50051   # spare unit
```

Use `ruvector-hailo-stats --workers-file <path> --list-workers` to
verify a manifest expands as expected (no health probe — works even if
the workers are down).

## Safety surface

Three layers of fingerprint integrity, end-to-end:

1. **Boot** (`validate_fleet`): rejects mismatched workers from the
   coordinator's pool, fails the boot if zero healthy workers remain.
2. **Runtime** (background health-checker, `--health-check N`): ejects
   workers that drift mid-flight, **auto-clears the cache** so stale
   vectors don't outlive the offending worker (see [ADR-169][adr169]).
3. **Ops monitoring** (`stats --strict-homogeneous`): detects drift
   purely from fleet-wide observation; alerts via exit code 3 even if
   no coordinator has fingerprint enforcement enabled.

```bash
# CI-friendly fleet health gate, no console noise:
ruvector-hailo-stats --tailscale-tag tag:ruvector-hailo-worker \
                     --strict-homogeneous --quiet \
    || alert "fleet drift detected"
```

## Caching

Optional in-process LRU. Capacity 0 ≡ disabled (default). Key includes
the model fingerprint so swapping models invalidates everything for free.

```rust
let cluster = HailoClusterEmbedder::new(...)?
    .with_cache(4096);
// Or with a TTL ceiling:
let cluster = HailoClusterEmbedder::new(...)?
    .with_cache_ttl(4096, Duration::from_secs(600));
```

CLI:
```bash
ruvector-hailo-embed --cache 4096 --cache-ttl 600 ...
```

Three eviction triggers:
- LRU overflow (capacity-bounded)
- TTL expiry (time-bounded)
- Manual `cluster.invalidate_cache()` or auto-fired by health-checker
  on detected fingerprint mismatch

See [ADR-169][adr169] for the full design.

## Tracing correlation

Every embed RPC propagates a `request_id` via gRPC metadata header
(`x-request-id`) — workers' tracing spans log it verbatim, so
loki/datadog queries can grep one ID across web → coordinator → worker.

```rust
// Caller-supplied (typical web-handler use case):
let trace_id = req.headers().get("x-request-id")?.to_str()?;
let v = cluster.embed_one_blocking_with_request_id(&query, trace_id)?;

// Auto-generated (default — sortable timestamp prefix):
let v = cluster.embed_one_blocking(&query)?;
// → request_id like "0000019de68b5707983b8745" (24 hex chars,
//   first 16 = epoch ms, last 8 = random)
```

CLI:
```bash
ruvector-hailo-embed --request-id "ci-build-${BUILD_NUM}" ...
ruvector-hailo-cluster-bench --request-id "${BUILD_NUM}" ...
# (bench suffixes per-thread / per-call: <id>.t<tid>.c<counter>)
```

See [ADR-170][adr170] for the full design.

## Output formats

`ruvector-hailo-embed --output {head|full|none}`:
- **head** (default) — first 8 components in `vec_head`, demo-friendly
- **full** — entire vector in `vector`, suitable for ingestion pipelines
- **none** — metadata only, useful for I/O-free benchmarking

`ruvector-hailo-stats {default|--json|--prom|--prom-file <path>}`:
- TSV with header (default)
- NDJSON (one JSON object per worker per tick)
- Prometheus textfile-collector format on stdout
- Atomic textfile write to `<path>` (paired with `--watch N` for
  drop-in node_exporter monitoring)

`ruvector-hailo-cluster-bench --prom <path>`:
- Atomic Prometheus textfile after the bench, including cache hit rate
  metrics when `--cache N` is set

## Test suite

```
                ┌──────────────────────────────┐
                │ Doctests             (7)     │  module + 6 method examples
                ├──────────────────────────────┤
                │ Lib unit             (69)    │  pure Rust, no IO
                ├──────────────────────────────┤
                │ Cluster integration  (12)    │  GrpcTransport + tonic mocks
                ├──────────────────────────────┤
                │ CLI integration      (18)    │  real binaries, real subprocess
                └──────────────────────────────┘
                 106 tests in this crate
```

Run all of them:
```bash
cargo test                                    # all suites
cargo test --doc                              # just doctests
cargo test --test cluster_load_distribution   # tonic integration only
cargo test --test embed_cli                   # binary CLI tests
```

## Deployment

`deploy/`:
- `ruvector-hailo-worker.service` — hardened systemd unit (`DeviceAllow=/dev/hailo0`,
  `ProtectSystem=strict`, `NoNewPrivileges`, etc.)
- `ruvector-hailo.env.example` — env template (model path, bind addr)
- `install.sh` — copies binary + unit + env, enables/starts the service
- `cross-build.sh` — `aarch64-unknown-linux-gnu` cross-compile via
  `gcc-aarch64-linux-gnu`

## ADRs

| ADR | Topic |
|---|---|
| [ADR-167][adr167] | NPU embedding backend (overall design) |
| [ADR-168][adr168] | Cluster CLI surface (3-binary split + flag conventions) |
| [ADR-169][adr169] | Cache architecture (LRU + TTL + fingerprint isolation + auto-invalidate) |
| [ADR-170][adr170] | Tracing correlation (gRPC metadata + sortable IDs + caller propagation) |
