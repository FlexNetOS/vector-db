# Edge-Net Dashboard

A real-time, browser-based control surface for the Edge-Net Time Crystal Network.
Built with React 19, Vite 7, HeroUI 2.8, Tailwind 3.4, Zustand, TanStack Query,
Recharts, Lucide icons, and Framer Motion. Tests run on Vitest + Playwright.

This package lives at `examples/edge-net/dashboard/` inside the `ruvector` repo
and is published privately as `@ruvector/edge-net-dashboard`.

## Quick start

```bash
# Linux / x86_64 (recommended dev environment)
cd examples/edge-net/dashboard
npm install --no-audit
npm run dev       # vite dev server with HMR
npm run build     # tsc -b && vite build  → dist/
npm run lint      # eslint .
npm run test      # vitest run
```

All other workflows are exposed via npm scripts in `package.json` (`test:watch`,
`test:coverage`, `preview`, plus the Docker pipeline below).

## Configuration

User-facing toggles are Vite env vars defined in `.env.example`. Copy that file
to `.env.local` and override values as needed.

### Feature flags

Surfaces that are not yet production-ready are gated by env flags and hidden
by default. The Activity and Settings panels were previously stubs and have
now been wired to their real implementations
(`src/components/dashboard/{ActivityPanel,SettingsPanel}.tsx`); only the PiKey
restore flow remains flag-gated because it depends on a WASM binding that has
not landed upstream yet.

| Variable                    | Default | Surface                                                    |
| --------------------------- | ------- | ---------------------------------------------------------- |
| `VITE_ENABLE_PIKEY_RESTORE` | `false` | "Restore from encrypted backup" path in the Identity panel |

Anything other than the literal string `true` resolves to `false`. The flags
are read in `src/utils/featureFlags.ts`; see that file for the canonical list.

## Docker

```bash
npm run docker:build  # build the production image
npm run docker:run    # serve dist/ via nginx on :3000
npm run docker:dev    # vite dev server inside the dev profile
```

The compose profile `dev` is the recommended workaround for Apple Silicon
hosts — see below.

## Project layout

```
src/
  assets/        # static images / svg
  components/    # feature-grouped React components
    brain/ cdn/ common/ dashboard/ docs/
    economics/ identity/ mcp/ network/ rewards/ wasm/
  hooks/         # custom hooks (network store wiring, etc.)
  services/      # WASM + service-layer integrations (edgeNet, …)
  stores/        # zustand stores (network, identity, …)
  tests/         # vitest unit / component tests
  types/         # shared TS types
  utils/         # featureFlags + helpers
  App.tsx        # top-level layout + routing
  main.tsx       # ReactDOM entrypoint
  index.css      # Tailwind layer + crystal/quantum theme
e2e/             # Playwright end-to-end specs
```

## Known Issue: Apple Silicon (darwin-arm64)

`npm install` currently fails on Apple Silicon (M1/M2/M3/M4) because a
transitive dependency on `wrtc` does not ship a `darwin-arm64` prebuilt
binary. This is tracked upstream as
[ruvnet/RuVector#276](https://github.com/ruvnet/RuVector/issues/276).

The failure mode looks like:

```
npm ERR! prebuild-install ... No prebuilt binaries found
       (target=… runtime=node arch=arm64 platform=darwin)
```

Workarounds (pick the one that fits your workflow):

1. **Develop on Linux / x86_64** — the only first-class supported environment
   today. CI runs on Linux x86_64 and is the source of truth.
2. **Docker dev container** — run the dashboard inside the supplied dev
   profile:

   ```bash
   npm run docker:dev   # docker-compose --profile dev up dashboard-dev
   ```

   The dev image is `linux/amd64`; Docker Desktop on Apple Silicon will run it
   under emulation, which avoids the `wrtc` native-module gap entirely.
3. **Wait for upstream** — once `wrtc` (or its replacement) publishes a
   `darwin-arm64` prebuilt, this section can be removed. Subscribe to
   [ruvnet/RuVector#276](https://github.com/ruvnet/RuVector/issues/276) for
   updates.

Do **not** attempt to work around the failure by switching package managers
or passing `--legacy-peer-deps` — those mask the underlying native-module
issue and tend to produce a half-installed tree that fails at runtime.
