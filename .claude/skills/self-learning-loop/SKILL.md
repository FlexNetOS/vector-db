---
name: self-learning-loop
description: Run the FlexNetOS self-learning agent loop — Identify → Implement → Validate → Optimize → Distill — using the Attractor NLSpec as the blueprint and ruvector's ReasoningBank + AgentDB as the memory substrate. Use when designing or extending agent behavior that should accumulate experience over multiple runs.
---

# Self-Learning Loop — Attractor pipeline on ruvector

## What this skill does

Defines the canonical self-learning agent cycle this repo implements,
based on the [strongdm/attractor](https://github.com/strongdm/attractor)
non-interactive coding agent NLSpec. ruvector is the **memory + reasoning**
side of the loop; weftos is the runtime that executes each node. This
skill is the ruvector-side reference.

## The five-node cycle

```
   ┌──────────────┐
   │ 1. Identify  │  pick the next learnable signal (drift, failure, gap)
   └──────┬───────┘
          ▼
   ┌──────────────┐
   │ 2. Implement │  apply a change (code edit, parameter sweep, prompt tweak)
   └──────┬───────┘
          ▼
   ┌──────────────┐
   │ 3. Validate  │  run tests, gate, benchmarks; capture verdict
   └──────┬───────┘
          ▼
   ┌──────────────┐
   │ 4. Optimize  │  tune, prune, reduce — keep what improves the verdict
   └──────┬───────┘
          ▼
   ┌──────────────┐
   │ 5. Distill   │  store the trajectory + pattern in ReasoningBank
   └──────┬───────┘
          │  (next iteration starts from a richer memory)
          ▼ ── back to 1.
```

A single Attractor DOT graph (committed at `.attractor/integration.dot`
in Phase 3) encodes this cycle declaratively; this skill is the
human-readable reference.

## Where each node lives in ruvector

| Node | Code | Notes |
|---|---|---|
| 1. Identify | `crates/sona/src/reasoning_bank.rs` (`find_similar`) + `crates/prime-radiant/src/sona_tuning/tuner.rs` | Query ReasoningBank for unsolved patterns; pick the lowest-confidence trajectory. |
| 2. Implement | external (weftos `weaver` or a Devin session) | ruvector does not edit code itself; it advises. |
| 3. Validate | `crates/prime-radiant/src/execution/gate.rs` (witness chain) | Gate runs the test/benchmark and records a SHAKE-256 audit anchor. |
| 4. Optimize | `crates/prime-radiant/src/sona_tuning/` (Bayesian + PSO + grid) | Submodule of `prime-radiant`, **not** a standalone crate. Closed-loop hyperparameter search. |
| 5. Distill | `ReasoningBank::add_trajectory` (re-exported by `sona`) | Pattern store — embeddings + verdicts. |

## Run it locally

### Prerequisites

```bash
# from repo root — RUST_MIN_STACK=16777216 is set in .cargo/config.toml
cargo check --workspace --exclude ruvector-postgres
```

### One iteration of the loop

```bash
# 1. Identify — query ReasoningBank for low-confidence patterns
cargo run -p sona-cli -- patterns query --confidence-below 0.6 --limit 5

# 2. Implement — typically external (weaver, Devin); for a dry run:
cargo run -p prime-radiant --example apply_pattern -- \
    --pattern-id <id> --dry-run

# 3. Validate — run the gate
cargo test -p prime-radiant --test integration

# 4. Optimize — invoke the tuner (sona_tuning is a module of prime-radiant,
#    not a standalone package; expose via prime-radiant subcommand or example)
cargo run -p prime-radiant --example tune_bayes -- --target validate_score --budget 20

# 5. Distill — promote the trajectory
cargo run -p sona-cli -- trajectory commit --verdict pass --pattern <id>
```

The exact CLI surface above is aspirational where it does not yet exist;
when implementing, prefer adding subcommands or examples to existing
binaries (`prime-radiant` for the tuner, plus a future `sona-cli`) over
creating new top-level crates. There is no `sona-tuning` workspace member
in ruvector — sona_tuning lives at `crates/prime-radiant/src/sona_tuning/`.

## Memory substrate

```
trajectory  →  ReasoningBank  →  pattern_bridge  →  ruvllm restriction map
     ↓
  gate.rs (witness chain)  →  SHAKE-256 audit log
     ↓
  AgentDB (HNSW)  ←  embedding → semantic recall
```

ReasoningBank is the **pattern store**; AgentDB is the **vector index**;
the witness chain is the **audit anchor**. All three must agree on a
trajectory before it is considered "distilled".

## Verdict policy

A trajectory is `pass` only if **all** of the following hold:

1. Gate produced a witness chain entry (no SHAKE-256 verification error).
2. Validation step's exit code was 0.
3. The post-hoc score is strictly greater than the prior trajectory's
   score (no regressions admitted into ReasoningBank).
4. No new `unsafe` block landed without a hand review (research lint
   policy notwithstanding).

If any of those fails, the trajectory is recorded with `verdict = fail`
but **still** stored — failures train the bank too.

## Drift detection

Use the DEMOCRITUS two-tier loop in weftos's kernel
(`feature = "ecc"`) for online drift. ruvector consumes the drift
signals via `mcp-brain-server`'s `/v1/status` `drift_count` field. When
drift rises above a threshold, schedule a fresh iteration of this loop.

## Versioning + reproducibility

- Each trajectory records the ruvector workspace version + the weftos
  workspace version it ran against. Cross-repo trajectories are
  un-replayable without both pins.
- The Attractor DOT graph at `.attractor/integration.dot` is the
  authoritative spec — when the implementation diverges, update the DOT
  first, then sync the code.

## Related skills

- `cross-repo-integration` — the topology this loop runs on top of.
- `agentdb-learning` — the 9 RL algorithms available for node 4.
- `reasoningbank-intelligence` — pattern recognition + meta-cognition.
- `verification-quality` — truth-score verification used by node 3.

## Forbidden actions

- Do **not** mutate ReasoningBank entries in place — append a new
  trajectory instead, so the audit chain stays linear.
- Do **not** skip the witness-chain step in node 3 even for "trivial"
  fixes; the audit log is the contract.
- Do **not** train an RL plugin (`agentdb-learning`) on a trajectory
  set whose verdicts include unverified `pass` markers.
