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

### Phase 1: imatrix Recalibration + TurboQuant KV Profiling (Week 1)

**Goal**: Produce improved GGUF quantizations with code-focused imatrix calibration, and generate TurboQuant KV-cache configuration profiles per model.

**Important**: TurboQuant operates at **runtime on KV-cache and embeddings** — it is not a weight quantization format. It is data-oblivious (no training, no codebooks). The optimization here is:
1. Better imatrix calibration → better base GGUF quantizations
2. Per-model TurboQuant KV profiles → optimal bit-width per attention layer at runtime

#### 1.1 imatrix Recalibration

Generate new importance matrices using RuvLTRA-specific calibration data:
- Code generation tasks (HumanEval, MBPP)
- Agent routing examples (Claude Flow dataset, 2,700+ examples)
- Claude Code instruction-following (ADR corpus, v2.1.0 code)

Produce updated GGUF variants with code-optimized imatrix:

| Variant | Format | Size (3B) | Use Case |
|---------|--------|-----------|----------|
| `Q4_K_M` (recalibrated) | Standard GGUF | ~2.1 GB | **Default — production** |
| `Q5_K_M` (recalibrated) | Standard GGUF | ~2.5 GB | Higher quality |
| `Q8_0` (recalibrated) | Standard GGUF | ~3.5 GB | Quality-first |
| `Q2_K` (recalibrated) | Standard GGUF | ~1.0 GB | Edge/mobile |

#### 1.2 TurboQuant KV-Cache Profiling

Profile each model's attention patterns to determine optimal per-layer TurboQuant configuration:

```bash
# Cloud Run Job: imatrix + TurboQuant profiling
gcloud run jobs create ruvltra-calibration \
  --image=gcr.io/ruv-dev/ruvltra-training:latest \
  --cpu=8 --memory=32Gi --gpu=1 --gpu-type=nvidia-l4 \
  --region=us-central1 \
  --set-secrets=HF_TOKEN=huggingface-token:latest \
  --max-retries=1 --task-timeout=3600s \
  --command="python3,calibrate_and_profile.py"
```

**Outputs**:
1. New imatrix files for each model
2. TurboQuant runtime config: recommended bits per layer, eviction policy, QJL settings
3. Perplexity delta report: standard KV vs TQ3 vs TQ4 per layer

### Phase 2: WET-Augmented LoRA Fine-Tuning (Week 2-3)

**Goal**: LoRA fine-tune RuvLTRA models on curated data from brain knowledge + WET (Common Crawl WARC/WET extraction) processing + new v2.1.0 documentation.

**Note**: Full pre-training is not in scope. The existing Rust training infrastructure supports LoRA adapters (rank 2-32) and embedding fine-tuning. For full SFT/DPO, we use Python (transformers + trl + peft) on Vertex AI.

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

## Current Gaps Identified

| Gap | Description | Resolution |
|-----|-------------|------------|
| **No GPU compute provisioned** | All GCloud is CPU-only Cloud Run except `phi4-finetuning-gpu` (L4) | Phase 1-2 provision GPU via Cloud Run Jobs and Vertex AI |
| **TurboQuant has no GGUF format** | TurboQuant is runtime-only (KV-cache/embeddings), no GGUF serialization | Ship TQ runtime configs alongside standard GGUF files |
| **Model checksums not set** | Registry `checksum` fields are `None` for all models | Compute and set SHA256 during Phase 4 publishing |
| **WET pipeline is brain-only** | `CommonCrawlAdapter` feeds brain memories, not model training | Export WET-processed content as training corpus in Phase 2 |
| **No full-model fine-tuning in Rust** | Rust training covers LoRA/embedding-level only | Use Python (transformers + peft) on Vertex AI for SFT/DPO |
| **WASM Pi-Quant incomplete** | ADR-090 Phase 4 (PiQ WASM export) listed as "In Progress" | Track separately, not blocking this ADR |

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

## Existing Training Infrastructure

| Component | Location | What It Does |
|-----------|----------|--------------|
| MicroLoRA training | `crates/ruvllm/src/lora/training.rs` | Per-request LoRA with EWC++ regularization |
| Adapter trainer | `crates/ruvllm/src/lora/adapters/trainer.rs` | Synthetic Claude dataset training |
| Pretrain pipeline | `crates/ruvllm/src/claude_flow/pretrain_pipeline.rs` | 4-phase: Bootstrap, Synthetic, Reinforce, Consolidate |
| TS training | `npm/packages/ruvllm/src/training.ts` | Full pipeline with LR scheduling, early stopping, EWC |
| Contrastive fine-tune | `npm/packages/ruvllm/scripts/training/contrastive-finetune.js` | Triplet loss router training |
| Brain LoRA training | `scripts/train-lora.py` | Federated LoRA with Byzantine-tolerant aggregation |
| 15-agent swarm | `scripts/swarm_train_15.sh` | Parallel discovery + training from 15 data sources |
| Weight quantization | `crates/ruvllm/src/quantize/ruvltra_quant.rs` | Q4_K_M, Q5_K_M, Q8_0, PiQ3, PiQ2 GGUF export |
| TurboQuant (runtime) | `crates/ruvllm/src/quantize/turbo_quant.rs` | 2-4 bit KV-cache/embedding compression |
| Benchmarks | `crates/ruvllm/benches/` | 13 benchmark files covering all subsystems |

## References

- [TurboQuant implementation](../../crates/ruvllm/src/quantize/turbo_quant.rs)
- [KV-Cache management](../../crates/ruvllm/src/kv_cache.rs)
- [WET processing pipeline](../../crates/mcp-brain-server/src/pipeline.rs)
- [ADR-128: SOTA Gap Implementations](./ADR-128-sota-gap-implementations.md)
- [v2.1.0 Release](https://github.com/ruvnet/RuVector/releases/tag/v2.1.0)
- [phi4-finetuning-gpu service](https://console.cloud.google.com/run/detail/us-central1/phi4-finetuning-gpu/revisions?project=ruv-dev) — existing template
- [ADR-049: Verified Training Pipeline](./ADR-049-verified-training-pipeline.md)
- [ADR-090: Ultra-Low-Bit QAT & Pi-Quantization](./ADR-090-ultra-low-bit-qat-pi-quantization.md)
- [ADR-093: Daily Discovery Brain Training](./ADR-093-daily-discovery-brain-training.md)
- [Federated LoRA training script](../../scripts/train-lora.py)
- [15-agent swarm training](../../scripts/swarm_train_15.sh)
- [RuvLTRA model registry](../../crates/ruvllm/src/hub/registry.rs)
