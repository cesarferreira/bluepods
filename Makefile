.PHONY: build build-release release ensure-cargo-release install clean test check fmt lint run all

LEVEL ?= minor

# Default target
all: check build test

# Build debug version
build:
	cargo build

# Build release version
build-release:
	cargo build --release

# Publish a new release (usage: make release or make release LEVEL=patch)
release: ensure-cargo-release
	cargo release $(LEVEL) --execute --no-confirm

# Ensure cargo-release (the `cargo release` subcommand that drives version bump,
# tag, and push) is available, so releasing does not require a manual install.
ensure-cargo-release:
	@command -v cargo-release >/dev/null 2>&1 || { \
		echo "cargo-release not found; installing via cargo..."; \
		cargo install cargo-release --locked; \
	}

# Install to ~/.cargo/bin
install:
	cargo install --path . --locked

# Clean build artifacts
clean:
	cargo clean

# Run all tests
test:
	cargo test

# Run clippy and check
check:
	cargo check --all-targets
	cargo clippy --all-targets

# Format code
fmt:
	cargo fmt

# Lint (check formatting)
lint:
	cargo fmt -- --check
	cargo clippy --all-targets

# Run with arguments (usage: make run ARGS="list")
run:
	cargo run -- $(ARGS)
