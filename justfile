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

# Build Linux x86_64 binary and deploy to remote host
deploy-linux host path="~/.local/bin":
    cross build --target=x86_64-unknown-linux-gnu --release
    scp -O target/x86_64-unknown-linux-gnu/release/mvx {{host}}:{{path}}/
    scp -O target/x86_64-unknown-linux-gnu/release/cpx {{host}}:{{path}}/
