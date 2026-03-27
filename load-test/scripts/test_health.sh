#!/bin/bash
set -e

echo "========================================="
echo "HSIP API Load Test: Health Endpoint"
echo "========================================="
echo ""

BASE_URL="${HSIP_BASE_URL:-http://localhost:3000}"

echo "Target: $BASE_URL/health"
echo "Duration: 60 seconds"
echo "Threads: 4"
echo "Connections: 100"
echo ""

if ! command -v wrk &> /dev/null; then
    echo "ERROR: wrk not found. Install with:"
    echo "  Ubuntu: sudo apt-get install wrk"
    echo "  macOS:  brew install wrk"
    exit 1
fi

echo "Starting load test..."
echo ""

wrk -t4 -c100 -d60s \
    --latency \
    "$BASE_URL/health"

echo ""
echo "Load test complete!"
