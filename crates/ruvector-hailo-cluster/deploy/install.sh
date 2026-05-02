#!/usr/bin/env bash
# Install ruvector-hailo-worker on a Pi 5 + AI HAT+.
#
# Run on the Pi (not on a dev host) after building the binary with:
#   cargo build --release --features hailo --bin ruvector-hailo-worker
#
# Idempotent — re-run after upgrading the binary.
#
# Usage:
#   sudo bash install.sh /path/to/ruvector-hailo-worker /path/to/models-dir

set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  echo "must run as root (use sudo)" >&2; exit 1
fi
if [[ $# -lt 2 ]]; then
  echo "usage: $0 <path/to/ruvector-hailo-worker> <path/to/models-dir>" >&2
  echo "  models-dir must contain model.hef, vocab.txt, special_tokens.json" >&2
  exit 1
fi

WORKER_BIN="$1"
MODELS_SRC="$2"

if [[ ! -x "$WORKER_BIN" ]]; then
  echo "binary not executable: $WORKER_BIN" >&2; exit 1
fi
if [[ ! -d "$MODELS_SRC" ]]; then
  echo "models dir not found: $MODELS_SRC" >&2; exit 1
fi
if [[ ! -f "$MODELS_SRC/model.hef" ]]; then
  echo "warning: $MODELS_SRC/model.hef missing — worker will fail to start" >&2
fi

DEPLOY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
USER_NAME="${SUDO_USER:-genesis}"

echo "==> install binary"
install -o root -g root -m 0755 "$WORKER_BIN" /usr/local/bin/ruvector-hailo-worker

echo "==> install models -> /var/lib/ruvector-hailo/models/all-minilm-l6-v2"
install -d -o "$USER_NAME" -g "$USER_NAME" -m 0755 \
  /var/lib/ruvector-hailo/models/all-minilm-l6-v2
cp -a "$MODELS_SRC/." /var/lib/ruvector-hailo/models/all-minilm-l6-v2/
chown -R "$USER_NAME":"$USER_NAME" /var/lib/ruvector-hailo

echo "==> install /etc/ruvector-hailo.env (skipped if exists)"
if [[ ! -f /etc/ruvector-hailo.env ]]; then
  install -o root -g root -m 0644 "$DEPLOY_DIR/ruvector-hailo.env.example" /etc/ruvector-hailo.env
  echo "    -> wrote default; edit if non-default bind/model dir wanted"
else
  echo "    -> existing /etc/ruvector-hailo.env preserved"
fi

echo "==> install systemd unit"
sed "s|^User=genesis|User=$USER_NAME|; s|^Group=genesis|Group=$USER_NAME|; s|/home/genesis|/home/$USER_NAME|g" \
  "$DEPLOY_DIR/ruvector-hailo-worker.service" \
  > /etc/systemd/system/ruvector-hailo-worker.service
chmod 0644 /etc/systemd/system/ruvector-hailo-worker.service

echo "==> daemon-reload + enable"
systemctl daemon-reload
systemctl enable ruvector-hailo-worker.service

echo
echo "Installed. To start now:"
echo "    sudo systemctl start ruvector-hailo-worker"
echo "Tail logs:"
echo "    journalctl -u ruvector-hailo-worker -f"
