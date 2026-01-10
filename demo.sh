#!/bin/bash
# Demo script for chat server and client

set -e

echo "Building chat server and client..."
source $HOME/.cargo/env
cd "$(dirname "$0")"

cargo build --release --quiet

echo "✓ Build successful"
echo ""
echo "Starting chat server..."
./target/release/server &
SERVER_PID=$!
echo "✓ Server started (PID: $SERVER_PID)"

# Give server time to start
sleep 1

echo ""
echo "Connecting 2 clients and sending messages..."
echo ""

# Function to send commands to a client
send_client_commands() {
    local username=$1
    local messages=$2
    echo "$messages" | timeout 5 ./target/release/client -u "$username" 2>/dev/null || true
}

# Client 1 sends a message
echo "--- Client 'alice' ---"
send_client_commands "alice" "send Hello from alice
leave" &
ALICE_PID=$!

sleep 0.5

# Client 2 sends a message
echo ""
echo "--- Client 'bob' ---"
send_client_commands "bob" "send Hi from bob
leave" &
BOB_PID=$!

# Wait for clients to finish
wait $ALICE_PID 2>/dev/null || true
wait $BOB_PID 2>/dev/null || true
