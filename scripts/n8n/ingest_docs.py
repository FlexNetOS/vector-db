#!/usr/bin/env python3
"""Walk a repo's markdown tree and ingest each chunk into RuVector.

Two modes:

* `--target ruvector` (default): POST each chunk directly to the RuVector
  REST API at `RUVECTOR_BASE_URL/v1/memories`.
* `--target n8n`: POST chunks to the n8n batch ingest webhook at
  `N8N_INGEST_URL` (default `http://localhost:5678/webhook/kb-ingest/batch`).
  The n8n workflow then forwards them to RuVector — letting you insert
  pre/post processing steps without changing this script.

Usage:

    python3 scripts/n8n/ingest_docs.py --root . --namespace ruvector-docs
    python3 scripts/n8n/ingest_docs.py --root . --target n8n --dry-run
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

EXCLUDE_DIRS = {
    ".git", "node_modules", "target", ".cargo", "dist", "build",
    ".next", ".turbo", ".pnpm-store", ".planning",
}

CHUNK_TARGET_CHARS = 1500   # roughly 200-400 tokens
CHUNK_OVERLAP = 200


def iter_md(root: Path):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
        for fn in filenames:
            if fn.lower().endswith(".md"):
                yield Path(dirpath) / fn


def chunk_markdown(text: str) -> list[str]:
    """Section-aware chunker: split on H1/H2/H3, then size-bound."""
    # Strip front-matter
    text = re.sub(r"^---\n.*?\n---\n", "", text, flags=re.S)
    sections = re.split(r"(?m)^(?=#{1,3}\s)", text)
    chunks: list[str] = []
    for sec in sections:
        sec = sec.strip()
        if not sec:
            continue
        if len(sec) <= CHUNK_TARGET_CHARS:
            chunks.append(sec)
            continue
        # Slide a window across the section
        i = 0
        while i < len(sec):
            chunks.append(sec[i : i + CHUNK_TARGET_CHARS])
            i += CHUNK_TARGET_CHARS - CHUNK_OVERLAP
    return [c for c in chunks if len(c) >= 80]


def post_json(url: str, payload: dict, headers: dict | None = None, timeout: float = 30.0) -> dict:
    body = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=body,
        headers={"content-type": "application/json", **(headers or {})},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def ingest_one(base_url: str, namespace: str, path: Path, content: str, headers: dict, dry: bool) -> int:
    pieces = chunk_markdown(content)
    n = 0
    for i, piece in enumerate(pieces):
        payload = {
            "content": piece,
            "category": "documentation",
            "namespace": namespace,
            "tags": ["audit-pipeline", f"path:{path.as_posix()}"],
            "metadata": {
                "source_path": path.as_posix(),
                "chunk_index": i,
                "chunk_count": len(pieces),
            },
        }
        if dry:
            print(json.dumps({"would_post": f"{base_url}/v1/memories", "payload_preview": payload["content"][:60]}))
            n += 1
            continue
        try:
            res = post_json(f"{base_url}/v1/memories", payload, headers)
            if res.get("deduplicated"):
                continue
            n += 1
        except urllib.error.HTTPError as e:
            sys.stderr.write(f"  HTTP {e.code} for {path}: {e.read()[:200]!r}\n")
        except Exception as e:
            sys.stderr.write(f"  ERR {type(e).__name__} for {path}: {e}\n")
    return n


def ingest_via_n8n(url: str, namespace: str, docs: list[dict], dry: bool) -> int:
    if dry:
        print(json.dumps({"would_post": url, "doc_count": len(docs)}))
        return len(docs)
    res = post_json(url, {"namespace": namespace, "docs": docs})
    return int(res.get("ingested", 0))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", default=".", help="repo root to scan")
    ap.add_argument("--namespace", default=os.environ.get("RUVECTOR_NAMESPACE", "ruvector-docs"))
    ap.add_argument(
        "--target",
        choices=["ruvector", "n8n"],
        default=os.environ.get("INGEST_TARGET", "ruvector"),
    )
    ap.add_argument("--ruvector-url", default=os.environ.get("RUVECTOR_BASE_URL", "http://localhost:8080"))
    ap.add_argument("--n8n-url", default=os.environ.get("N8N_INGEST_URL", "http://localhost:5678/webhook/kb-ingest/batch"))
    ap.add_argument("--max-files", type=int, default=int(os.environ.get("INGEST_MAX_FILES", "0")), help="0 = no limit")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    root = Path(args.root).resolve()
    headers: dict = {}
    if "RUVECTOR_API_KEY" in os.environ:
        headers["authorization"] = f"Bearer {os.environ['RUVECTOR_API_KEY']}"

    files = list(iter_md(root))
    if args.max_files:
        files = files[: args.max_files]

    total_chunks = 0
    if args.target == "ruvector":
        for p in files:
            try:
                content = p.read_text(errors="replace")
            except Exception as e:
                sys.stderr.write(f"  read error {p}: {e}\n")
                continue
            n = ingest_one(args.ruvector_url, args.namespace, p.relative_to(root), content, headers, args.dry_run)
            total_chunks += n
        print(json.dumps({"target": "ruvector", "files": len(files), "chunks_ingested": total_chunks, "namespace": args.namespace}, indent=2))
        return 0

    # target = n8n: send a batch
    docs = []
    for p in files:
        try:
            content = p.read_text(errors="replace")
        except Exception:
            continue
        for i, piece in enumerate(chunk_markdown(content)):
            docs.append({
                "content": piece,
                "tags": [f"path:{p.relative_to(root).as_posix()}"],
                "metadata": {"source_path": p.relative_to(root).as_posix(), "chunk_index": i},
            })
    n = ingest_via_n8n(args.n8n_url, args.namespace, docs, args.dry_run)
    print(json.dumps({"target": "n8n", "files": len(files), "chunks_posted": len(docs), "ingested": n, "namespace": args.namespace}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
