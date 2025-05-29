.PHONY: all fmt clippy test install

# Default target that runs all checks
all: fmt clippy test

# Format the code
fmt:
	cargo fmt --all -- --check

# Run clippy for linting
clippy:
	cargo clippy -- -W clippy::pedantic

# Run tests
test:
	cargo test

# Install the binary
install:
	cargo install --path .
