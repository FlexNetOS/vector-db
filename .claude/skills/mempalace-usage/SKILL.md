---
name: mempalace-usage
description: Use the MemPalace MCP server for persistent cross-session memory of decisions, conversations, and learnings. The palace lives at ~/.mempalace/ (per-user, not per-repo) and exposes 9 MCP tools — search, add_drawer, list_wings, list_rooms, get_taxonomy, check_duplicate, delete_drawer, reconnect, status. Reach for this skill BEFORE asking the user to re-explain something they already told you, AFTER making a non-obvious decision worth remembering, or when you need verbatim recall of past conversations.
triggers:
  - mempalace
  - mem palace
  - memory palace
  - persistent memory
  - long-term memory
  - cross-session memory
  - remember this
  - what did we decide
  - check memory
  - search memory
  - "/mempalace"
  - .mempalace/
---

# MemPalace Usage — RuVector

## What it is

MemPalace is a local-first AI memory store. It saves verbatim text,
indexes it with sentence-transformer embeddings stored in ChromaDB, and
exposes search/store/list operations over MCP. **Nothing leaves the
local box** — no API keys, no cloud calls. The palace itself lives at
`~/.mempalace/` (per-user, cross-project).

The data model: **wings** (people / projects) → **rooms** (topics,
drawn from folder structure) → **drawers** (verbatim content). Searches
can be scoped to a wing or room, or run flat across the whole palace.

## When to reach for MemPalace (vs. GitNexus / Understand-Anything)

| Question type | Tool | Why |
|---|---|---|
| "What did we decide about X last week?" | **MemPalace** `search` | Verbatim conversation history with semantic recall |
| "What's the call graph for `fn foo`?" | GitNexus `impact` | Structural code-graph traversal |
| "What does this crate do?" | Understand-Anything `/understand-onboard` | LLM-assembled comprehension graph |
| "Why was this pattern chosen?" | **MemPalace** `search` first, then GitNexus `context` | Memory captures intent, GitNexus captures structure |
| "Has this been discussed before?" | **MemPalace** `check_duplicate` | Pre-commit dedup against existing memories |

The three tools are **complementary**: GitNexus = structure, Understand-Anything = comprehension, MemPalace = history/intent. An ideal pre-edit pass queries all three.

## Install

```bash
scripts/install-mempalace.sh
```

This is idempotent. It:
1. Verifies Python 3.9+ is on PATH.
2. `pipx install mempalace` (preferred) or `pip install --user mempalace`.
   To close the dependency-confusion window, the install subprocess in
   both paths runs with:
   - `--index-url https://pypi.org/simple/` (pin the primary index)
   - `PIP_EXTRA_INDEX_URL=''` (strip any inherited extra-index env var)
   - `PIP_CONFIG_FILE=/dev/null` + `--no-config` (ignore `pip.conf` /
     `pip.ini` `extra-index-url` entries for this install)
   `--index-url` *alone* would not be enough — it only overrides the
   primary index and pip will still consult any configured extra index.
   Override with `MEMPALACE_INDEX_URL=...` only if you have a verified
   internal proxy.
3. Surfaces the Claude Code plugin install command if `claude` is on PATH.

**The bootstrap does NOT call `mempalace init`.** Per upstream issue
[MemPalace/mempalace#185](https://github.com/MemPalace/mempalace/issues/185),
`mempalace init <dir>` writes `<dir>/entities.json` and
`<dir>/mempalace.yaml` into the target directory — running it against
the repo root would dirty every contributor's checkout with two
untracked generated files. Instead, run init manually against a
directory you're willing to dirty (a per-user staging dir, or a
project directory outside the repo):

```bash
mkdir -p ~/.mempalace/projects/ruvector
mempalace init ~/.mempalace/projects/ruvector   # safe to dirty; per-user
```

The palace database itself lives at `~/.mempalace/` regardless of where
you ran init — the init target only receives the two metadata files.

Mining (importing source/docs into the palace) is **OFF by default** —
it can take minutes on ruvector's ~150-crate workspace. Opt in only
after you've run `mempalace init` somewhere:

```bash
mempalace mine "$(pwd)"                                  # code + docs
mempalace mine "$(pwd)" --mode convos                    # conversation exports
mempalace mine ~/chats/ --mode convos --extract general  # auto-classify
```

`mempalace mine <dir>` does NOT write `entities.json`/`mempalace.yaml`
into `<dir>` — it only ingests file content into `~/.mempalace/`. Safe
to run against any directory.

## MCP tool list (Claude Code, codex, etc.)

```
mempalace_status            — palace health, total drawers, wing/room breakdown
mempalace_list_wings        — list all wings + drawer counts
mempalace_list_rooms        — rooms within a wing
mempalace_get_taxonomy      — full wing → room → count tree
mempalace_search            — semantic search; optional wing/room filter
mempalace_check_duplicate   — check before filing a drawer (dedup pass)
mempalace_add_drawer        — file verbatim content into a wing/room
mempalace_delete_drawer     — remove a drawer by ID
mempalace_reconnect         — invalidate cache after external writes
```

(Some marketing pages claim 19 or 29 tools — the actual MCP server source is
9. Verified against `mempalace/mcp_server.py` on the develop branch.)

### Recommended pre-edit pattern

```
1. mempalace_search    "<task summary>"          # has anyone done this before?
2. gitnexus.context    <symbol>                  # structural neighborhood
3. /understand-explain <symbol>                  # comprehension layer
4. (do the work)
5. mempalace_add_drawer  wing=ruvector room=<topic> content=<decision + rationale>
```

## Storage location & scope

| Path | Contents | Status |
|---|---|---|
| `~/.mempalace/` | Palace database (ChromaDB + SQLite) | **Per-user.** Never committed; cross-project. |
| `~/.mempalace/config.yaml` | User config | Per-user. |
| In-repo state | _none_ | MemPalace does **not** write into the repo. |

Multiple repos share one palace by design — that's how agents remember
the cross-cutting context that spans repos (e.g., a decision made while
working on `ruvector` also matters when you switch to `weftos` next
week).

## When NOT to use MemPalace

- For **code-structure queries** ("where is X called from?", "rename Foo")
  — use **GitNexus**. MemPalace stores conversational memory, not AST.
- For **fresh-codebase comprehension** ("what does this crate do?") —
  use **Understand-Anything** `/understand-onboard <crate>`. MemPalace
  is empty on first run; it's only useful once the palace has history.
- For **secrets, credentials, PII**. MemPalace stores verbatim — never
  pass raw API keys, customer data, or anything that would harm someone
  if leaked locally.

## Repo-specific notes

- **Workspace size**: ruvector has ~150 crates; a full `mempalace mine`
  pass walks every source/docs file. Allow ~5–15 minutes the first time.
  Subsequent runs are incremental.
- **`crates/ruvector-postgres`** and other workspace-excluded crates are
  still walked by `mempalace mine` (it follows the filesystem, not the
  Cargo manifest). Use `--exclude` flags or a `.mempalace_ignore` file
  if you want to scope down.
- **Brain integration**: this repo's `mcp-brain-server` is a separate
  shared-memory layer (REST API at `~/.mcp-brain/`). MemPalace is local
  per-user; brain-server is multi-user shared. Use both — they don't
  conflict.

## Smoke test

```bash
# After bootstrap (CLI installed, no palace yet):
mempalace --version       # confirms CLI is on PATH

# After you have run `mempalace init <some-dir>` once anywhere:
mempalace status          # confirms palace exists + connected
mempalace list-wings      # shows wings (empty until you mine)
mempalace search "test"   # semantic search smoke test
```

If `status` / `list-wings` / `search` error with `palace not
initialized`, you haven't run `mempalace init <dir>` yet — run it
against a directory you're willing to dirty (see Install above). The
bootstrap script intentionally skips this step.

## License & provenance

MemPalace ships under MIT. Upstream:
- GitHub: https://github.com/MemPalace/mempalace
- PyPI:   https://pypi.org/project/mempalace/
- Docs:   https://mempalaceofficial.com/

**Do NOT install from `mempalace.tech`** — that's a known impostor
domain that distributes malware. The bootstrap script's install
subprocess pins `--index-url https://pypi.org/simple/`, clears
`PIP_EXTRA_INDEX_URL`, and ignores `pip.conf` / `pip.ini` so that no
ambient pip configuration can redirect resolution to a malicious
mirror.
