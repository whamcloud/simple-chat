<!-- below is the git-precommit hook -->

```bash
#!/bin/bash
set -e

echo "Running Pre-commit Checks..."

# 1. Formatting
# We use --check to fail if files are unformatted
if ! cargo fmt --all -- --check; then
    echo "❌ Cargo Fmt failed. Run 'cargo fmt' to fix."
    exit 1
fi

# 2. Clippy (Linting)
# We deny warnings to ensure clean code
if ! cargo clippy --workspace --all-targets --all-features -- -D warnings; then
    echo "❌ Clippy failed. Fix warnings before committing."
    exit 1
fi

# 3. Tests
if ! cargo test --workspace; then
    echo "❌ Tests failed."
    exit 1
fi

echo "✅ All checks passed."
```
