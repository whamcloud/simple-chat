#!/bin/bash

# Setup script to install pre-commit hook

if [ ! -d ".git" ]; then
    echo "Error: This script must be run from the repository root"
    exit 1
fi

if [ ! -f "hooks/pre-commit" ]; then
    echo "Error: hooks/pre-commit not found"
    exit 1
fi

cp hooks/pre-commit .git/hooks/pre-commit
# Convert line endings to Unix format (LF) if needed
sed -i 's/\r$//' .git/hooks/pre-commit 2>/dev/null || sed -i '' 's/\r$//' .git/hooks/pre-commit 2>/dev/null || true
chmod +x .git/hooks/pre-commit

echo "Pre-commit hook installed successfully!"

