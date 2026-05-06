.PHONY: setup clean fmt-check fmt clippy check build test adr-check ci help

# Setup development environment
setup:
	rustup component add rustfmt clippy
	cargo fetch
	lefthook install

# Cleanup compilation outputs
clean:
	cargo clean

# Check the code format
fmt-check:
	cargo fmt --all -- --check

# Format the code
fmt:
	cargo fmt --all

# Run rust clippy
clippy:
	cargo clippy --all-targets --all-features -- -D warnings

# Check code
check:
	cargo check --all-targets

# Build the binary
build:
	cargo build

# Run all tests
test:
	cargo test --all

# Validate ADRs with arkouda
adr-check:
	cargo run -- check

# Run all CI checks
ci: fmt-check clippy test build adr-check

# Show help
help:
	@echo ''
	@echo 'Usage:'
	@echo ' make [target]'
	@echo ''
	@echo 'Targets:'
	@awk '/^[a-zA-Z\-\_0-9]+:/ { \
	helpMessage = match(lastLine, /^# (.*)/); \
		if (helpMessage) { \
			helpCommand = substr($$1, 0, index($$1, ":")); \
			helpMessage = substr(lastLine, RSTART + 2, RLENGTH); \
			printf "\033[36m%-30s\033[0m %s\n", helpCommand,helpMessage; \
		} \
	} \
	{ lastLine = $$0 }' $(MAKEFILE_LIST)

.DEFAULT_GOAL := help
