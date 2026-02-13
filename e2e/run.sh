#!/usr/bin/env bash
#
# E2E test runner for OpenERP.
#
# Usage:
#   ./e2e/run.sh              # Run with existing server
#   ./e2e/run.sh --start      # Start server, run tests, stop server
#   HEADLESS=false ./e2e/run.sh   # Show browser window
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BASE_URL="${BASE_URL:-http://localhost:8088}"

# Check if server is running.
check_server() {
  curl -sf "${BASE_URL}/health" > /dev/null 2>&1
}

# Start server if requested.
SERVER_PID=""
if [[ "${1:-}" == "--start" ]]; then
  echo "Building openerpd..."
  cd "$ROOT_DIR"
  bazel build //rust/bin/openerpd 2>&1

  # Create temp context if needed.
  CONF_DIR="/tmp/openerp-e2e/config"
  DATA_DIR="/tmp/openerp-e2e/data"
  mkdir -p "$CONF_DIR" "$DATA_DIR"

  if [[ ! -f "$CONF_DIR/e2e.toml" ]]; then
    echo "Creating E2E test context..."
    bazel build //rust/bin/openerp 2>&1
    expect -c "
      spawn bazel-bin/rust/bin/openerp/openerp context create e2e \
        --config-dir $CONF_DIR --data-dir $DATA_DIR \
        --config /tmp/openerp-e2e/client.toml
      expect \"Enter root password:\"
      send \"openerp123\r\"
      expect \"Confirm root password:\"
      send \"openerp123\r\"
      expect eof
    "
  fi

  echo "Starting openerpd..."
  RUST_LOG=info "$ROOT_DIR/bazel-bin/rust/bin/openerpd/openerpd" \
    -c "$CONF_DIR/e2e.toml" --listen 0.0.0.0:8088 &
  SERVER_PID=$!

  # Wait for server.
  for i in $(seq 1 30); do
    if check_server; then break; fi
    sleep 1
  done

  if ! check_server; then
    echo "ERROR: Server failed to start"
    kill "$SERVER_PID" 2>/dev/null || true
    exit 1
  fi
  echo "Server ready at $BASE_URL"
fi

# Run tests.
cd "$SCRIPT_DIR"
export BASE_URL
export ROOT_PASS="${ROOT_PASS:-openerp123}"

echo "Running E2E tests..."
node --test tests/*.test.mjs
EXIT_CODE=$?

# Stop server if we started it.
if [[ -n "$SERVER_PID" ]]; then
  echo "Stopping server..."
  kill "$SERVER_PID" 2>/dev/null || true
fi

exit $EXIT_CODE
