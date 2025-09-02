# GitHub Actions CI/CD

This directory contains the continuous integration workflows for the Simple Chat application.

## Workflows

### `ci.yml` - Continuous Integration

**Triggers:**
- Push to `main` branch
- Pull requests to `main` branch

**Jobs:**

#### 1. Code Formatting Check (`format`)
- **Platform:** Ubuntu Latest
- **Timeout:** 10 minutes
- **Purpose:** Ensures code follows Rust formatting standards
- **Command:** `cargo fmt --all -- --check`

#### 2. Linting (`lint`)
- **Platform:** Ubuntu Latest
- **Timeout:** 15 minutes
- **Purpose:** Static analysis and linting with Clippy
- **Command:** `cargo clippy --all-targets --all-features -- -D warnings`
- **Features:** Treats warnings as errors for high code quality

#### 3. Unit and Integration Tests (`test`)
- **Platforms:** Ubuntu, Windows, macOS
- **Timeout:** 30 minutes
- **Purpose:** Comprehensive testing across platforms
- **Tests:**
  - Unit tests (`cargo test --lib`)
  - Integration tests (`cargo test --tests`)
  - Binary compilation verification

#### 4. End-to-End Integration Test (`e2e`)
- **Platform:** Ubuntu Latest
- **Timeout:** 20 minutes
- **Purpose:** Full system integration testing
- **Tests:**
  1. **Basic Client-Server Communication**
     - Starts server in background
     - Connects client and sends test message
     - Verifies successful communication
  
  2. **Multiple Clients Test**
     - Spawns multiple concurrent clients
     - Tests message exchange between users
     - Validates server handles concurrent connections
  
  3. **Server Responsiveness**
     - Checks server remains responsive after load
     - Validates connection acceptance after multiple clients
     - Ensures proper resource management

#### 5. Build Verification (`build-check`)
- **Platforms:** Ubuntu, Windows, macOS
- **Timeout:** 15 minutes  
- **Purpose:** Cross-platform build verification
- **Tests:**
  - Debug build compilation
  - Release build compilation
  - Binary help command functionality

#### 6. CI Success Summary (`ci-success`)
- **Platform:** Ubuntu Latest
- **Purpose:** Aggregates results from all jobs
- **Behavior:** Fails if any dependent job fails

## Features

### Robust Error Handling
- **Timeouts:** All jobs have appropriate timeout limits
- **Cleanup:** Proper server process cleanup in E2E tests
- **Trap handling:** Signal handling for graceful shutdowns
- **Exit codes:** Proper error reporting and propagation

### Performance Optimization
- **Caching:** Rust dependencies cached using `Swatinem/rust-cache`
- **Parallel execution:** Jobs run concurrently where possible
- **Incremental builds:** Cache keys based on `Cargo.lock` checksums

### Cross-Platform Support
- **Linux (Ubuntu):** Primary development and testing platform
- **Windows:** Compatibility verification  
- **macOS:** Apple silicon and x86_64 support
- **Binary verification:** Platform-specific binary testing

### Comprehensive Testing Strategy
- **Unit Tests:** Core functionality and message protocol
- **Integration Tests:** Server-client interaction
- **End-to-End Tests:** Real-world usage scenarios
- **Build Tests:** Compilation verification across platforms

## Usage

### Local Development
```bash
# Run the same checks locally before pushing
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

### Monitoring CI
- **Status:** Check the Actions tab in GitHub repository
- **Logs:** View detailed logs for each job step
- **Artifacts:** Download build artifacts if configured
- **Notifications:** Configure GitHub notifications for failures

### Troubleshooting

**Common Issues:**
- **Format failures:** Run `cargo fmt` to fix formatting
- **Clippy warnings:** Fix warnings or add `#[allow(...)]` where appropriate
- **Test timeouts:** Check for infinite loops or blocking operations
- **E2E failures:** Verify server starts correctly and ports are available

**Debug Steps:**
1. Check job logs for specific error messages
2. Reproduce issues locally using the same commands
3. Verify environment variables and dependencies
4. Test on the same platform as failing CI job

## Configuration

### Environment Variables
- `CARGO_TERM_COLOR=always` - Colored output in CI logs
- `RUST_BACKTRACE=1` - Full backtraces on panics
- `RUST_LOG=info` - Logging level for integration tests

### Timeouts
- **Format:** 10 minutes (formatting is fast)
- **Lint:** 15 minutes (clippy analysis takes time)
- **Tests:** 30 minutes (includes multiple platforms)
- **E2E:** 20 minutes (includes server startup and multiple test scenarios)
- **Build:** 15 minutes (cross-platform builds)

### Dependencies
- **Rust toolchain:** Managed by `dtolnay/rust-toolchain`
- **Caching:** Handled by `Swatinem/rust-cache`
- **Checkout:** Uses `actions/checkout@v4`