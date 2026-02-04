#!/bin/bash
# Simple end-to-end test script

set -e

PORT=9999
SERVER_LOG=$(mktemp)
CLIENT1_LOG=$(mktemp)
CLIENT2_LOG=$(mktemp)

echo "Starting server on port $PORT..."
CHAT_HOST=127.0.0.1:$PORT cargo run --bin server --release > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 2

echo "Starting client 1 (alice)..."
{
    sleep 1
    echo "send Hello from Alice!"
    sleep 1
    echo "leave"
} | cargo run --bin client --release -- --port $PORT --username alice > "$CLIENT1_LOG" 2>&1 &
CLIENT1_PID=$!

echo "Starting client 2 (bob)..."
{
    sleep 1
    echo "send Hello from Bob!"
    sleep 2
    echo "leave"
} | cargo run --bin client --release -- --port $PORT --username bob > "$CLIENT2_LOG" 2>&1 &
CLIENT2_PID=$!

# Wait for clients to finish
wait $CLIENT1_PID 2>/dev/null || true
wait $CLIENT2_PID 2>/dev/null || true

# Stop server
kill $SERVER_PID 2>/dev/null || true

echo ""
echo "=== Server Log ==="
cat "$SERVER_LOG"

echo ""
echo "=== Client 1 (alice) Log ==="
cat "$CLIENT1_LOG"

echo ""
echo "=== Client 2 (bob) Log ==="
cat "$CLIENT2_LOG"

# Cleanup
rm -f "$SERVER_LOG" "$CLIENT1_LOG" "$CLIENT2_LOG"

echo ""
echo "✅ End-to-end test completed!"
