# TODO — Cognitum Seed host integration follow-ups

Tracks work spawned by the `feat/cognitum-seed-host-setup` branch / PR
but intentionally kept out of it to keep that PR focused on host plumbing.

The PR itself delivered the persistent fix (udisks2 override + NM keyfile
+ udev rule + idempotent installer + 15-probe doctor). Verified on a
paired Seed (firmware 0.21.11, device_id `0e34a5e5-…-e437e22f326a`):
all four transports reachable, token-authed `/api/v1/identity` returns
valid JSON.

## Verify replug / reboot / Seed-swap behavior periodically

The NM keyfile, udev rule, and udisks2 override are designed to survive
unplug, replug, reboot, and Seed swaps. Re-run the doctor after any of:

- Linux distro upgrade (NetworkManager or udisks2 version bump)
- NetworkManager config reset / reinstall
- New Seed device (different MAC, different `cognitum-XXXX.local`
  suffix; `match.driver=cdc_ncm;cdc_ether;rndis_host` in the keyfile
  is what makes this work without per-device config)
- Manual edits to `/etc/NetworkManager/system-connections/cognitum-seed.nmconnection`,
  `/etc/udev/rules.d/70-cognitum-seed.rules`, or
  `/etc/udisks2/mount_options.conf.d/cognitum-fat.conf`

```bash
crates/ruvector-cognitum-host/deploy/seed-doctor.sh
```

Expected: `15 PASS, 0 FAIL` plus `Seed is REACHABLE via at least one transport.`
If any row FAILs, re-run `crates/ruvector-cognitum-host/deploy/install.sh` —
idempotent, only writes + reloads if something changed.

## File issue against cognitum.one MCP-SSE endpoint

`https://cognitum.one/mcpSse` violates the MCP 2024-11-05 SSE transport
spec on three counts (confirmed by reading the raw `curl -N` stream
during the host-setup work, 2026-05-24):

1. Missing `endpoint` event. Spec: "When a client connects, the server
   MUST send an `endpoint` event containing a URI for the client to
   use for sending messages." Cognitum's server never sends one — its
   first SSE frame is a bare `data: {...notifications/initialized...}`.
2. No `event:` field on any SSE frame. All frames are bare `data:`
   lines, which default to event type `message` (not `endpoint`,
   and not the spec's named `message` for subsequent JSON-RPC).
3. Pushes `notifications/initialized` with `serverInfo` inside the
   `params` before the client has sent `initialize`. The `serverInfo`
   field belongs in the `InitializeResult` response to a
   client-sent `initialize` request, not in an unsolicited
   notification. Lifecycle violation on top of transport violation.

Effect: Claude Code's `SSEClientTransport` registers an
`addEventListener('endpoint', …)` and times out / errors when the
event never arrives. `claude mcp list` flags the `cognitum` cloud
entry as `✗ Failed to connect`.

Suggested fix to the cognitum team (recommend either):

- **Quick**: emit `event: endpoint\ndata: https://cognitum.one/mcpPost\n\n`
  as the literal first SSE frame, then continue with `event: message`
  framed JSON-RPC. Spec-compliant under 2024-11-05.
- **Better**: migrate the endpoint to Streamable HTTP (MCP 2025-03-26)
  at `/mcp`. Today every `cognitum.one/mcp*` and `/v1/mcp` path
  returns nginx 405 on POST — no working Streamable HTTP endpoint
  exists. The Seed appliance itself already implements this transport
  correctly (`http://169.254.42.1/mcp` works fine), so the same code
  could be brought to the cloud front-end.

Spec references:

- https://modelcontextprotocol.io/specification/2024-11-05/basic/transports
- https://modelcontextprotocol.io/specification/2025-03-26/basic/transports

`crates/ruvector-cognitum-host/.mcp.json` (project root) keeps the
broken entry registered with `autoStart: false` so it's just cosmetic
red until the server is fixed; it'll start working automatically the
moment the upstream fix ships.

## Update the COGNITUM USB volume's README + ship a Linux fix

The Seed's mass-storage volume (`/run/media/$USER/COGNITUM/`) ships a
`README.txt` that tells Linux users to "Run `./launch.sh`". On any
distro using `udisks2` defaults (Ubuntu, Fedora, Debian, etc.) the
vfat mount strips `+x` from `.sh` files via the `showexec` option,
so double-clicking `launch.sh` triggers GNOME's "program is not
marked as executable" dialog. Same problem affects `install-trust.sh`
in the `trust/` dir.

Two vendor-side fixes worth proposing upstream
(`github.com/ruvnet/cognitum-claude-plugin` or wherever the COGNITUM
volume image is generated):

**a. Ship a udisks2 override on the volume.** The 14-line file at
`crates/ruvector-cognitum-host/deploy/cognitum-fat.conf` (this repo)
solves the problem when dropped into
`/etc/udisks2/mount_options.conf.d/`. The Seed's existing
`trust/install-trust.sh` already runs `sudo` for the CA install —
adding a single extra line to also install this override would fix
the problem permanently on every Linux host the user ever plugs the
Seed into.

**b. Update `README.txt`'s Linux quick-start.** Replace
`Run ./launch.sh` with one of:

- `Run: bash ./launch.sh` (works without any setup; the +x bit
  isn't needed when invoking through bash explicitly), OR
- `Run: bash trust/install-trust.sh && ./launch.sh` (CA installer
  patched per (a) to also drop the udisks2 override; subsequent
  plug-ins work via double-click)

Either path eliminates the GNOME dialog for every Linux user.

Local screenshot of the user-facing failure:
`/home/drdave/Pictures/Screenshots/Screenshot From 2026-05-24 21-35-51.png`.
