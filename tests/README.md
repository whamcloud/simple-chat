# Integration Tests

This directory contains comprehensive integration tests for the Simple Chat application.

## Test Structure

### Server Integration Tests (`server_integration.rs`)
- **Server startup and basic connection**: Tests server startup and client connections
- **Multiple client connections**: Verifies concurrent client handling
- **Message broadcasting**: Tests message distribution between clients
- **Username uniqueness**: Ensures usernames cannot be duplicated
- **Client disconnect cleanup**: Verifies proper cleanup when clients disconnect
- **Server max users limit**: Tests user capacity limits
- **Invalid message handling**: Tests error handling for malformed messages

### Client-Server Integration Tests (`client_server_integration.rs`)
- **Full communication flow**: End-to-end message exchange testing
- **Join, send, leave operations**: Tests complete user lifecycle
- **Error handling scenarios**: Tests various error conditions
- **Concurrent client operations**: Tests multiple clients operating simultaneously
- **Connection failure scenarios**: Tests network error handling

### End-to-End Scenario Tests (`end_to_end_scenarios.rs`)
- **Multiple clients chatting**: Realistic chat scenario with many participants
- **Client disconnect during chat**: Tests mid-conversation disconnections
- **Server shutdown with active clients**: Tests graceful/forced server shutdown
- **High load stress testing**: Performance testing under load

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Specific Test Suites
```bash
# Server integration tests only
cargo test --test server_integration

# Client-server integration tests only
cargo test --test client_server_integration

# End-to-end scenario tests only
cargo test --test end_to_end_scenarios
```

### Run Individual Tests
```bash
# Run a specific test
cargo test test_server_startup_and_basic_connection

# Run tests matching a pattern
cargo test message_broadcasting
```

### Run Tests with Output
```bash
# Show test output (including server logs)
cargo test -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test
```

## Test Features

### Automatic Server Management
- Tests automatically start server processes on available ports
- Proper cleanup of server processes after tests complete
- No manual setup required

### Realistic Scenarios
- Tests use actual TCP connections
- Message serialization/deserialization is fully tested
- Concurrent operations are properly tested

### Comprehensive Coverage
- **Unit tests**: Message protocol and validation (in `src/lib.rs`)
- **Integration tests**: Server-client interactions
- **End-to-end tests**: Complete application workflows
- **Error scenarios**: Network failures, invalid inputs, edge cases

### Test Reliability
- Automatic port allocation to avoid conflicts
- Proper message sequencing and timing
- Robust error handling for test flakiness

## Test Requirements

- All tests are self-contained and require no external dependencies
- Tests can run in parallel safely
- No manual server startup required
- Compatible with standard `cargo test` workflow

## Performance Notes

- Integration tests spawn actual server processes, so they're slower than unit tests
- Some tests include intentional delays for message propagation
- High-load tests may take longer on slower systems
- Tests are designed to be reliable across different system loads