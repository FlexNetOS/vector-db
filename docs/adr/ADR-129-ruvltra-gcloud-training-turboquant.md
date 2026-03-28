# ADR-129: RuvLTRA Model Training & TurboQuant Optimization on Google Cloud

## Status

Proposed

## Date

2026-03-28

## Context

RuvLTRA models (0.5B-3B parameters) are the purpose-built LLMs powering Claude Code integrations via RuvLLM. The current published models (`ruv/ruvltra-claude-code`, `ruv/ruvltra`, `ruv/ruvltra-medium`, `ruv/ruvltra-small`) have accumulated 8,281 HuggingFace downloads but haven't been retrained since their initial release. Meanwhile, significant new capabilities have been implemented:

1. **TurboQuant** (1,483 lines) — 2-4 bit asymmetric per-channel KV-cache quantization with 6-8x memory reduction
2. **WET Processing** — Common Crawl data pipeline (`brain-wet-daily`) extracting training-relevant web content
3. **Brain Knowledge** — pi.ruv.io brain with 3,870+ memories and 4.7M+ graph edges of accumulated knowledge
4. **v2.1.0 SOTA modules** — FlashAttention-3, Graph RAG, MLA, Mamba SSM, DiskANN, ColBERT, OPQ

### Available GCloud Infrastructure

| Resource | Details |
|----------|---------|
| **Project** | `ruv-dev` |
| **Billing** | `Generative Ai` account (active) |
| **GPUs** | GB200 (192GB), B200 (180GB), H200 (141GB), H100 (80GB), A100 (80GB/40GB), L4 (24GB), T4 (16GB) |
| **TPUs** | v3, v3p, v5l, v5lp, v5p, v6e |
| **Existing finetuning service** | `phi4-finetuning-gpu` — Cloud Run, L4 GPU, 8 CPU, 32GB RAM, HF_TOKEN configured |
| **Scheduler** | 21 active jobs including `brain-train` (every 5min), `brain-transfer` (30min), `brain-wet-daily` |
| **Secrets** | HuggingFace token, Anthropic key, Google AI key, brain keys |
| **Artifact Registry** | `ruvector` Docker repo in us-central1 |
| **Vertex AI** | Enabled, no current jobs |

### Current Model Artifacts

| Model | Parameters | Quants | Downloads | Status |
|-------|-----------|--------|-----------|--------|
| ruvltra-claude-code | Fine-tuned | Q4K, Q5K, Q8, imatrix | 7,615 | Production |
| ruvltra | 0.5B | Q4K, Q5K, Q8, FP16 | 560 | Production |
| ruvltra-medium | 3B | Q4K, Q5K, Q8, FP16 | 74 | Production |
| ruvltra-small | 494M | Q4K, Q5K, Q8, FP16 | 32 | Production |

## Decision

### Phase 1: TurboQuant-Optimized Quantization (Week 1)

**Goal**: Produce TurboQuant-calibrated GGUF files with imatrix calibration data that minimizes TurboQuant quantization error.

#### 1.1 imatrix Recalibration for TurboQuant

Standard imatrix calibration optimizes for standard quantization. TurboQuant uses asymmetric per-channel quantization with different error characteristics. We will:

1. Generate TurboQuant-aware importance matrices using calibration data that emphasizes:
   - KV-cache attention patterns (high-attention token distributions)
   - Per-channel variance analysis for asymmetric quantization
   - Code generation, agent routing, and Claude Code instruction-following tasks

2. Produce new GGUF variants:

| Variant | Quantization | TurboQuant KV | Total Memory (3B) | Use Case |
|---------|-------------|---------------|-------------------|----------|
| `Q4_K_M-TQ3` | Q4_K_M weights + 3-bit KV | 10.7x KV compression | ~2.1 GB | **Default — production** |
| `Q4_K_M-TQ4` | Q4_K_M weights + 4-bit KV | 8x KV compression | ~2.3 GB | High quality |
| `Q8_0-TQ3` | Q8 weights + 3-bit KV | 10.7x KV compression | ~3.5 GB | Quality-first |
| `Q2_K-TQ2` | Q2_K weights + 2-bit KV | 32x KV compression | ~1.0 GB | Edge/mobile |

#### 1.2 Implementation

```bash
# Cloud Run Job: TurboQuant calibration
gcloud run jobs create ruvltra-turboquant-calibration \
  --image=gcr.io/ruv-dev/ruvltra-training:latest \
  --cpu=8 --memory=32Gi --gpu=1 --gpu-type=nvidia-l4 \
  --region=us-central1 \
  --set-secrets=HF_TOKEN=huggingface-token:latest \
  --max-retries=1 --task-timeout=3600s \
  --command="python3,calibrate_turboquant.py"
```

**Calibration script** produces:
1. TurboQuant-aware imatrix from calibration dataset (code generation + agent routing examples)
2. Per-channel scale/zero-point calibration for each KV head
3. Optimal bit-width recommendation per layer based on attention entropy

### Phase 2: WET-Augmented Fine-Tuning (Week 2-3)

**Goal**: Fine-tune RuvLTRA models on curated data from brain knowledge + WET processing + new v2.1.0 documentation.

#### 2.1 Training Data Sources

| Source | Records | Content | Pipeline |
|--------|---------|---------|----------|
| **Brain memories** | 3,870+ | Architecture patterns, solutions, conventions, debug knowledge | `pi.ruv.io/v1/memories/list` |
| **WET extraction** | ~50K pages | Rust/ML/vector-DB documentation from Common Crawl | `brain-wet-daily` scheduler |
| **Claude Flow routing** | 2,700+ | Claude-style training examples (existing HF dataset) | `ruvnet/claude-flow-routing` |
| **v2.1.0 code** | 8,577 lines | TurboQuant, Graph RAG, FlashAttention-3, DiskANN implementations | Git history |
| **ADR corpus** | 129 docs | Architectural decisions with rationale | `docs/adr/` |

#### 2.2 Data Processing Pipeline

```
WET segments → CommonCrawlAdapter → Dedup (bloom) → Content filter
                                                          ↓
Brain memories → /v1/memories/search → Category filter → Merge
                                                          ↓
Claude dataset → HF download → Format validation → Unified corpus
                                                          ↓
                                                    SFT/DPO split
                                                    (80/20 train/eval)
```

#### 2.3 Training Configuration

**Infrastructure**:
- **Phase 2a (SFT)**: Vertex AI Custom Job, 1x A100-80GB, 4-8 hours
- **Phase 2b (DPO)**: Vertex AI Custom Job, 1x A100-80GB, 2-4 hours
- **Estimated cost**: ~$30-50 per full training run (A100 at $3.67/hr)

**Hyperparameters (SFT)**:

| Parameter | RuvLTRA-Small (0.5B) | RuvLTRA-Medium (3B) |
|-----------|---------------------|---------------------|
| Learning rate | 2e-5 | 1e-5 |
| Batch size | 16 | 8 |
| Epochs | 3 | 2 |
| LoRA rank | 16 | 32 |
| LoRA alpha | 32 | 64 |
| LoRA targets | Q,K,V,O,Gate,Up | Q,K,V,O,Gate,Up |
| Max seq length | 4096 | 8192 |
| Warmup ratio | 0.05 | 0.03 |
| Weight decay | 0.01 | 0.01 |
| Gradient checkpointing | Yes | Yes |

**Hyperparameters (DPO)**:

| Parameter | Value |
|-----------|-------|
| Beta | 0.1 |
| Learning rate | 5e-6 |
| Epochs | 1 |
| Max prompt length | 1024 |
| Max completion length | 2048 |

### Phase 3: Benchmarking & Validation (Week 3-4)

#### 3.1 Benchmark Suite

| Benchmark | Metric | Current Baseline | Target |
|-----------|--------|-----------------|--------|
| **Code generation** | pass@1 on HumanEval | TBD | >50% (0.5B), >65% (3B) |
| **Agent routing** | Accuracy on routing dataset | 80% | >85% |
| **TurboQuant quality** | Perplexity degradation | N/A | <0.5% at 4-bit, <1% at 3-bit |
| **Inference speed** | tok/s on M4 Pro | 88-135 | >100 (0.5B), >60 (3B) |
| **Memory** | Peak VRAM with TQ3 KV | N/A | <2GB (0.5B), <4GB (3B) |
| **Long context** | Perplexity at 32K tokens | N/A | <15 PPL (3B with TQ3) |
| **SWE-Bench Lite** | Resolution rate | TBD | >10% (0.5B), >20% (3B) |

#### 3.2 TurboQuant-Specific Benchmarks

```rust
// Run from crates/ruvllm
cargo bench --bench turbo_quant_bench

// Benchmarks included:
// - compress_batch/128d, 256d, 512d, 1024d
// - decompress_batch
// - inner_product_asymmetric vs inner_product_asymmetric_optimized
// - kv_cache_tier push/get throughput
// - embedding_store search latency
```

| Benchmark | Expected Result |
|-----------|----------------|
| Compress 1M KV vectors (128d, 3-bit) | <500ms |
| Asymmetric inner product (batch 1000) | <1ms |
| KV-cache tier push (per entry) | <10µs |
| Embedding store search (10K vectors, top-10) | <5ms |

#### 3.3 Automated Benchmark Pipeline

```yaml
# Cloud Scheduler: weekly benchmark
gcloud scheduler jobs create http ruvltra-benchmark-weekly \
  --schedule="0 6 * * 1" \
  --uri="https://us-central1-run.googleapis.com/apis/run.googleapis.com/v1/namespaces/ruv-dev/jobs/ruvltra-benchmark:run" \
  --location=us-central1
```

### Phase 4: Publishing (Week 4)

#### 4.1 Model Publishing Pipeline

```
Train → Merge LoRA → Convert GGUF → TurboQuant calibrate → Benchmark
                                           ↓
                         Q4_K_M, Q5_K_M, Q8_0 (standard)
                         Q4_K_M-TQ3, Q4_K_M-TQ4 (TurboQuant-optimized)
                                           ↓
                                    Upload to HuggingFace
                                    Update model cards
                                    Notify via Resend email
```

#### 4.2 Model Card Updates

Each model card will include:
- TurboQuant benchmark results (compression ratio, perplexity delta)
- Training data sources and sizes
- SWE-Bench and HumanEval scores
- Recommended `ruvllm` configuration
- Memory footprint comparison (standard vs TurboQuant KV)

#### 4.3 Versioning

| Model | Current | After Training |
|-------|---------|---------------|
| ruvltra-claude-code | v1.0 | v2.0-tq |
| ruvltra | v1.0 | v2.0-tq |
| ruvltra-medium | v1.0 | v2.0-tq |
| ruvltra-small | v1.0 | v2.0-tq |

## Cost Estimate

| Phase | Resource | Duration | Cost |
|-------|----------|----------|------|
| TurboQuant calibration | L4 GPU (Cloud Run) | 2 hours | ~$4 |
| SFT training (0.5B) | A100-80GB (Vertex AI) | 4 hours | ~$15 |
| SFT training (3B) | A100-80GB (Vertex AI) | 8 hours | ~$30 |
| DPO training (both) | A100-80GB (Vertex AI) | 4 hours | ~$15 |
| GGUF conversion | L4 GPU (Cloud Run) | 1 hour | ~$2 |
| Benchmarking | L4 GPU (Cloud Run) | 2 hours | ~$4 |
| **Total** | | **~21 hours** | **~$70** |

Weekly benchmark runs add ~$4/week (~$16/month).

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Catastrophic forgetting during SFT | Model loses general ability | EWC++ regularization (SONA integration), eval after each epoch |
| WET data quality | Noisy training data degrades model | Content filtering, dedup, quality scoring before inclusion |
| TurboQuant calibration mismatch | Quantized model quality drops | A/B test against standard quantization on eval set |
| GPU quota limits | Training job fails | Use preemptible instances, retry logic, L4 fallback |
| HuggingFace token scope | Upload fails | Verify write scope before training pipeline starts |

## Alternatives Considered

1. **Vertex AI Model Garden**: Pre-built fine-tuning pipelines, but no TurboQuant integration and limited model architecture support.
2. **GKE with GPU node pool**: More flexible but higher operational complexity. Cloud Run jobs are simpler for batch workloads.
3. **TPU training**: Better cost/perf for large models, but RuvLTRA models (0.5B-3B) are small enough that A100 is sufficient and simpler.
4. **External training providers** (Lambda, RunPod): Cheaper GPU hours but no integration with existing GCloud secrets, scheduler, and Artifact Registry.

## Next Steps

1. [ ] Build `gcr.io/ruv-dev/ruvltra-training:latest` Docker image with TurboQuant calibration tooling
2. [ ] Export brain memories and WET-processed data as training corpus
3. [ ] Create Vertex AI custom training job template
4. [ ] Run Phase 1 TurboQuant calibration on existing models
5. [ ] Benchmark calibrated models against uncalibrated baseline
6. [ ] Run Phase 2 SFT + DPO training
7. [ ] Produce new GGUF variants and publish to HuggingFace
8. [ ] Update model cards with benchmark results
9. [ ] Set up weekly benchmark scheduler job

## References

- [TurboQuant implementation](../../crates/ruvllm/src/quantize/turbo_quant.rs)
- [KV-Cache management](../../crates/ruvllm/src/kv_cache.rs)
- [WET processing pipeline](../../crates/mcp-brain-server/src/pipeline.rs)
- [ADR-128: SOTA Gap Implementations](./ADR-128-sota-gap-implementations.md)
- [v2.1.0 Release](https://github.com/ruvnet/RuVector/releases/tag/v2.1.0)
- [phi4-finetuning-gpu service](https://console.cloud.google.com/run/detail/us-central1/phi4-finetuning-gpu/revisions?project=ruv-dev) — existing template
