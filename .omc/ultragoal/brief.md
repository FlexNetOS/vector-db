# Ruvector UI Healing — Continuation Brief

**Branch:** `feat/examples-ui-ci`
**Target:** `FlexNetOS/develop`
**Created:** 2026-05-24
**Origin session ID:** linked to UI audit session `8271cb67-b517-43c3-b0bc-7840b61329ca`

## Context

The prior session audited all 7 UI surfaces in this repo and identified that only
`ui/ruvocal/` had CI; the 6 example dashboards had none in either upstream
(`ruvnet/RuVector`) or fork (`FlexNetOS/ruvector`). The user reported "all UI
features are not working" — the structural root cause is silent rot from missing CI.

## State at start of this continuation

| Goal | Status |
|---|---|
| Add CI workflow covering 5 dashboard surfaces | DONE (commit `3a91c0fd`) — `examples-ui-ci.yml` (159 lines) |
| Excluded surface: `crates/ruvllm-wasm/src/webgpu` | REASONED — built by `ruvllm-build.yml` as cargo module, not a standalone UI |
| Heal each of 6 UI surfaces to install/build/lint/smoke-test green | TODO |
| Commit AgentDB reflexion schema fix (32.4K patch + 6 wiring files) | IN-PROGRESS, DIRTY |
| Add "User Interfaces" section to root README | TODO |
| Open PR vs FlexNetOS/develop summarizing all changes | TODO |

## Constraints (non-negotiable)

- PRs target `FlexNetOS/develop`, NOT `main`. Branch first.
- Never gitignore `.claude-flow/` or `.omc/` — committed state.
- Do not write Contributor Covenant 2.1 verbatim in parallel Write batches (content filter trips).
- Do not touch `ui/ruvocal/` TODOs — vendored from `huggingface/chat-ui` upstream.
- Verify each UI actually runs, not just compiles. Type-check pass ≠ feature working.
- Per CLAUDE.md "Boil the Ocean": tests + CI + docs. No "table this for later" PRs.

## Surfaces (6 total)

1. `examples/rvf/dashboard/` — TS / Vite 6 / Three.js. 14 view files; mock `fetchStatus()` + WebSocket for offline dev.
2. `examples/edge-net/dashboard/` — React 19 / Vite 7 / HeroUI. 3 "coming soon" stubs. Blocked on upstream #276 (wrtc darwin-arm64). Document the gap.
3. `examples/scipix/web/` — Rust→WASM. Build the missing `pkg/`. Wire `npm run dev` to build first.
4. `examples/ruvLLM/esp32-flash/web-flasher/` — Vanilla JS / Web Serial. Verify upstream #409 fix. Parameterize firmware URL. Add Chrome-only warning.
5. `crates/rvlite/examples/dashboard/` — React 19 / Vite 7. Consolidate 11 markdown design docs. Verify parent crate integration.
6. `crates/ruvllm-wasm/src/webgpu/` — Rust / WebGPU. Re-enable in CI via `wasm-pack build --target web`. No-WebGPU fallback test.

## AgentDB reflexion fix (Goal 4 from prompt)

`mcp__agentic-flow__agentdb_pattern_store` had 3 failures (ETIMEDOUT, "no such table: episodes", shell-escape bug). The in-progress patch adds `node_modules/agentdb/dist/schemas/frontier-schema.sql` via `patch-package`. Must verify the high-fidelity tool works post-patch OR document gap and fall back to `mcp__claude-flow__agentdb_pattern-store`.

## Definition of done

- All 6 dashboards green: install + build + lint + smoke test
- `examples-ui-ci.yml` passes on a PR to develop
- Root README has "User Interfaces" section linking all 7 UIs
- AgentDB reflexion either fixed OR documented as won't-fix
- PR opened against `FlexNetOS/develop`
- Auto-memory updated with anything new learned
