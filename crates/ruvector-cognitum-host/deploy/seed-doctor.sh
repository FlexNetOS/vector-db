#!/usr/bin/env bash
# Cognitum Seed — connectivity doctor
#
# Probes every documented transport and prints a PASS/FAIL matrix.
# Exit 0 if the Seed responds on at least one transport; exit 1 otherwise.

set -u

PASS=0
FAIL=0
TOKEN_PATH="$HOME/.config/cognitum-mcp/token"
REPO_MCP_JSON="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)/.mcp.json"

probe() {
    local name="$1" cmd="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        printf "  [PASS] %s\n" "$name"
        PASS=$((PASS + 1))
    else
        printf "  [FAIL] %s\n" "$name"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== Cognitum Seed Doctor ==="
echo ""

echo "USB-gadget device:"
probe "USB device 1d6b:0104 enumerated"                "lsusb | grep -q '1d6b:0104'"
probe "Net iface exists (enx* — cdc_ncm or rndis_host)" "ip -br link | grep -qE '^enx'"
probe "Link-local IPv4 assigned to USB iface"           "ip -br addr | awk '/^enx/ && /169\\.254/' | grep -q ."

echo ""
echo "Transports (each must work for its scope):"
probe "USB HTTP   http://169.254.42.1/api/v1/status"        "curl -s  --max-time 4 http://169.254.42.1/api/v1/status | grep -q device_id"
probe "USB HTTPS  https://169.254.42.1:8443/api/v1/status"  "curl -sk --max-time 4 https://169.254.42.1:8443/api/v1/status | grep -q device_id"
probe "mDNS hostname cognitum-578b.local resolves"          "getent hosts cognitum-578b.local | grep -q ."
probe "mDNS HTTPS https://cognitum-578b.local:8443/status"  "curl -sk --max-time 4 https://cognitum-578b.local:8443/api/v1/status | grep -q device_id"

echo ""
echo "Auth + trust:"
probe "Token at $TOKEN_PATH"                            "[[ -s '$TOKEN_PATH' ]]"
probe "Cognitum CA in /usr/local/share/ca-certificates" "[[ -f /usr/local/share/ca-certificates/cognitum-ca.crt ]]"
if [[ -s "$TOKEN_PATH" ]]; then
    probe "Token-authed GET /api/v1/identity"           "curl -sk --max-time 4 -H \"Authorization: Bearer \$(cat '$TOKEN_PATH')\" https://169.254.42.1:8443/api/v1/identity | grep -q device_id"
fi

echo ""
echo "Host plumbing (this crate's deploy/ artifacts):"
probe "NM keyfile installed"          "[[ -f /etc/NetworkManager/system-connections/cognitum-seed.nmconnection ]]"
probe "udev rule installed"           "[[ -f /etc/udev/rules.d/70-cognitum-seed.rules ]]"
probe "udisks2 override installed"    "[[ -f /etc/udisks2/mount_options.conf.d/cognitum-fat.conf ]]"
probe "avahi-daemon running (mDNS)"   "systemctl is-active --quiet avahi-daemon"

echo ""
echo "MCP registration:"
probe "Project .mcp.json has cognitum-seed entry"  "grep -q 'cognitum-seed' '$REPO_MCP_JSON'"

echo ""
echo "=== Summary: $PASS PASS, $FAIL FAIL ==="

if curl -s --max-time 4 http://169.254.42.1/api/v1/status 2>/dev/null | grep -q device_id; then
    echo "Seed is REACHABLE via at least one transport. OK"
    exit 0
else
    echo "Seed is NOT reachable on any transport. Check 'FAIL' rows above."
    echo ""
    echo "Common fixes:"
    echo "  - Replug the Seed (NM keyfile applies on iface (re)add)"
    echo "  - Verify USB cable is a data cable, not power-only"
    echo "  - sudo nmcli connection up cognitum-seed   (force-apply keyfile)"
    echo "  - Re-run this crate's deploy/install.sh"
    exit 1
fi
