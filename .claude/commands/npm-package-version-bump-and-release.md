---
name: npm-package-version-bump-and-release
description: Workflow command scaffold for npm-package-version-bump-and-release in ruvector.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /npm-package-version-bump-and-release

Use this workflow when working on **npm-package-version-bump-and-release** in `ruvector`.

## Goal

Bumps the version of npm packages (meta and platform-specific) to surface new features or fixes, ensuring all packages are in sync for release.

## Common Files

- `npm/packages/router/package.json`
- `npm/packages/router-darwin-arm64/package.json`
- `npm/packages/router-darwin-x64/package.json`
- `npm/packages/router-linux-arm64-gnu/package.json`
- `npm/packages/router-linux-x64-gnu/package.json`
- `npm/packages/router-win32-x64-msvc/package.json`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Update version numbers in all relevant npm package.json files (meta and platform-specific).
- Commit the version bump.
- Publish to npm (handled by CI after tag is cut).

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.