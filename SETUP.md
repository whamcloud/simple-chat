# Development Setup

This guide covers the tools needed to develop and contribute to this
project.

## Required Tools

### Rust Toolchain

Install Rust using rustup (the official Rust installer):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, ensure Rust is in your PATH:

```bash
source $HOME/.cargo/env
```

Verify installation:

```bash
rustc --version
cargo --version
```

### Rust Components

Install the required Rust components for linting and formatting:

```bash
rustup component add rustfmt clippy
```

### Pre-commit

Pre-commit is used to run automated checks before commits. Install it using
one of these methods:

#### Option 1: Using pipx (recommended)

```bash
# Install pipx if not already installed
sudo apt install pipx
pipx ensurepath

# Install pre-commit
pipx install pre-commit
```

#### Option 2: Using pip in a virtual environment

```bash
# Create a virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install pre-commit
pip install pre-commit
```

#### Option 3: System-wide (Debian/Ubuntu)

```bash
sudo apt install pre-commit
```

After installing pre-commit, initialize it in the repository:

```bash
pre-commit install
```

## Quick Start

Once all tools are installed:

1. **Build the project:**

   ```bash
   cargo build
   ```

2. **Run tests:**

   ```bash
   cargo test
   ```

3. **Format code:**

   ```bash
   cargo fmt
   ```

4. **Run linter:**

   ```bash
   cargo clippy -- -D warnings
   ```

5. **Run all pre-commit hooks manually:**

   ```bash
   pre-commit run --all-files
   ```

## Development Workflow

With pre-commit installed:

- On every commit: code formatting, clippy checks, and build verification run automatically
- On every push: tests run automatically
- Markdown files are automatically linted and fixed

If any check fails, the commit/push will be blocked until issues are resolved.
