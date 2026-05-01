#!/usr/bin/env python3
"""Import the workflow JSON files in `scripts/n8n/workflows/` into a running
n8n instance and activate them.

Uses n8n's REST API (auth-disabled by default per the dev `docker-compose.yml`).
For an authenticated n8n, set `N8N_API_KEY` and the script will send it as
`X-N8N-API-KEY`.
"""
from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

N8N_HOST = os.environ.get("N8N_HOST", "http://localhost:5678").rstrip("/")
API_KEY = os.environ.get("N8N_API_KEY")

WORKFLOW_DIR = Path(__file__).parent / "workflows"


def http(method: str, path: str, body: dict | None = None) -> dict:
    url = f"{N8N_HOST}{path}"
    data = json.dumps(body).encode("utf-8") if body is not None else None
    headers = {"content-type": "application/json", "accept": "application/json"}
    if API_KEY:
        headers["X-N8N-API-KEY"] = API_KEY
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            text = resp.read().decode("utf-8") or "{}"
            return json.loads(text)
    except urllib.error.HTTPError as e:
        text = e.read().decode("utf-8", errors="replace")
        try:
            return {"_status": e.code, **json.loads(text)}
        except Exception:
            return {"_status": e.code, "_text": text}


def find_workflow_by_name(name: str) -> dict | None:
    res = http("GET", "/rest/workflows")
    items = res.get("data", res) if isinstance(res, dict) else res
    if isinstance(items, dict):
        items = items.get("data", [])
    for w in items or []:
        if w.get("name") == name:
            return w
    return None


def import_one(path: Path) -> dict:
    raw = json.loads(path.read_text())
    payload = {
        "name": raw["name"],
        "nodes": raw["nodes"],
        "connections": raw["connections"],
        "settings": raw.get("settings", {"executionOrder": "v1"}),
    }
    existing = find_workflow_by_name(raw["name"])
    if existing and existing.get("id"):
        wid = existing["id"]
        res = http("PATCH", f"/rest/workflows/{wid}", payload)
        if "_status" in res:
            res = http("PUT", f"/rest/workflows/{wid}", payload)
        action = "updated"
    else:
        res = http("POST", "/rest/workflows", payload)
        action = "created"
        wid = (res.get("data") or res).get("id")
    if wid:
        http("POST", f"/rest/workflows/{wid}/activate", {})
    return {"file": path.name, "action": action, "id": wid, "response_keys": sorted(list(res.keys())) if isinstance(res, dict) else []}


def main() -> int:
    if not WORKFLOW_DIR.exists():
        print(f"workflow dir not found: {WORKFLOW_DIR}", file=sys.stderr)
        return 1
    results = []
    for p in sorted(WORKFLOW_DIR.glob("*.json")):
        results.append(import_one(p))
    print(json.dumps({"n8n": N8N_HOST, "imported": results}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
