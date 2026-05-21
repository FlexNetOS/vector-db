#!/usr/bin/env node
// Funnels .understand-anything/ and .omc/ artifacts into the repo's memory
// layer. Writes one structured JSON memory entry per artifact under
// .claude/memory/funneled/<namespace>/<safe-key>.json plus a manifest.json
// index that lists every entry with tags + summary for cheap search.
//
// Best-effort: also tries `claude-flow memory store` so entries land in
// the AgentDB-backed memory if the CLI runtime is healthy. File-based
// entries are the source of truth — CLI is opportunistic.
//
// Run from repo root:  node scripts/funnel-to-memory.mjs

import {
  readFileSync,
  statSync,
  readdirSync,
  mkdirSync,
  writeFileSync,
  existsSync,
} from "node:fs";
import { execFileSync } from "node:child_process";
import { join, relative, dirname } from "node:path";

const REPO_ROOT = process.cwd();
const OUT_DIR = join(REPO_ROOT, ".claude", "memory", "funneled");
const MANIFEST_PATH = join(OUT_DIR, "manifest.json");
const HEAD_EXCERPT_BYTES = 4_000;
const TRY_CLI = process.env.FUNNEL_TRY_CLI === "1";

function walk(dir, out = []) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const e of entries) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      if (/^(tmp|intermediate|\.omc|node_modules)$/.test(e.name)) continue;
      walk(p, out);
    } else if (e.isFile()) {
      out.push(p);
    }
  }
  return out;
}

function shouldSkip(relPath) {
  return (
    /\.bak-pre-slice/.test(relPath) ||
    /\.understandignore\.combined-slice/.test(relPath) ||
    /^\.omc\/state\/last-tool-error\.json$/.test(relPath) ||
    /SLICE-\d+-PLAN\.md$/.test(relPath)
  );
}

function deriveNamespace(relPath) {
  if (relPath.startsWith(".omc/skills/")) return "omc.skills";
  if (relPath.startsWith(".omc/sessions/")) return "omc.sessions";
  if (relPath.startsWith(".omc/state/")) return "omc.state";
  if (relPath === ".omc/project-memory.json") return "omc.project";
  if (/knowledge-graph\.json$/.test(relPath)) return "ua.knowledge-graph";
  if (/fingerprints.*\.json$/.test(relPath)) return "ua.fingerprints";
  if (relPath === ".understand-anything/meta.json") return "ua.meta";
  if (relPath === ".understand-anything/README.md") return "ua.docs";
  return "ua.other";
}

function safeKey(relPath) {
  return relPath.replace(/[^a-zA-Z0-9._-]/g, "_");
}

function summarizeJson(content, relPath) {
  let parsed;
  try {
    parsed = JSON.parse(content);
  } catch {
    return { kind: "json-unparseable", head: content.slice(0, HEAD_EXCERPT_BYTES) };
  }

  const summary = {
    kind: "json",
    topLevelKeys: Object.keys(parsed || {}).slice(0, 30),
  };

  if (/fingerprints.*\.json$/.test(relPath) && parsed && parsed.files) {
    const fileKeys = Object.keys(parsed.files);
    summary.kind = "fingerprints";
    summary.gitCommitHash = parsed.gitCommitHash;
    summary.generatedAt = parsed.generatedAt;
    summary.version = parsed.version;
    summary.fileCount = fileKeys.length;
    summary.sampleFiles = fileKeys.slice(0, 20);
  } else if (/knowledge-graph\.json$/.test(relPath) && parsed) {
    summary.kind = "knowledge-graph";
    summary.project = parsed.project || null;
    if (Array.isArray(parsed.nodes)) summary.nodeCount = parsed.nodes.length;
    if (Array.isArray(parsed.edges)) summary.edgeCount = parsed.edges.length;
    if (parsed.metadata) summary.metadata = parsed.metadata;
    if (Array.isArray(parsed.nodes)) {
      const types = {};
      for (const n of parsed.nodes.slice(0, 5000)) {
        const t = n.type || n.kind || "unknown";
        types[t] = (types[t] || 0) + 1;
      }
      summary.nodeTypes = types;
    }
  } else if (parsed && typeof parsed === "object") {
    const flat = {};
    for (const [k, v] of Object.entries(parsed)) {
      if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") {
        flat[k] =
          typeof v === "string" && v.length > 200 ? v.slice(0, 200) + "…" : v;
      } else if (Array.isArray(v)) {
        flat[`${k}_length`] = v.length;
      } else if (v && typeof v === "object") {
        flat[`${k}_keys`] = Object.keys(v).slice(0, 30);
      }
    }
    summary.flat = flat;
  }

  return summary;
}

function summarizeJsonl(content) {
  const lines = content.split("\n").filter(Boolean);
  return {
    kind: "jsonl",
    eventCount: lines.length,
    firstEvents: lines.slice(0, 5).map((l) => {
      try {
        return JSON.parse(l);
      } catch {
        return l.slice(0, 200);
      }
    }),
    lastEvent: (() => {
      const last = lines[lines.length - 1];
      try {
        return JSON.parse(last);
      } catch {
        return last ? last.slice(0, 200) : null;
      }
    })(),
  };
}

function buildEntry(absPath, relPath) {
  const st = statSync(absPath);
  const content = readFileSync(absPath, "utf8");
  const entry = {
    path: relPath,
    sizeBytes: st.size,
    mtime: st.mtime.toISOString(),
  };

  if (relPath.endsWith(".md")) {
    entry.kind = "markdown";
    entry.content = content;
  } else if (relPath.endsWith(".jsonl")) {
    Object.assign(entry, summarizeJsonl(content));
  } else if (relPath.endsWith(".json")) {
    const summary = summarizeJson(content, relPath);
    Object.assign(entry, summary);
    entry.headExcerpt = content.slice(0, HEAD_EXCERPT_BYTES);
  } else {
    entry.kind = "raw";
    entry.headExcerpt = content.slice(0, HEAD_EXCERPT_BYTES);
  }

  return entry;
}

function writeEntryFile(namespace, key, entry) {
  const ns = namespace.replace(/\./g, "/");
  const dir = join(OUT_DIR, ns);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  const fname = join(dir, safeKey(key) + ".json");
  writeFileSync(fname, JSON.stringify(entry, null, 2));
  return fname;
}

function tryCliStore(key, value, namespace, tags) {
  if (!TRY_CLI) return { skipped: true };
  const args = [
    "@claude-flow/cli@latest",
    "memory",
    "store",
    "-k",
    key,
    "--value",
    value,
    "-n",
    namespace,
    "--upsert",
  ];
  if (tags && tags.length) args.push("--tags", tags.join(","));
  try {
    execFileSync("npx", args, {
      stdio: ["ignore", "pipe", "pipe"],
      encoding: "utf8",
    });
    return { ok: true };
  } catch (e) {
    return {
      ok: false,
      error: (e.stderr || e.message || "").toString().slice(0, 200),
    };
  }
}

function main() {
  const candidateDirs = [".omc", ".understand-anything"];
  const targets = [];
  for (const d of candidateDirs) targets.push(...walk(join(REPO_ROOT, d)));

  if (!existsSync(OUT_DIR)) mkdirSync(OUT_DIR, { recursive: true });

  const manifest = {
    funneledAt: new Date().toISOString(),
    sourceDirs: candidateDirs,
    cliAttempted: TRY_CLI,
    entries: [],
  };

  let stored = 0;
  let skipped = 0;
  let failed = 0;
  let cliOk = 0;
  let cliFail = 0;
  const failures = [];

  for (const abs of targets) {
    const rel = relative(REPO_ROOT, abs);
    if (shouldSkip(rel)) {
      skipped++;
      continue;
    }
    let entry;
    try {
      entry = buildEntry(abs, rel);
    } catch (e) {
      failed++;
      failures.push({ rel, error: String(e).slice(0, 200) });
      continue;
    }

    const ns = deriveNamespace(rel);
    const tagsList = [
      ns.split(".")[0],
      ns.split(".")[1] || "general",
      entry.kind || "file",
    ];
    const slice = rel.match(/slice-(\d+[a-z]?)/);
    if (slice) tagsList.push(`slice-${slice[1]}`);

    const entryFile = writeEntryFile(ns, rel, entry);
    stored++;

    const cliResult = tryCliStore(rel, JSON.stringify(entry).slice(0, 24_000), ns, tagsList);
    if (cliResult.ok) cliOk++;
    else if (cliResult.error) cliFail++;

    manifest.entries.push({
      key: rel,
      namespace: ns,
      tags: tagsList,
      kind: entry.kind || "file",
      sizeBytes: entry.sizeBytes,
      mtime: entry.mtime,
      entryFile: relative(REPO_ROOT, entryFile),
    });

    if (stored % 25 === 0) process.stdout.write(`  stored ${stored}…\n`);
  }

  manifest.totalEntries = stored;
  manifest.skipped = skipped;
  manifest.failed = failed;
  manifest.cliOk = cliOk;
  manifest.cliFail = cliFail;
  writeFileSync(MANIFEST_PATH, JSON.stringify(manifest, null, 2));

  console.log("\n=== FUNNEL SUMMARY ===");
  console.log(`  manifest: ${relative(REPO_ROOT, MANIFEST_PATH)}`);
  console.log(`  out dir:  ${relative(REPO_ROOT, OUT_DIR)}`);
  console.log(`  stored:   ${stored}`);
  console.log(`  skipped:  ${skipped}`);
  console.log(`  failed:   ${failed}`);
  if (TRY_CLI) console.log(`  cli-ok:   ${cliOk}  cli-fail: ${cliFail}`);
  if (failures.length) {
    console.log("\n=== FAILURES (first 5) ===");
    for (const f of failures.slice(0, 5)) console.log(`  ${f.rel}: ${f.error}`);
  }
}

main();
