//! ruvector-cognitum-host — deploy-only crate.
//!
//! No Rust code yet. Houses the host-side configuration files in `deploy/`
//! needed to connect a Cognitum Seed USB-gadget appliance on Linux:
//!
//!   - `cognitum-fat.conf`           — udisks2 mount override (drops `showexec`
//!                                     so `launch.sh` / `install-trust.sh` on
//!                                     the COGNITUM USB volume become executable)
//!   - `cognitum-seed.nmconnection`  — NetworkManager keyfile (assigns
//!                                     link-local IPv4 to any USB-CDC iface,
//!                                     no DHCP fight)
//!   - `70-cognitum-seed.rules`      — udev rule (stable symlink + systemd tag
//!                                     for the Seed's USB-gadget interfaces)
//!   - `install.sh`                  — idempotent installer (writes the three
//!                                     files to /etc only if changed, reloads
//!                                     udev + NM)
//!   - `seed-doctor.sh`              — connectivity probe matrix
//!
//! ADR-163 M3 names a future Rust adapter `clawft-substrate-cognitum` that
//! will consume the vendored `cognitum-one` SDK at `vendor/cognitum-one/`.
//! When that adapter lands, it can live alongside this crate's deploy/
//! artifacts (or in a sibling crate).
//!
//! See `README.md` for installation and rationale.
