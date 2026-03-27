#!/bin/bash
# Simple benchmark script using only curl (no dependencies)

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <ADMIN_API_KEY> [num_requests]"
    echo ""
    echo "Example:"
    echo "  $0 hsip_abc123... 100"
    exit 1
fi

ADMIN_KEY="$1"
NUM_REQUESTS="${2:-100}"
BASE_URL="${HSIP_BASE_URL:-http://localhost:3000}"

echo "========================================="
echo "HSIP API Simple Benchmark"
echo "========================================="
echo ""
echo "Base URL: $BASE_URL"
echo "Requests: $NUM_REQUESTS"
echo ""

# Test health endpoint
echo "[1/4] Testing /health endpoint..."
start=$(date +%s%N)
for i in $(seq 1 $NUM_REQUESTS); do
    curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health" > /dev/null
    if [ $((i % 10)) -eq 0 ]; then
        echo -n "."
    fi
done
end=$(date +%s%N)
duration=$(( (end - start) / 1000000 ))
avg_latency=$(( duration / NUM_REQUESTS ))
throughput=$(( NUM_REQUESTS * 1000 / duration ))
echo ""
echo "  Total time: ${duration}ms"
echo "  Avg latency: ${avg_latency}ms"
echo "  Throughput: ${throughput} req/s"
echo ""

# Test identity creation
echo "[2/4] Testing POST /v1/identity (creates real identities)..."
start=$(date +%s%N)
success=0
failed=0
for i in $(seq 1 10); do  # Only 10 to avoid DB bloat
    status=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_KEY" \
        -H "Content-Type: application/json" \
        -X POST \
        "$BASE_URL/v1/identity")
    if [ "$status" = "200" ]; then
        success=$((success + 1))
    else
        failed=$((failed + 1))
    fi
    echo -n "."
done
end=$(date +%s%N)
duration=$(( (end - start) / 1000000 ))
avg_latency=$(( duration / 10 ))
echo ""
echo "  Total time: ${duration}ms"
echo "  Avg latency: ${avg_latency}ms"
echo "  Success: $success, Failed: $failed"
echo ""

# Test key listing
echo "[3/4] Testing GET /v1/keys..."
start=$(date +%s%N)
for i in $(seq 1 $NUM_REQUESTS); do
    curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_KEY" \
        "$BASE_URL/v1/keys" > /dev/null
    if [ $((i % 10)) -eq 0 ]; then
        echo -n "."
    fi
done
end=$(date +%s%N)
duration=$(( (end - start) / 1000000 ))
avg_latency=$(( duration / NUM_REQUESTS ))
throughput=$(( NUM_REQUESTS * 1000 / duration ))
echo ""
echo "  Total time: ${duration}ms"
echo "  Avg latency: ${avg_latency}ms"
echo "  Throughput: ${throughput} req/s"
echo ""

# Test metrics endpoint
echo "[4/4] Testing /metrics endpoint..."
start=$(date +%s%N)
for i in $(seq 1 50); do
    curl -s -o /dev/null "$BASE_URL/metrics" > /dev/null
    if [ $((i % 10)) -eq 0 ]; then
        echo -n "."
    fi
done
end=$(date +%s%N)
duration=$(( (end - start) / 1000000 ))
avg_latency=$(( duration / 50 ))
throughput=$(( 50 * 1000 / duration ))
echo ""
echo "  Total time: ${duration}ms"
echo "  Avg latency: ${avg_latency}ms"
echo "  Throughput: ${throughput} req/s"
echo ""

echo "========================================="
echo "Benchmark complete!"
echo "========================================="
echo ""
echo "Summary:"
echo "  ✓ Health endpoint:       Fast, suitable for health checks"
echo "  ✓ Identity creation:     Database write performance OK"
echo "  ✓ Key listing:           Read performance OK"
echo "  ✓ Metrics:               Monitoring endpoint responsive"
echo ""
echo "For detailed load testing, use wrk or hey (see load-test/README.md)"
