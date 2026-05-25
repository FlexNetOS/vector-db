# RvLite Dashboard

Interactive React dashboard for exploring the **RvLite** WebAssembly vector
database. Demonstrates SQL, SPARQL, Cypher, vector search, graph neural
networks and self-learning capabilities in a single Vite + React 19 SPA.

This dashboard is a **demonstration/playground** for the parent
[`crates/rvlite`](../../README.md) crate. It is not a production application.

---

## Overview

| Surface | Purpose |
|---------|---------|
| **Vectors tab** | Insert, search, inspect and bulk-import vectors (CSV/JSON). |
| **SQL tab** | Execute `CREATE TABLE` / `SELECT` queries; automatically renders the resulting schema in a browsable tree. |
| **SPARQL tab** | Run SPARQL queries over the RDF graph store. |
| **Cypher tab** | Run Cypher queries over the property-graph store. |
| **Filter Builder** | Visual constructor for metadata filters used by vector search (no JSON typing required). |
| **Simulation / Supply Chain** | Graph + neural visualization examples driven by `useLearning` and `NeuralEngine`. |

## Quick Start

The dashboard depends on the parent `rvlite` crate compiled to WebAssembly. See
[Parent Crate Integration](#parent-crate-integration) for the prerequisite
WASM build.

```bash
# 1. Install dependencies
npm install

# 2. Build the parent WASM crate (see "Parent Crate Integration" below)
#    so that public/pkg/rvlite_bg.wasm + public/pkg/rvlite.js exist

# 3. Run the dev server
npm run dev            # http://localhost:5173

# 4. Production build
npm run build          # outputs dist/

# 5. Lint
npm run lint
```

## Features

### Bulk Vector Import (CSV / JSON)

The Quick Actions panel exposes a **Bulk Import Vectors** modal that accepts
either CSV or JSON. Sample fixtures live under `docs/`:

- `docs/sample-bulk-import.csv`
- `docs/sample-bulk-import.json`

**CSV format** (header row required, `id` + `embedding` mandatory):

```csv
id,embedding,metadata
vec1,"[1.0,2.0,3.0]","{""category"":""test""}"
vec2,"[4.0,5.0,6.0]","{}"
```

**JSON format** (array of records):

```json
[
  { "id": "vec1", "embedding": [1.0, 2.0, 3.0], "metadata": { "category": "test" } },
  { "id": "vec2", "embedding": [4.0, 5.0, 6.0] }
]
```

Workflow: upload file (or paste data) -> Preview (first 5 vectors) -> Import.
Progress and error counts are reported in real time; vectors are inserted via
the existing `insertVectorWithId` hook so they behave identically to
single-vector inserts.

### Filter Builder

The vector search input supports a visual filter builder
(`src/FilterBuilder.tsx`) that compiles to the JSON filter format the parent
crate's `searchVectorsWithFilter` expects.

**Supported operators**: `eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `contains`,
`exists`.

Multiple conditions combine with **AND**. Multiple conditions on the same
field merge into one object (range queries):

| Builder rows | Generated JSON |
|--------------|----------------|
| `category = ML`, `tags contains sample` | `{ "category": "ML", "tags": { "$contains": "sample" } }` |
| `price > 50`, `price < 100` | `{ "price": { "$gt": 50, "$lt": 100 } }` |
| `description exists true` | `{ "description": { "$exists": true } }` |

Toggle **Show JSON** in the builder header to see the generated filter live.

### SQL Schema Browser

When you execute a `CREATE TABLE` statement in the SQL tab, the dashboard
parses it with a lightweight regex
(`/CREATE\s+TABLE\s+(\w+)\s*\(([^)]+)\)/i`), extracts column definitions
(including `VECTOR(n)` dimensions), and renders the schema in an expandable
**Schema Browser** card next to the SQL result.

For each tracked table:

- **Query** button -> auto-fills `SELECT * FROM <table>`
- **Drop** button -> runs `DROP TABLE <table>` with confirmation and removes
  it from the browser state.

Column-type colour coding: `VECTOR(n)` purple, `TEXT` blue, `INTEGER`/`REAL`
green, others grey.

### Vector Inspector

Clicking a vector's ID (or the eye icon in the Actions column) opens a modal
showing:

- Vector ID (copyable via `Snippet`)
- Dimensions (chip with `<n>D` label)
- Embedding values (first 20 + count if `length > 20`), with **Copy Array**
- Metadata as formatted JSON, with **Copy JSON**

The inspector uses `getVector(id)` from the `useRvLite` hook and falls back to
an empty state when the WASM store does not return data.

## Architecture

```
src/
  App.tsx                  Top-level SPA (large; do not refactor without coordination)
  FilterBuilder.tsx        Visual filter builder component
  main.tsx, App.css, ...   Vite / React entry
  hero.ts                  HeroUI theme tokens
  components/
    GraphVisualization.tsx       D3-based graph rendering
    SimulationEngine.tsx         Neural simulation tab
    SupplyChainSimulation.tsx    Supply-chain demo
  hooks/
    useRvLite.ts           WASM bindings facade: insertVector, searchVectors,
                           searchVectorsWithFilter, getVector, executeSql,
                           executeSparql, executeCypher, ...
    useLearning.ts         ReasoningBank / SONA helpers
  lib/
    NeuralEngine.ts        Local neural utilities

public/
  pkg/                     wasm-pack output (NOT checked in; see below)
  vite.svg

scripts/
  test-all.mjs             Full WASM smoke suite (Vectors, SQL, SPARQL, Cypher)
  test-sql.mjs             SQL-only subset
  e2e-wasm-test.mjs        End-to-end suite
  debug-keys.mjs, debug-sparql.mjs   Ad-hoc debugging helpers

docs/
  INTEGRATION_GUIDE.md     Bulk-import implementation notes
  VISUAL_INTEGRATION_MAP.md
  QUICK_REFERENCE.md
  sample-bulk-import.csv
  sample-bulk-import.json
  bulk-import-code.tsx     Reference snippets (not compiled by Vite)

filter-helpers.ts          Reference helpers (not compiled by Vite)
```

### Stack

- React 19 + Vite 7 (ESM, type=module)
- TypeScript ~5.9.3 with `tsc -b` project references
  (`tsconfig.app.json`, `tsconfig.node.json`)
- HeroUI 2.8 + Tailwind CSS 4 (with `@tailwindcss/postcss`)
- Recharts 3, Lucide React, framer-motion
- ESLint 9 + typescript-eslint 8 (flat config in `eslint.config.js`)

### State and data flow

The single `App` component holds dashboard state and delegates WASM calls to
`useRvLite`. Filter Builder, Bulk Import, SQL Schema Browser, and Vector
Inspector are integrated **inline in `App.tsx`** and share state via React
hooks (`useState`, `useCallback`, `useEffect`, `useDisclosure`). No external
state management library is used.

Reference helper modules — `filter-helpers.ts`, `docs/bulk-import-code.tsx` —
are kept as documentation of the original integration snippets but are **not**
referenced by the compiled bundle.

## Parent Crate Integration

`scripts/test-*.mjs` and the dashboard runtime expect the parent `rvlite`
crate's wasm-pack output at:

```
public/pkg/rvlite_bg.wasm
public/pkg/rvlite.js
```

These artefacts are produced by building the parent crate (this is **not**
performed automatically by `npm install` or `npm run build`):

```bash
# From the repo root
cd crates/rvlite
wasm-pack build --target web --release

# Copy / link the pkg/ output to the dashboard's public/pkg
cp -r pkg/. examples/dashboard/public/pkg/
```

The CI job `rvlite-dashboard` runs `npm ci` + `npm run lint` + `npm run build`
without the parent crate built, so the `test` step is currently
`continue-on-error: true` until WASM packaging is wired into CI. See
`.github/workflows/` and ADR-118 (or successor) for the integration plan.

## Development

```bash
npm run dev          # Vite dev server with HMR
npm run build        # tsc -b && vite build  -> dist/
npm run lint         # eslint .
npm run preview      # vite preview (serves dist/)
```

### Dev-server smoke check

`npm run dev` listens on `http://localhost:5173` by default. The dashboard
will render the shell even without `public/pkg/` present, but vector / SQL /
SPARQL / Cypher operations will fail at runtime until the WASM artefacts
are in place.

### File-organisation rules

- `App.tsx` is intentionally large (~4k lines). Do **not** refactor it
  without coordinating with the rvlite crate owners; many integration
  points use line-anchored snippets in CI documentation.
- New features should land as separate components under `src/components/`
  and be wired into `App.tsx` with minimal diff.

## Testing

```bash
npm test             # node scripts/test-all.mjs
npm run test:sql     # node scripts/test-sql.mjs
npm run test:e2e     # node scripts/e2e-wasm-test.mjs
```

All three test scripts dynamically `import('../public/pkg/rvlite.js')` and
require the parent crate's WASM build (see
[Parent Crate Integration](#parent-crate-integration)). Without that build
the test scripts will exit early with a "module not found" error.

The CI `rvlite-dashboard` job currently marks the `test` step as
`continue-on-error: true` because WASM packaging is not yet wired into the
workflow; lint + build are hard gates.

### What the tests cover

- **`test-all.mjs`** — vector insert / search / delete; SQL `CREATE TABLE`,
  `SELECT`, `DROP TABLE`; SPARQL inserts / queries; Cypher graph creation
  and traversal. Treated as the canonical smoke suite.
- **`test-sql.mjs`** — SQL CRUD subset (faster local iteration).
- **`e2e-wasm-test.mjs`** — broader E2E covering all four query languages
  plus filter combinations.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Blank screen, console error `Failed to fetch /pkg/rvlite_bg.wasm` | `public/pkg/` missing | Build parent crate (`wasm-pack build --target web`) and copy `pkg/` into `public/pkg/`. |
| `npm test` exits with "Cannot find module './../public/pkg/rvlite.js'" | Same as above. | Same as above. |
| Bulk import preview parses 0 rows | CSV missing `id` or `embedding` columns; embedding column not JSON-encoded. | Use the format under [Bulk Vector Import](#bulk-vector-import-csv--json). |
| Filter Builder JSON is empty `{}` | All conditions have blank `field`. | Fill in the field name; blank-field conditions are skipped intentionally. |
| ESLint flags unused imports after refactor | Some HeroUI / Lucide imports are kept for the inline integration. | Either use them or remove cautiously — they are intentionally referenced by inline modal code. |

## License

Inherits the parent repository license. See the repo-root `LICENSE` file.
