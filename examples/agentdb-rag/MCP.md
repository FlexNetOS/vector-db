# Wire AgentDB MCP into Claude Code

This makes the AgentDB tools (vector search, store-pattern, query, reflexion,
skills, causal) available to Claude Code as first-class MCP tools, so any agent
in any project can read/write the same vector store.

## One-time setup

```bash
# 1. Register the AgentDB MCP server. The server uses stdio transport.
claude mcp add agentdb -- npx -y agentdb@latest mcp start

# 2. Verify it registered.
claude mcp list | grep agentdb
```

You should see something like:
```
agentdb  npx -y agentdb@latest mcp start  (stdio)
```

## Confirm in-session

After restarting Claude Code (or `/mcp reconnect`), the server surfaces
**32 tools** organized in five groups:

| Group | Count | Examples |
|---|---|---|
| Core | 5 | `vector_search`, `store_pattern`, `query`, `db_stats` |
| Frontier | 9 | reflexion (store/retrieve/critique), skill (create/search/consolidate) |
| Learning | 10 | 9 RL algorithms (Q-Learning, SARSA, Actor-Critic, …) |
| AgentDB | 5 | causal edges, recall-with-certificate |
| Batch ops | 3 | bulk insert / search / export |

Inspect the canonical names in-session via Claude Code's `/mcp` slash command,
or list registered servers from the shell with:
```bash
claude mcp list
claude mcp get agentdb
```

Verified smoke-test response (server replies to `initialize`):
```json
{"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},
 "serverInfo":{"name":"agentdb","version":"1.3.0"}},"jsonrpc":"2.0","id":1}
```

## Point AgentDB at this example's database

By default the MCP server uses `./agentdb.db` (cwd) or `AGENTDB_PATH` if set.
To make Claude Code read this example's seeded DB:

```bash
# In the shell you launch claude from:
export AGENTDB_PATH="$(pwd)/examples/agentdb-rag/vectors.db"
claude
```

Or, if you prefer a project-scoped MCP definition, edit
`~/.claude/mcp.json` (or `<project>/.mcp.json`) and add an `env`:

```json
{
  "mcpServers": {
    "agentdb-ragdemo": {
      "command": "npx",
      "args": ["-y", "agentdb@latest", "mcp", "start"],
      "env": {
        "AGENTDB_PATH": "/abs/path/to/ruvector/examples/agentdb-rag/vectors.db"
      }
    }
  }
}
```

## Smoke test the MCP server standalone

The MCP server speaks JSON-RPC over stdio. You can poke it directly:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
  | npx --yes agentdb@latest mcp start
```

A valid response includes `"protocolVersion"` and `"capabilities":{"tools":{}}`.

## Removal

```bash
claude mcp remove agentdb
```

## Notes & gotchas

- The MCP server's working directory is whatever `claude` was launched from.
  Always pin `AGENTDB_PATH` to an absolute path in the MCP env block to avoid
  surprises.
- AgentDB v3 auto-detects the **ruvector** backend if it's installed (it is,
  in this repo). Otherwise it falls back to hnswlib or sql.js — slower but
  functionally identical for the tool surface.
- If `/mcp` in Claude Code shows the server as failed (or `claude mcp get
  agentdb` reports an unhealthy state), the server probably crashed on
  startup. Run `npx --yes agentdb@latest doctor` to diagnose.
