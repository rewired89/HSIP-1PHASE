#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <ADMIN_API_KEY>"
    echo ""
    echo "Example:"
    echo "  $0 hsip_abc123..."
    exit 1
fi

ADMIN_KEY="$1"
BASE_URL="${HSIP_BASE_URL:-http://localhost:3000}"

echo "========================================="
echo "HSIP API Load Test: Identity Creation"
echo "========================================="
echo ""

echo "Target: $BASE_URL/v1/identity"
echo "Duration: 60 seconds"
echo "Threads: 4"
echo "Connections: 50"
echo "Method: POST (authenticated)"
echo ""

if ! command -v wrk &> /dev/null; then
    echo "ERROR: wrk not found. Install with:"
    echo "  Ubuntu: sudo apt-get install wrk"
    echo "  macOS:  brew install wrk"
    exit 1
fi

# Create Lua script for POST requests
cat > /tmp/create_identity.lua <<'EOF'
wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"
wrk.headers["Authorization"] = "Bearer " .. os.getenv("ADMIN_KEY")
wrk.body = "{}"
EOF

echo "Starting load test..."
echo ""

export ADMIN_KEY
wrk -t4 -c50 -d60s \
    --latency \
    -s /tmp/create_identity.lua \
    "$BASE_URL/v1/identity"

echo ""
echo "Load test complete!"
echo ""
echo "NOTE: This test creates real identities in the database."
echo "Clean up with: DELETE FROM identities WHERE tenant_id LIKE 'test-%';"
