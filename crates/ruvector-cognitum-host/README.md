# ruvector-cognitum-host

Host-side plumbing for connecting a [Cognitum Seed](https://cognitum.one) USB-gadget appliance to a Linux host. Companion to the vendored `cognitum-one` Rust SDK at `vendor/cognitum-one/` and ADR-163 (FlexNetOS Unification & Cognitum Integration).

## What this fixes

The Cognitum Seed enumerates as a USB composite gadget (VID/PID `1d6b:0104`, manufacturer `RuVector`) and exposes:

- A network adapter at static link-local `169.254.42.1` (HTTPS port 8443, plus HTTP port 80 for USB-only trust)
- A USB mass-storage volume (label `COGNITUM`) carrying the CA cert, launchers, and an offline setup guide

Out of the box on Linux this fails three ways:

1. **NetworkManager tries DHCP** on the USB-CDC interface and loops forever — the Seed serves static link-local with no DHCP responder, so the host never gets an IPv4 address on that link and `169.254.42.1` is unreachable.
2. **The COGNITUM USB volume mounts with `showexec`** (default udisks2 vfat option), which strips `+x` from `.sh` files. The vendor's `launch.sh` and `install-trust.sh` are unrunnable via double-click, and GNOME's autorun pops a "program is not marked as executable" dialog.
3. **No stable identifier** for the Seed exists (interface names are MAC-derived per-device), so any config tied to one Seed breaks for another.

Upstream `ruvnet/*` ships no host-side fix for any of these — see the agent research log in this branch's commit message.

## What's in `deploy/`

| File | Installed to | Purpose |
|---|---|---|
| `cognitum-fat.conf` | `/etc/udisks2/mount_options.conf.d/cognitum-fat.conf` | Per-label override: COGNITUM volumes mount without `showexec` so `.sh` files are executable. |
| `cognitum-seed.nmconnection` | `/etc/NetworkManager/system-connections/cognitum-seed.nmconnection` (mode 600) | Matches any USB-CDC driver (`cdc_ncm`, `cdc_ether`, `rndis_host`), uses `ipv4.method=link-local`. Works for any Seed, survives replug. |
| `70-cognitum-seed.rules` | `/etc/udev/rules.d/70-cognitum-seed.rules` | Matches VID/PID + manufacturer + product strings; tags `systemd`; creates `/dev/cognitum-seed-usb` symlink for the mass-storage device. |
| `install.sh` | n/a (run in place) | Idempotent installer. Diffs each target; only writes + reloads if changed. |
| `seed-doctor.sh` | n/a (run in place) | Probes USB device presence, iface state, all three transports (HTTP/HTTPS/mDNS), token + CA, host plumbing, MCP registration. Prints PASS/FAIL matrix. |

## Install

```bash
crates/ruvector-cognitum-host/deploy/install.sh
```

Requires `sudo` for the three `/etc/*` writes. Idempotent — re-runs are no-ops if nothing changed.

## Verify

```bash
crates/ruvector-cognitum-host/deploy/seed-doctor.sh
```

Exit code 0 if the Seed responds on at least one transport.

## MCP registration

After install + replug, the project `.mcp.json` registers the Seed as a direct HTTP MCP server (no proxy, no auth, USB cable as trust anchor):

```json
"cognitum-seed": {
  "type": "http",
  "url": "http://169.254.42.1/mcp",
  "autoStart": false
}
```

For Wi-Fi mode (after pairing the Seed to Wi-Fi via the on-device SSH path documented in `STATUS.txt`), use the mDNS-resolved HTTPS endpoint with bearer auth — see the `cognitum-seed-wifi` entry in `.mcp.json`.

## What's **NOT** done here

- The future Rust adapter `clawft-substrate-cognitum` (ADR-163 M3) — separate work item.
- Tailscale-based fleet management (per RuView ADR-124) — out of v1 scope.
- Multi-Seed disambiguation on a single host (both would claim `169.254.42.1`). The doctor flags this; the Tailscale path is the documented solution.

## See also

- `vendor/cognitum-one/` — vendored Rust SDK with `SeedClient`, `MdnsDiscovery`, `TailscaleDiscovery`, `SeedTls::Pinned`
- `docs/adr/ADR-163-flexnetos-unification-cognitum-integration.md` — operative host-contract spec
- `/run/media/$USER/COGNITUM/STATUS.txt` — device identity + endpoints (printed by the firmware on each plug-in)
- `/run/media/$USER/COGNITUM/open.html` — vendor's browser-based connection launcher (auto-detect USB/mDNS/Wi-Fi)
