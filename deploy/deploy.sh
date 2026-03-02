#!/usr/bin/env bash
set -euo pipefail

SERVER="jpdata"
APP_DIR="/opt/ally"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Deploying Ally to $SERVER ==="

# Step 1: Build frontend
echo ">>> Building frontend..."
cd "$PROJECT_DIR/frontend"
npm run build
cd "$PROJECT_DIR"

# Step 2: Sync codebase
echo ">>> Syncing files to server..."
rsync -azP --delete \
  --exclude='.git' \
  --exclude='target' \
  --exclude='node_modules' \
  --exclude='frontend/node_modules' \
  --exclude='.DS_Store' \
  --exclude='.env' \
  ./ "$SERVER:$APP_DIR/src/"

# Step 3: Sync frontend dist separately
echo ">>> Syncing frontend dist..."
rsync -azP --delete \
  frontend/dist/ "$SERVER:$APP_DIR/frontend/dist/"

# Step 4: Build on server + install services + restart
echo ">>> Building & restarting on server..."
ssh "$SERVER" bash -c "'
  set -euo pipefail
  cd $APP_DIR/src

  # Compile Rust
  source /root/.cargo/env
  echo \">>> cargo build --release...\"
  cargo build --release 2>&1 | tail -5

  # Copy binaries (stop first to avoid Text file busy)
  systemctl stop ally-server ally-worker ally-watchdog 2>/dev/null || true
  mkdir -p $APP_DIR/bin
  cp target/release/waoowaoo-server $APP_DIR/bin/
  cp target/release/waoowaoo-worker $APP_DIR/bin/
  cp target/release/waoowaoo-watchdog $APP_DIR/bin/

  # Install systemd services
  cp deploy/ally-server.service /etc/systemd/system/
  cp deploy/ally-worker.service /etc/systemd/system/
  cp deploy/ally-watchdog.service /etc/systemd/system/
  systemctl daemon-reload

  # Nginx config
  cp deploy/nginx-ally.conf /etc/nginx/sites-available/ally
  ln -sf /etc/nginx/sites-available/ally /etc/nginx/sites-enabled/ally
  rm -f /etc/nginx/sites-enabled/default
  nginx -t && systemctl reload nginx

  # Restart services
  systemctl restart ally-server ally-worker ally-watchdog
  systemctl enable ally-server ally-worker ally-watchdog

  sleep 3
  echo \">>> Health check...\"
  curl -sf http://localhost:3001/healthz && echo \" ✅ API OK\" || echo \" ❌ API FAILED\"
  echo \">>> Service status:\"
  systemctl is-active ally-server ally-worker ally-watchdog
'"

echo ""
echo "=== Deploy complete ==="
echo "  API: http://185.200.65.233/api/"
echo "  Web: http://185.200.65.233"
