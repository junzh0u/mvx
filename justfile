# Run all checks
check: fmt clippy test

# Check formatting
fmt:
    cargo fmt --all -- --check

# Run clippy for linting
clippy:
    cargo clippy -- -W clippy::pedantic -D warnings

# Run tests
test:
    cargo test

# Auto-fix formatting and clippy warnings
fix:
    cargo fmt --all
    cargo clippy --fix --allow-dirty -- -W clippy::pedantic -D warnings

# Install the binary
install:
    cargo install --path .
