.PHONY: build install clean test

# Default target
all: build

# Build the project
build:
	RUSTFLAGS="-lX11" cargo build --release

# Build in debug mode
debug:
	RUSTFLAGS="-lX11" cargo build

# Install to system
install: build
	sudo cp target/release/xremap /usr/local/bin/xremap
	sudo chmod +x /usr/local/bin/xremap

# Install to custom directory
install-local: build
	mkdir -p $(HOME)/bin
	cp target/release/xremap $(HOME)/bin/xremap
	chmod +x $(HOME)/bin/xremap

# Run tests
test:
	RUSTFLAGS="-lX11" cargo test

# Clean build artifacts
clean:
	cargo clean

# Run with example config
run-example: debug
	./target/debug/xremap example_config.yaml

# Check code formatting
fmt:
	cargo fmt

# Run clippy linter
clippy:
	cargo clippy

# Show help
help:
	@echo "Available targets:"
	@echo "  build         - Build release version"
	@echo "  debug         - Build debug version"
	@echo "  install       - Install to /usr/local/bin (requires sudo)"
	@echo "  install-local - Install to ~/bin"
	@echo "  test          - Run tests"
	@echo "  clean         - Clean build artifacts"
	@echo "  run-example   - Run with example config"
	@echo "  fmt           - Format code"
	@echo "  clippy        - Run linter"
	@echo "  help          - Show this help"