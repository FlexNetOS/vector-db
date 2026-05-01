#!/usr/bin/env python3
"""Detect duplicate / near-duplicate / contradictory markdown across repos.

Usage:
    python3 detect_dups.py <repo_root> <repo_label>

Outputs JSON to stdout containing:
    - exact_duplicates: groups of files with identical normalized content
    - near_duplicates:  pairs with Jaccard(shingles) >= 0.85
    - contradictions:   files that pair-wise mention conflicting "version X.Y.Z" / counts / numerics
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Iterable

EXCLUDE_DIRS = {".git", "node_modules", "target", ".cargo", "dist", "build",
                ".next", ".turbo", ".pnpm-store", ".planning"}
# .planning in weftos is session-scratch, not knowledge.

WORD_RE = re.compile(r"[A-Za-z0-9_]+")


def iter_md_files(root: Path) -> Iterable[Path]:
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
        for fn in filenames:
            if fn.lower().endswith(".md"):
                yield Path(dirpath) / fn


def normalize(text: str) -> str:
    # Strip front-matter, trailing whitespace, blank lines.
    text = re.sub(r"^---\n.*?\n---\n", "", text, flags=re.S)
    text = re.sub(r"[ \t]+\n", "\n", text)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip().lower()


def shingles(text: str, k: int = 7) -> set[str]:
    words = WORD_RE.findall(text)
    if len(words) < k:
        return {" ".join(words)} if words else set()
    return {" ".join(words[i : i + k]) for i in range(len(words) - k + 1)}


def jaccard(a: set, b: set) -> float:
    if not a or not b:
        return 0.0
    inter = len(a & b)
    union = len(a | b)
    return inter / union if union else 0.0


# Patterns we will scan for cross-doc contradictions.
NUMERIC_CLAIM_PATTERNS = [
    # Performance numbers
    (re.compile(r"(\d+(?:\.\d+)?)\s*x\s*(?:faster|speedup)", re.I), "speedup"),
    (re.compile(r"(\d+(?:\.\d+)?)\s*ms\b", re.I), "latency_ms"),
    # Counts
    (re.compile(r"\b(\d{1,3})\s+(?:agents?|crates?|skills?|workflows?|domains?)\b", re.I), "count"),
    # Versions of named projects
    (re.compile(r"\bruvector\s+v?(\d+\.\d+(?:\.\d+)?)", re.I), "ruvector_version"),
    (re.compile(r"\bweftos\s+v?(\d+\.\d+(?:\.\d+)?)", re.I), "weftos_version"),
    (re.compile(r"\brust\s+(\d+\.\d+(?:\.\d+)?)", re.I), "rust_version"),
    (re.compile(r"\bnode(?:\.js)?\s+(\d+(?:\.\d+)?)", re.I), "node_version"),
]


def extract_claims(text: str) -> dict[str, set[str]]:
    claims: dict[str, set[str]] = defaultdict(set)
    for pat, label in NUMERIC_CLAIM_PATTERNS:
        for m in pat.finditer(text):
            claims[label].add(m.group(1))
    return claims


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: detect_dups.py <repo_root> <repo_label>", file=sys.stderr)
        return 2
    root = Path(sys.argv[1]).resolve()
    label = sys.argv[2]

    files = list(iter_md_files(root))
    print(f"# scanned {len(files)} markdown files in {label}", file=sys.stderr)

    by_hash: dict[str, list[str]] = defaultdict(list)
    file_text: dict[str, str] = {}
    file_shingles: dict[str, set[str]] = {}
    file_claims: dict[str, dict[str, set[str]]] = {}
    file_size: dict[str, int] = {}

    for p in files:
        try:
            raw = p.read_text(errors="replace")
        except Exception:
            continue
        norm = normalize(raw)
        if len(norm) < 200:
            # too small to be meaningful "knowledge" — skip from clustering
            continue
        rel = str(p.relative_to(root))
        h = hashlib.sha256(norm.encode("utf-8", errors="replace")).hexdigest()
        by_hash[h].append(rel)
        file_text[rel] = norm
        file_shingles[rel] = shingles(norm)
        file_claims[rel] = extract_claims(raw)
        file_size[rel] = len(norm)

    exact_dups = [
        {"hash": h, "files": sorted(group), "size_chars": file_size[group[0]]}
        for h, group in by_hash.items() if len(group) > 1
    ]
    exact_dups.sort(key=lambda d: -len(d["files"]))

    # Pairwise near-dup detection — bucket by 50-shingle minhash signature to keep cheap.
    near_pairs: list[dict] = []
    files_for_pairs = list(file_shingles.keys())
    # Bucketing: signature = sorted minhash of first N shingles using hash mod
    BUCKETS = 64
    bucket_index: dict[int, list[str]] = defaultdict(list)
    for f in files_for_pairs:
        sig_words = sorted(file_shingles[f])[:50]
        if not sig_words:
            continue
        b = hash(sig_words[0]) % BUCKETS
        bucket_index[b].append(f)
    seen = set()
    for b, members in bucket_index.items():
        for i in range(len(members)):
            for j in range(i + 1, len(members)):
                a, c = members[i], members[j]
                key = (a, c) if a < c else (c, a)
                if key in seen:
                    continue
                seen.add(key)
                sim = jaccard(file_shingles[a], file_shingles[c])
                if sim >= 0.7 and a != c:
                    # Also check it isn't already an exact dup pair
                    if file_text[a] == file_text[c]:
                        continue
                    near_pairs.append({"a": a, "b": c, "jaccard": round(sim, 3)})
    # Cross-bucket scan for high-similarity pairs is expensive; we additionally
    # do a coarse scan based on first 200 chars to catch missed buckets.
    by_prefix: dict[str, list[str]] = defaultdict(list)
    for f, t in file_text.items():
        by_prefix[t[:200]].append(f)
    for grp in by_prefix.values():
        if len(grp) < 2:
            continue
        for i in range(len(grp)):
            for j in range(i + 1, len(grp)):
                a, c = grp[i], grp[j]
                key = (a, c) if a < c else (c, a)
                if key in seen:
                    continue
                seen.add(key)
                if file_text[a] == file_text[c]:
                    continue
                sim = jaccard(file_shingles[a], file_shingles[c])
                if sim >= 0.7:
                    near_pairs.append({"a": a, "b": c, "jaccard": round(sim, 3)})
    near_pairs.sort(key=lambda d: -d["jaccard"])

    # Contradictions — pairs of files with overlapping topic words that disagree
    # on the same numeric label (e.g. one says ruvector_version=0.1 other 0.3).
    contradictions: list[dict] = []
    by_label_value: dict[str, dict[str, set[str]]] = defaultdict(lambda: defaultdict(set))
    for f, claims in file_claims.items():
        for label_, vals in claims.items():
            for v in vals:
                by_label_value[label_][v].add(f)
    for label_, value_map in by_label_value.items():
        if label_ in {"speedup", "latency_ms", "count"}:
            # Too noisy — many docs cite different speedups for different ops.
            continue
        if len(value_map) < 2:
            continue
        # If multiple distinct values, any file from value A vs file from value B
        # is a contradiction candidate.
        values = list(value_map.keys())
        for i in range(len(values)):
            for j in range(i + 1, len(values)):
                v1, v2 = values[i], values[j]
                contradictions.append({
                    "label": label_,
                    "value_a": v1,
                    "value_b": v2,
                    "files_a": sorted(value_map[v1])[:5],
                    "files_b": sorted(value_map[v2])[:5],
                })

    out = {
        "repo": label,
        "total_md": len(files),
        "considered_md": len(file_text),
        "exact_duplicate_groups": exact_dups,
        "near_duplicate_pairs": near_pairs,
        "potential_contradictions": contradictions,
    }
    json.dump(out, sys.stdout, indent=2)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
