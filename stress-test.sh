#!/bin/bash
set -e

echo "=== Chat Server Stress Test ==="
echo ""

# Build in release mode for performance
echo "Building in release mode..."
cargo build --release --workspace

# Kill any existing server
pkill -f "target/release/server" 2>/dev/null || true
sleep 1

# Start server
echo "Starting server..."
./target/release/server > logs/stress-server.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"
sleep 2

# Configuration
NUM_CLIENTS=${NUM_CLIENTS:-50}
MESSAGES_PER_CLIENT=${MESSAGES_PER_CLIENT:-10}
CONCURRENT_LIMIT=${CONCURRENT_LIMIT:-25}

echo ""
echo "Configuration:"
echo "  Clients: $NUM_CLIENTS"
echo "  Messages per client: $MESSAGES_PER_CLIENT"
echo "  Concurrent limit: $CONCURRENT_LIMIT"
echo ""

mkdir -p logs

# Function to run a single client
run_client() {
    local client_num=$1
    local username="stress_user_${client_num}"

    {
        # Send messages
        for i in $(seq 1 $MESSAGES_PER_CLIENT); do
            echo "send Stress test message $i from $username"
            sleep 0.01  # Small delay between messages
        done
        echo "leave"
    } | timeout 30 ./target/release/client --username "$username" > "logs/stress-client-${client_num}.log" 2>&1

    if [ $? -eq 0 ]; then
        echo "✓ Client $client_num completed"
    else
        echo "✗ Client $client_num failed"
        return 1
    fi
}

# Export function for parallel execution
export -f run_client
export MESSAGES_PER_CLIENT

# Run clients
echo "Starting clients..."
START_TIME=$(date +%s)

# Run clients in batches to control concurrency
for batch_start in $(seq 1 $CONCURRENT_LIMIT $NUM_CLIENTS); do
    batch_end=$((batch_start + CONCURRENT_LIMIT - 1))
    if [ $batch_end -gt $NUM_CLIENTS ]; then
        batch_end=$NUM_CLIENTS
    fi

    echo "Running batch: clients $batch_start to $batch_end"

    # Run batch in parallel
    pids=()
    for i in $(seq $batch_start $batch_end); do
        run_client $i &
        pids+=($!)
    done

    # Wait for batch to complete
    for pid in "${pids[@]}"; do
        wait $pid || echo "Client failed"
    done
done

END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

echo ""
echo "=== Results ==="
echo "Total time: ${ELAPSED}s"
echo "Total messages sent: $((NUM_CLIENTS * MESSAGES_PER_CLIENT))"
echo "Messages per second: $((NUM_CLIENTS * MESSAGES_PER_CLIENT / ELAPSED))"
echo ""

# Check server is still running
if ps -p $SERVER_PID > /dev/null; then
    echo "✅ Server is still running"
else
    echo "❌ Server crashed!"
    cat logs/stress-server.log
    exit 1
fi

# Clean up
echo ""
echo "Stopping server..."
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

# Show server stats
echo ""
echo "=== Server Log Summary ==="
grep -c "joined" logs/stress-server.log || echo "0 joins"
grep -c "left" logs/stress-server.log || echo "0 leaves"
echo ""

# Clean up client logs
rm -f logs/stress-client-*.log

echo "✅ Stress test completed successfully!"
