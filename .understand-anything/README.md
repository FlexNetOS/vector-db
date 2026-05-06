# `.understand-anything/` — Codebase Intelligence Artifacts

This directory holds the outputs of the
[Understand-Anything](https://github.com/Lum1104/Understand-Anything) plugin
when it analyzes RuVector. The plugin itself is installed at the user/agent
level via:

```bash
./scripts/install-understand-anything.sh
```

## Generating the artifacts

From inside any agent runtime that has the plugin loaded (Claude Code, Codex,
Gemini CLI, OpenCode, Pi Agent), run one of:

| Command | Purpose | Output |
|---|---|---|
| `/understand` | Multi-agent full analysis | `knowledge-graph.json`, `intermediate/` (gitignored) |
| `/understand-onboard` | Guided onboarding tour | `onboarding.md`, `tours/` |
| `/understand-knowledge` | Build / refresh shared graph | `knowledge-graph.json` |
| `/understand-domain` | Domain-specific deep dive | merged into `knowledge-graph.json` |
| `/understand-diff` | Change-impact analysis | `diff-overlay.json` (gitignored) |

The `knowledge-graph.json` artifact is intended to be committed and shared
with the team — it is the input to the dashboard (`/understand-dashboard`) and
to downstream tooling (Attractor pipelines, GitNexus indexing,
MemPalace distillation).

## Why this lives in-repo

Per the upstream plugin design, knowledge graphs are versioned alongside the
codebase so they can be regenerated under CI and diffed in PRs. See
`.github/workflows/self-learn.yml` (added in a later phase) for the automated
regeneration job.
