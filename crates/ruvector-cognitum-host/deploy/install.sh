#!/usr/bin/env bash
# Cognitum Seed — host setup installer
#
# Idempotent: each target is diff'd against the source; we only write +
# reload if something actually changed. Re-running is a no-op.
#
# Requires sudo for /etc/* writes (three files, all under 1 KB).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

UDISKS_TARGET=/etc/udisks2/mount_options.conf.d/cognitum-fat.conf
UDEV_TARGET=/etc/udev/rules.d/70-cognitum-seed.rules
NM_TARGET=/etc/NetworkManager/system-connections/cognitum-seed.nmconnection

CHANGED=0

install_file() {
    local src="$1" dst="$2" mode="$3"
    if [[ ! -f "$src" ]]; then
        echo "  ERROR: source file missing: $src" >&2
        exit 2
    fi
    if [[ ! -f "$dst" ]] || ! sudo cmp -s "$src" "$dst"; then
        sudo install -D -m "$mode" "$src" "$dst"
        echo "  installed   $dst (mode $mode)"
        CHANGED=1
    else
        echo "  unchanged   $dst"
    fi
}

echo "=== Cognitum Seed host setup ==="
echo ""
echo "Installing config files (may prompt for sudo):"
install_file "$SCRIPT_DIR/cognitum-fat.conf"            "$UDISKS_TARGET" 0644
install_file "$SCRIPT_DIR/70-cognitum-seed.rules"       "$UDEV_TARGET"   0644
install_file "$SCRIPT_DIR/cognitum-seed.nmconnection"   "$NM_TARGET"     0600

if [[ $CHANGED -eq 1 ]]; then
    echo ""
    echo "Reloading subsystems:"
    sudo udevadm control --reload-rules
    sudo udevadm trigger --subsystem-match=net --action=change
    sudo nmcli connection reload
    echo "  udev + NetworkManager reloaded"

    # If COGNITUM is already mounted, the new udisks2 perms won't apply until
    # remount. Surface this so the user knows to unplug + replug (or sudo umount + replug).
    MOUNT_POINT="/run/media/${USER}/COGNITUM"
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        echo ""
        echo "  NOTE: COGNITUM volume is currently mounted at $MOUNT_POINT"
        echo "        New executable perms will apply after unplug + replug."
    fi
else
    echo ""
    echo "All files up to date; no reload needed."
fi

echo ""
echo "Running doctor to verify..."
echo ""
exec "$SCRIPT_DIR/seed-doctor.sh"
