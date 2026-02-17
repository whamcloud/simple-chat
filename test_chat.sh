#!/bin/bash

# Test script for simple-chat

# Start server
cargo run --bin server &
SERVER_PID=$!
echo "Started server with PID $SERVER_PID"
sleep 2

# Test 1: Client connection and send message
echo "Test 1: Sending a message from client"
echo -e "send Hello from test\nleave" | cargo run --bin client -- --username testuser --host 127.0.0.1 --port 8080 &
CLIENT1_PID=$!

sleep 2

# Test 2: Two clients
echo "Test 2: Two clients chatting"
(echo -e "send Hello from Alice\nsleep 2\nleave" | cargo run --bin client -- --username alice --host 127.0.0.1 --port 8080) &
CLIENT2_PID=$!

sleep 1

(echo -e "send Hi Alice from Bob\nsleep 2\nleave" | cargo run --bin client -- --username bob --host 127.0.0.1 --port 8080) &
CLIENT3_PID=$!

sleep 4

# Cleanup
echo "Cleaning up..."
kill $CLIENT1_PID 2>/dev/null || true
kill $CLIENT2_PID 2>/dev/null || true
kill $CLIENT3_PID 2>/dev/null || true
kill $SERVER_PID
wait

echo "Test complete!"
