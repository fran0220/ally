#!/usr/bin/env bash
# First-time server setup for jpdata
# Run: ssh jpdata 'bash -s' < deploy/setup-server.sh
set -euo pipefail

APP_DIR="/opt/ally"

echo "=== Setting up Ally on $(hostname) ==="

# Create directory structure
mkdir -p "$APP_DIR"/{bin,src,frontend/dist,uploads,logs}

# Ensure Rust toolchain
if ! command -v cargo &>/dev/null; then
  source /root/.cargo/env 2>/dev/null || true
fi
if ! command -v cargo &>/dev/null; then
  echo ">>> Installing Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source /root/.cargo/env
fi
echo "Rust: $(rustc --version)"

# Docker infra (if not running)
if ! docker ps | grep -q ally-mysql; then
  echo ">>> Starting Docker infra..."
  cd "$APP_DIR/src"
  docker compose -f deploy/docker-compose.infra.yml up -d
fi

# .env
if [ ! -f "$APP_DIR/.env" ]; then
  echo "⚠️  Create $APP_DIR/.env from .env.example before deploying"
fi

echo ""
echo "=== Setup Complete ==="
echo "  Next: run 'bash deploy/deploy.sh' from local machine"
