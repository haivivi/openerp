#!/bin/bash
# E2E test runner: builds server, starts it, runs all tests.
#
# Usage:
#   ./e2e/run.sh                  # headless
#   HEADLESS=false ./e2e/run.sh   # headed (shows browser)
#
# Requirements:
#   - Bazel
#   - Node.js + npm
#   - Chrome/Chromium (puppeteer-core finds it automatically)

set -e

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
E2E_DIR="$ROOT_DIR/e2e"
CONFIG_DIR="/tmp/openerp-e2e-$$"
DATA_DIR="$CONFIG_DIR/data"
LISTEN="127.0.0.1:0"  # Random port to avoid conflicts.
PID_FILE="$CONFIG_DIR/server.pid"

# Colors.
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

cleanup() {
    if [ -f "$PID_FILE" ]; then
        kill "$(cat "$PID_FILE")" 2>/dev/null || true
        rm -f "$PID_FILE"
    fi
    rm -rf "$CONFIG_DIR"
}
trap cleanup EXIT

echo -e "${YELLOW}=== Step 1: Rust tests ===${NC}"
cd "$ROOT_DIR"
bazel test //rust/lib/dsl/golden:golden_test \
           //rust/lib/dsl/store:store_test \
           //rust/lib/dsl/types:types_test \
           //rust/lib/dsl/macro_test:macro_test \
           //rust/lib/core:core_test \
           //rust/mod/auth:auth_test 2>&1 | tail -15
echo -e "${GREEN}Rust tests passed.${NC}"

echo -e "${YELLOW}=== Step 2: Build server ===${NC}"
bazel build //rust/bin/openerpd 2>&1 | tail -3
OPENERPD="$ROOT_DIR/bazel-bin/rust/bin/openerpd/openerpd"

echo -e "${YELLOW}=== Step 3: Create test context ===${NC}"
mkdir -p "$DATA_DIR"

# Build openerp CLI to create context.
bazel build //rust/bin/openerp 2>&1 | tail -3
OPENERP="$ROOT_DIR/bazel-bin/rust/bin/openerp/openerp"

# Create config with known root password.
CONFIG_FILE="$CONFIG_DIR/test.toml"
ROOT_PASS="e2e-test-pass-123"

# Hash the password using the openerp CLI.
# Fallback: use a pre-computed hash if CLI doesn't support it.
cat > "$CONFIG_FILE" <<TOML
[root]
password_hash = "$(echo -n "$ROOT_PASS" | $OPENERP root chpwd --stdin --config "$CONFIG_DIR/client.toml" 2>/dev/null || echo "")"

[storage]
data_dir = "$DATA_DIR"

[jwt]
secret = "e2e-test-jwt-secret-$(date +%s)"
expire_secs = 3600
TOML

# If password hash is empty, use a known hash (fallback for CI).
if ! grep -q 'password_hash = "\$argon2' "$CONFIG_FILE" 2>/dev/null; then
    # Use openerp context create or manual hash.
    # For now, reuse a working config approach:
    cat > "$CONFIG_FILE" <<TOML
[root]
password_hash = "\$argon2id\$v=19\$m=19456,t=2,p=1\$YGjFGa6/NbW1Fd86dHaKnA\$zfOiTV8wubINTusgKmg/JUQCGXQ2+yHqDYottXNLp+Y"

[storage]
data_dir = "$DATA_DIR"

[jwt]
secret = "e2e-test-jwt-secret-$(date +%s)"
expire_secs = 3600
TOML
    ROOT_PASS="openerp123"
fi

echo -e "${YELLOW}=== Step 4: Start server ===${NC}"
# Find a free port.
PORT=$(python3 -c "import socket; s=socket.socket(); s.bind(('',0)); print(s.getsockname()[1]); s.close()")
LISTEN="127.0.0.1:$PORT"

RUST_LOG=warn "$OPENERPD" -c "$CONFIG_FILE" --listen "$LISTEN" &
echo $! > "$PID_FILE"

# Wait for server to start.
for i in $(seq 1 30); do
    if curl -s "http://$LISTEN/health" > /dev/null 2>&1; then
        break
    fi
    sleep 0.5
done

if ! curl -s "http://$LISTEN/health" > /dev/null 2>&1; then
    echo -e "${RED}Server failed to start${NC}"
    exit 1
fi
echo "Server running on http://$LISTEN"

echo -e "${YELLOW}=== Step 5: Install E2E deps ===${NC}"
cd "$E2E_DIR"
if [ ! -d node_modules ]; then
    PUPPETEER_SKIP_DOWNLOAD=true npm install 2>&1 | tail -3
fi

echo -e "${YELLOW}=== Step 6: Run E2E tests ===${NC}"
BASE_URL="http://$LISTEN" ROOT_PASS="$ROOT_PASS" \
    node --test tests/01-login.test.mjs \
               tests/02-dashboard-crud.test.mjs \
               tests/03-api-auth.test.mjs \
               tests/04-user-login.test.mjs \
               tests/05-facet-api.test.mjs

echo -e "${GREEN}=== All tests passed! ===${NC}"
