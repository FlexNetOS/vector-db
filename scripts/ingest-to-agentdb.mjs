#!/usr/bin/env node
// Ingests the file-based memory layer at .claude/memory/funneled/ into the
// AgentDB v3 vector store at .agentdb/reasoningbank.db. Each manifest entry
// becomes an AgentDB Episode (auto-embedded via the controller's internal
// embedder) with the funnelled JSON content as task + input + critique.
//
// Run from repo root: node scripts/ingest-to-agentdb.mjs

import { readFileSync, existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { AgentDB } from "agentdb";

const REPO_ROOT = process.cwd();
const MANIFEST = join(REPO_ROOT, ".claude", "memory", "funneled", "manifest.json");
const DB_PATH = join(REPO_ROOT, ".agentdb", "reasoningbank.db");

function entryToTask(entry, m) {
  const bits = [`[${m.namespace}] ${m.key}`, `kind=${entry.kind || "file"}`];
  if (entry.project?.name) bits.push(`project=${entry.project.name}`);
  if (entry.fileCount != null) bits.push(`files=${entry.fileCount}`);
  if (entry.nodeCount != null) bits.push(`nodes=${entry.nodeCount}`);
  if (entry.edgeCount != null) bits.push(`edges=${entry.edgeCount}`);
  if (entry.nodeTypes)
    bits.push(`nodeTypes=${Object.keys(entry.nodeTypes).slice(0, 8).join(",")}`);
  if (entry.topLevelKeys?.length)
    bits.push(`topKeys=${entry.topLevelKeys.slice(0, 10).join(",")}`);
  if (Array.isArray(m.tags)) bits.push(`tags=${m.tags.join(",")}`);
  return bits.join(" | ");
}

function entryToInput(entry) {
  if (entry.content) return entry.content.slice(0, 4000);
  if (entry.headExcerpt) return entry.headExcerpt.slice(0, 4000);
  if (entry.firstEvents) return JSON.stringify(entry.firstEvents).slice(0, 4000);
  if (entry.sampleFiles) return entry.sampleFiles.slice(0, 30).join("\n");
  return JSON.stringify(entry).slice(0, 4000);
}

function entryToCritique(entry, m) {
  const meta = {
    path: m.key,
    namespace: m.namespace,
    tags: m.tags,
    kind: m.kind,
    sizeBytes: m.sizeBytes,
    mtime: m.mtime,
    entryFile: m.entryFile,
  };
  return JSON.stringify(meta);
}

async function main() {
  console.log("Initializing AgentDB…");
  if (!existsSync(".agentdb")) mkdirSync(".agentdb", { recursive: true });
  const db = new AgentDB({
    dbPath: DB_PATH,
    dimension: 384,
    namespace: "ruvector.funneled",
  });
  await db.initialize();
  const memory = db.getController("memory");

  console.log("Reading manifest…");
  const manifest = JSON.parse(readFileSync(MANIFEST, "utf8"));
  console.log(`  ${manifest.totalEntries} entries to ingest`);

  let inserted = 0;
  let failed = 0;
  const failures = [];

  for (const m of manifest.entries) {
    try {
      const entryPath = join(REPO_ROOT, m.entryFile);
      const entry = JSON.parse(readFileSync(entryPath, "utf8"));

      const tsSec = Math.floor((Date.parse(m.mtime) || Date.now()) / 1000);
      await memory.storeEpisode({
        sessionId: `funnel.${m.namespace}`,
        task: entryToTask(entry, m),
        reward: 1.0,
        success: true,
        input: entryToInput(entry),
        output: `Funnelled from ${m.entryFile}`,
        critique: entryToCritique(entry, m),
        ts: tsSec,
      });
      inserted++;
      if (inserted % 5 === 0)
        process.stdout.write(`  inserted ${inserted}/${manifest.totalEntries}…\n`);
    } catch (e) {
      failed++;
      failures.push({ key: m.key, error: String(e).slice(0, 300) });
    }
  }

  // Rebuild the in-memory vector index from the freshly persisted embeddings
  // so semantic search works in this process and after restarts. The RVF
  // backend doesn't auto-load embeddings from SQLite on initialize.
  console.log("Rebuilding vector index from persisted embeddings…");
  const built = await memory.rebuildIndex();
  console.log(`  rebuildIndex: ${built} vectors indexed`);

  await db.close();

  console.log("\n=== AGENTDB INGEST SUMMARY ===");
  console.log(`  db:        ${DB_PATH}`);
  console.log(`  inserted:  ${inserted}`);
  console.log(`  failed:    ${failed}`);
  if (failures.length) {
    console.log("\n=== FAILURES (first 5) ===");
    for (const f of failures.slice(0, 5)) console.log(`  ${f.key}: ${f.error}`);
  }
}

main().catch((e) => {
  console.error("FATAL:", e);
  process.exit(1);
});
