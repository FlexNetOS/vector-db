#!/usr/bin/env python3
"""Replace exact-duplicate doc files with redirect stubs.

For each (canonical, duplicate) pair, the duplicate file is overwritten
with a 4-line stub linking to the canonical file. Original content is
*not* deleted from history (it remains in `git log`) and is reproduced
verbatim in `docs/audit/AUDIT_REPORT.md` so no information is lost.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path


def repo_relpath(canonical: Path, duplicate: Path) -> str:
    """Compute the canonical path relative to the duplicate's directory."""
    return os.path.relpath(canonical, duplicate.parent)


def stub_for(canonical_rel_to_root: str, repo_label: str) -> str:
    return (
        f"<!-- consolidated -->\n"
        f"# Moved\n\n"
        f"This document was consolidated during the {repo_label} knowledge-base\n"
        f"audit (see `docs/audit/AUDIT_REPORT.md`).\n\n"
        f"**Canonical location:** [`{canonical_rel_to_root}`](/{canonical_rel_to_root})\n\n"
        f"All previous content is preserved in git history and at the canonical\n"
        f"location. If you believe consolidation was incorrect, see\n"
        f"`docs/audit/MANUAL_REVIEW.md`.\n"
    )


def consolidate(root: Path, label: str, pairs: list[tuple[str, str]]) -> None:
    for canonical, duplicate in pairs:
        c = root / canonical
        d = root / duplicate
        if not c.exists():
            print(f"  SKIP missing canonical: {canonical}")
            continue
        if not d.exists():
            print(f"  SKIP missing duplicate: {duplicate}")
            continue
        d.write_text(stub_for(canonical, label))
        print(f"  stub  {duplicate} -> {canonical}")


RUVECTOR_PAIRS = [
    # examples/dragnes/docs/* mirrors docs/research/DrAgnes/*
    ("docs/research/DrAgnes/architecture.md",       "examples/dragnes/docs/architecture.md"),
    ("docs/research/DrAgnes/competitive-analysis.md","examples/dragnes/docs/competitive-analysis.md"),
    ("docs/research/DrAgnes/data-sources.md",       "examples/dragnes/docs/data-sources.md"),
    ("docs/research/DrAgnes/deployment.md",         "examples/dragnes/docs/deployment.md"),
    ("docs/research/DrAgnes/dermlite-integration.md","examples/dragnes/docs/dermlite-integration.md"),
    ("docs/research/DrAgnes/future-vision.md",      "examples/dragnes/docs/future-vision.md"),
    ("docs/research/DrAgnes/HAM10000_analysis.md",  "examples/dragnes/docs/HAM10000_analysis.md"),
    ("docs/research/DrAgnes/hipaa-compliance.md",   "examples/dragnes/docs/hipaa-compliance.md"),
    ("docs/research/DrAgnes/README.md",             "examples/dragnes/docs/README.md"),
    # patches/hnsw_rs duplicated under scripts/patches
    ("patches/hnsw_rs/README.md",  "scripts/patches/hnsw_rs/README.md"),
    ("patches/hnsw_rs/Changes.md", "scripts/patches/hnsw_rs/Changes.md"),
    # research paper mirrored in example
    ("docs/research/cognitive-frontier/delta-behavior-computational-paradigm.md",
     "examples/delta-behavior/research/THEORETICAL-FOUNDATIONS.md"),
]


WEFTOS_PAIRS: list[tuple[str, str]] = [
    # The agents/ copy mirrors the canonical .claude/skills/ copy.
    (".claude/skills/weftos-ecc/WEAVER.md", "agents/weftos-ecc/WEAVER.md"),
]


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in {"ruvector", "weftos"}:
        print("usage: consolidate.py {ruvector|weftos}", file=sys.stderr)
        return 2
    label = sys.argv[1]
    root = Path(f"/home/ubuntu/repos/{label}")
    pairs = RUVECTOR_PAIRS if label == "ruvector" else WEFTOS_PAIRS
    print(f"# consolidating {len(pairs)} pair(s) in {label}")
    consolidate(root, label, pairs)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
