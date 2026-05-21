---
name: understand-anything-slice-pipeline
description: Hard-won orchestration pattern for RuVector's .understand-anything per-slice pipeline; covers scanner schema mismatch, intermediate-vs-tmp path confusion, cwd persistence in Bash, and the canonical 9-step flow per sub-slice.
triggers:
  - "understand-anything"
  - "slice pipeline"
  - "slice-10"
  - "merge-subdomain-graphs"
  - "fingerprints.slice"
  - "knowledge-graph.json"
  - "project-scanner schema"
  - "file-analyzer batch"
  - "intermediate/ vs tmp/"
---

# Understand-Anything Slice Pipeline (RuVector)

## The Insight

The `.understand-anything` pipeline has THREE separate file-system contracts that look identical from the outside but behave differently, and the project-scanner schema **changed between slice 6 and slice 10** without a migration. If you run the pipeline assuming a single shape, the merger silently drops data or crashes mid-pipeline.

The principle: **never let the schema or output directory be implicit**. State both explicitly in every agent prompt, and verify with `Read`/`ls` after each agent returns.

## Why This Matters

Three concrete failures observed in this codebase:

1. **Fingerprints schema drift.** Slice 6 scanners produced `{files: {<path>: {...}}}` (dict). Slice 10 scanners produced `{slice, scope, generatedAt, totalFiles, files: [...]}` (list-of-objects). The original `merge-fingerprints.py` calls `.items()` on `files` and crashes with `AttributeError: 'list' object has no attribute 'items'`. There is no version field that distinguishes them — only structure.

2. **File-analyzers write to the wrong directory.** When the prompt says "write to `.understand-anything/tmp/slice-<N>-batch-<M>-graph.json`" but doesn't say "absolute path", some agents resolve relative paths against `.understand-anything/intermediate/` (which exists because graph-reviewer needs symlinks of prior slice graphs there). Outputs vanish from where you expect them. Specifically observed: slice 10f wave 8-14 landed in `intermediate/`, slice 10c batches 10-13 also landed in `intermediate/`.

3. **Bash cwd persists across calls in the same session.** A single `cd .understand-anything/intermediate` (used to create symlinks) silently shifts cwd for every subsequent `ls` and `python3` command until you `cd` back. The next status check returns `exit 2` and zero counts, and you start chasing imaginary failures.

## Recognition Pattern

You're inside this pattern's failure mode when you see:

- `AttributeError: 'list' object has no attribute 'items'` from a fingerprints merge → schema drift.
- An agent reports "wrote N batch files" but `ls .understand-anything/tmp/slice-<X>-batch-*-graph.json | wc -l` returns fewer → check `intermediate/` for the missing files.
- `ls` exit 2, monitor says `0/N` but agents reported success → cwd is wrong, run `pwd && cd /home/drdave/repos/RuVector` first.
- `graph-reviewer` agent crashes with `ENOENT: ... .understand-anything/intermediate/slice-Nc-knowledge-graph.json` → reviewer reads sibling slices from `intermediate/`, you need symlinks.
- Tour-builder validates but reports nodes counts mismatching layered → cleanup pass needed (drop dangling edges, normalize `fn→function`, `doc→document`, `mod→module`).

## The Approach

Apply this 9-step pipeline per sub-slice. Always in this order. Always with absolute paths in agent prompts.

```
1. BACKUP   .understandignore, fingerprints.json, knowledge-graph.json, meta.json → *.bak-pre-slice<X>
2. SCOPE    Write .understandignore restricted to the sub-slice's crates only
3. SCAN     Dispatch understand-anything:project-scanner — produces fingerprints.slice-<X>.json + tmp/slice-<X>-all-file-paths.json
            The paths file is a dict: {scope, totalFiles, paths: [...]} — extract d['paths'], not the dict
4. BATCH    Split paths into ~22-25 files per batch; write tmp/slice-<X>-batch-<NN>.json
5. ANALYZE  Spawn understand-anything:file-analyzer agents in waves of 4-5 parallel
            Tell agents to write to ABSOLUTE paths; verify with ls
            If files land in intermediate/, move them: for f in intermediate/slice-<X>-*; do mv "$f" tmp/; done
6. MERGE    python3 merge-slice-batches.py <X> tmp/slice-<X>-assembled.json
7. LAYER    Dispatch architecture-analyzer (skip assemble-reviewer for speed)
8. TOUR     Dispatch tour-builder against the layered graph
9. CLEAN    Python pass: drop dangling edges and self-loops, normalize types. Save as slice-<X>-final.json AND copy to .understand-anything/slice-<X>-knowledge-graph.json. Symlink into intermediate/ for future graph-reviewer reads.
```

After all sub-slices done:

```
10. FINGERPRINTS  Run merge-fingerprints-v2.py (handles BOTH dict and list schemas)
11. MASTER MERGE  Run Understand-Anything/.../merge-subdomain-graphs.py — auto-discovers slice-*-knowledge-graph.json
12. META          Update meta.json with new slicesPresent, remaining, analyzedFiles count
```

Performance: 12 parallel file-analyzers process a 300-file slice in ~5 minutes wall clock. Pre-prep batch files for the next sub-slice while the current one runs.

Budget: Each sub-slice consumes ~10-15k tokens of context in agent summaries alone. A 6-sub-slice slice 10 run uses ~80-100k tokens; plan to span compaction.

## Example

The schema-drift fix that unblocked slice 10's fingerprint merge:

```python
files = d.get("files", {})
if isinstance(files, dict):
    items = files.items()
elif isinstance(files, list):
    items = [(f.get("path") or f.get("filePath"), f) for f in files if (f.get("path") or f.get("filePath"))]

for path, fp in items:
    fp.setdefault("filePath", fp.get("path", path))
    fp.setdefault("contentHash", fp.get("sha1", ""))
    fp.setdefault("functions", []); fp.setdefault("classes", [])
    fp.setdefault("imports", []); fp.setdefault("exports", [])
    fp.setdefault("totalLines", fp.get("sizeLines", 0))
    fp.setdefault("hasStructuralAnalysis", False)
    master_files[path] = fp
```

The lesson is not the code — it's that **two pipeline generations of the same agent produce schemas that look interchangeable but aren't**, and the merger needs to detect-and-coerce rather than assume.
