#!/bin/bash
# Pre-commit hook for simple-chat
# Ensures code is formatted, compiles without errors, and passes clippy

set -e

echo "Running pre-commit checks..."

# Check formatting
echo "1. Checking code formatting..."
cargo fmt -- --check
if [ $? -ne 0 ]; then
    echo "❌ Code is not formatted. Run 'cargo fmt' to fix."
    exit 1
fi
echo "✅ Code formatting check passed"

# Check compilation
echo "2. Checking compilation..."
cargo check --all-targets
if [ $? -ne 0 ]; then
    echo "❌ Code does not compile."
    exit 1
fi
echo "✅ Compilation check passed"

# Check clippy
echo "3. Checking clippy..."
cargo clippy --all-targets -- -D warnings
if [ $? -ne 0 ]; then
    echo "❌ Clippy found issues."
    exit 1
fi
echo "✅ Clippy check passed"

# Run tests
echo "4. Running tests..."
cargo test
if [ $? -ne 0 ]; then
    echo "❌ Tests failed."
    exit 1
fi
echo "✅ All tests passed"

echo "✨ All pre-commit checks passed!"
exit 0
