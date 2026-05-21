---
name: sync-readme-and-package-json-to-published-npm
description: Workflow command scaffold for sync-readme-and-package-json-to-published-npm in ruvector.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /sync-readme-and-package-json-to-published-npm

Use this workflow when working on **sync-readme-and-package-json-to-published-npm** in `ruvector`.

## Goal

Synchronizes the README.md and package.json of an npm package to match the already-published version, ensuring the git repository reflects the npm registry state.

## Common Files

- `npm/packages/*/README.md`
- `npm/packages/*/package.json`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Verify the published npm package contents.
- Update README.md and package.json in the corresponding npm package directory.
- Commit the changes to git.

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.