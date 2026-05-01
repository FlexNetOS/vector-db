#!/usr/bin/env python3
"""Tiny RuVector-compatible memory shim for local n8n development.

Implements the subset of the `mcp-brain-server` REST surface that the n8n
workflows in this directory talk to:

    POST   /v1/memories            store a memory (returns id)
    GET    /v1/memories/search     k-NN over hash-of-bag-of-words embedding
    GET    /v1/memories/list       list recent memories
    GET    /v1/memories/{id}       fetch by id
    DELETE /v1/memories/{id}       delete
    GET    /v1/health              liveness
    GET    /v1/status              counts

This is **not** a production RuVector — it only exists so that the n8n
workflows can be tested end-to-end without building the full Rust stack.
For real workloads, swap `RUVECTOR_BASE_URL` to point at:

  • `mcp-brain-server-local` (Rust + SQLite + HNSW), or
  • the cloud `mcp-brain-server` at https://pi.ruv.io

The embedding here is a deterministic 256-dim float vector built from
SHA-256 bytes of a normalised bag-of-words; it is faithful enough for
sanity-checking retrieval logic and doc-change re-ingest, but should not
be confused with a learned semantic embedding.
"""
from __future__ import annotations

import hashlib
import json
import math
import os
import re
import threading
import time
import uuid
from pathlib import Path
from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field

DATA_DIR = Path(os.environ.get("RUVECTOR_DATA_DIR", "/data"))
DATA_DIR.mkdir(parents=True, exist_ok=True)
DB_PATH = DATA_DIR / "memories.jsonl"

EMBED_DIM = 256
WORD_RE = re.compile(r"[A-Za-z0-9_]{2,}")

_lock = threading.Lock()


def _embed(text: str) -> list[float]:
    """Deterministic, dependency-free dense embedding (NOT semantic).

    Builds a fixed-dim vector by hashing each token into one of EMBED_DIM
    slots and L2-normalising. This replicates the *shape* of a real
    embedding so the retrieval workflow can be exercised end-to-end.
    """
    vec = [0.0] * EMBED_DIM
    tokens = WORD_RE.findall(text.lower())
    if not tokens:
        return vec
    for tok in tokens:
        h = hashlib.sha256(tok.encode("utf-8")).digest()
        idx = int.from_bytes(h[:4], "little") % EMBED_DIM
        sign = 1.0 if (h[4] & 1) else -1.0
        vec[idx] += sign
    norm = math.sqrt(sum(x * x for x in vec)) or 1.0
    return [x / norm for x in vec]


def _cosine(a: list[float], b: list[float]) -> float:
    return sum(x * y for x, y in zip(a, b))


class CreateMemory(BaseModel):
    content: str
    category: str = "general"
    namespace: str = "default"
    tags: list[str] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class Memory(BaseModel):
    id: str
    content: str
    category: str
    namespace: str
    tags: list[str]
    metadata: dict[str, Any]
    embedding: list[float]
    created_at: float
    content_hash: str


_memories: list[Memory] = []


def _persist() -> None:
    tmp = DB_PATH.with_suffix(".tmp")
    with tmp.open("w") as fh:
        for m in _memories:
            fh.write(m.model_dump_json() + "\n")
    tmp.replace(DB_PATH)


def _load() -> None:
    if not DB_PATH.exists():
        return
    with DB_PATH.open() as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                _memories.append(Memory(**json.loads(line)))
            except Exception:
                continue


_load()


app = FastAPI(title="ruvector-shim", version="0.1.0")


@app.get("/v1/health")
def health() -> dict[str, str]:
    return {"status": "ok"}


@app.get("/v1/status")
def status() -> dict[str, Any]:
    by_ns: dict[str, int] = {}
    for m in _memories:
        by_ns[m.namespace] = by_ns.get(m.namespace, 0) + 1
    return {
        "total": len(_memories),
        "namespaces": by_ns,
        "embed_dim": EMBED_DIM,
        "backend": "ruvector-shim (NOT for production use)",
    }


@app.post("/v1/memories", status_code=201)
def create_memory(req: CreateMemory) -> dict[str, Any]:
    content_hash = hashlib.sha256(req.content.encode("utf-8")).hexdigest()
    with _lock:
        # Deduplicate by (namespace, content_hash)
        for m in _memories:
            if m.namespace == req.namespace and m.content_hash == content_hash:
                return {"id": m.id, "deduplicated": True, "content_hash": content_hash}
        m = Memory(
            id=uuid.uuid4().hex,
            content=req.content,
            category=req.category,
            namespace=req.namespace,
            tags=list(req.tags),
            metadata=dict(req.metadata),
            embedding=_embed(req.content),
            created_at=time.time(),
            content_hash=content_hash,
        )
        _memories.append(m)
        _persist()
    return {"id": m.id, "content_hash": m.content_hash, "created_at": m.created_at}


@app.get("/v1/memories/search")
def search(q: str, k: int = 5, namespace: str | None = None) -> dict[str, Any]:
    if not q:
        raise HTTPException(400, "missing q")
    qv = _embed(q)
    scored: list[tuple[float, Memory]] = []
    for m in _memories:
        if namespace and m.namespace != namespace:
            continue
        scored.append((_cosine(qv, m.embedding), m))
    scored.sort(key=lambda t: -t[0])
    hits = [
        {
            "id": m.id,
            "score": round(score, 6),
            "namespace": m.namespace,
            "category": m.category,
            "tags": m.tags,
            "metadata": m.metadata,
            "snippet": m.content[:512],
        }
        for score, m in scored[: max(1, min(k, 50))]
    ]
    return {"query": q, "k": k, "namespace": namespace, "hits": hits}


@app.get("/v1/memories/list")
def list_memories(namespace: str | None = None, limit: int = 50) -> dict[str, Any]:
    items = [m for m in _memories if (namespace is None or m.namespace == namespace)]
    items.sort(key=lambda m: -m.created_at)
    items = items[: max(1, min(limit, 1000))]
    return {
        "total": len(items),
        "items": [
            {
                "id": m.id,
                "namespace": m.namespace,
                "category": m.category,
                "tags": m.tags,
                "snippet": m.content[:200],
                "created_at": m.created_at,
            }
            for m in items
        ],
    }


@app.get("/v1/memories/{mid}")
def get_memory(mid: str) -> dict[str, Any]:
    for m in _memories:
        if m.id == mid:
            return m.model_dump()
    raise HTTPException(404, "not found")


@app.delete("/v1/memories/{mid}")
def delete_memory(mid: str) -> dict[str, Any]:
    with _lock:
        for i, m in enumerate(_memories):
            if m.id == mid:
                _memories.pop(i)
                _persist()
                return {"deleted": True, "id": mid}
    raise HTTPException(404, "not found")


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        "ruvector_shim:app",
        host="0.0.0.0",
        port=int(os.environ.get("PORT", "8080")),
        reload=False,
    )
