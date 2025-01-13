.PHONY: all macos linux clean test lint update-formula build-release

FORMULA_PATH := $(shell pwd)/Formula/yek.rb

all: macos

macos:
	cargo build --release

linux:
	cargo build --release

build-release: ## Build release artifacts for all platforms
	@echo "Building release v$(VERSION)..."
	@mkdir -p dist
	# Build for macOS ARM64
	cargo build --release --target aarch64-apple-darwin
	cd target/aarch64-apple-darwin/release && tar czf ../../../dist/yek-aarch64-apple-darwin.tar.gz yek
	# Build for macOS x86_64
	cargo build --release --target x86_64-apple-darwin
	cd target/x86_64-apple-darwin/release && tar czf ../../../dist/yek-x86_64-apple-darwin.tar.gz yek
	# Build for Linux x86_64
	cargo build --release --target x86_64-unknown-linux-gnu
	cd target/x86_64-unknown-linux-gnu/release && tar czf ../../../dist/yek-x86_64-unknown-linux-gnu.tar.gz yek
	@echo "Release artifacts built in dist/"

clean:
	cargo clean
	rm -rf dist

test:
	cargo test

lint:
	cargo clippy -- -D warnings
	cargo fmt --check 

update-formula: ## Update Homebrew formula with new SHA values
	@echo "Updating formula with SHA values for v$(VERSION)..."
	@mkdir -p /tmp/yek-release
	@cd /tmp/yek-release && \
		curl -sLO https://github.com/mohsen1/yek/releases/download/v$(VERSION)/yek-aarch64-apple-darwin.tar.gz && \
		curl -sLO https://github.com/mohsen1/yek/releases/download/v$(VERSION)/yek-x86_64-apple-darwin.tar.gz && \
		curl -sLO https://github.com/mohsen1/yek/releases/download/v$(VERSION)/yek-x86_64-unknown-linux-gnu.tar.gz
	@echo "Calculating SHA256 hashes..."
	@cd /tmp/yek-release && \
		ARM64_HASH=$$(shasum -a 256 yek-aarch64-apple-darwin.tar.gz | cut -d ' ' -f 1) && \
		X86_64_HASH=$$(shasum -a 256 yek-x86_64-apple-darwin.tar.gz | cut -d ' ' -f 1) && \
		LINUX_HASH=$$(shasum -a 256 yek-x86_64-unknown-linux-gnu.tar.gz | cut -d ' ' -f 1) && \
		sed -i '' \
			-e 's/version ".*"/version "$(VERSION)"/' \
			-e "s/sha256 \".*\"  # arm64/sha256 \"$$ARM64_HASH\"  # arm64/" \
			-e "s/sha256 \".*\"  # x86_64/sha256 \"$$X86_64_HASH\"  # x86_64/" \
			-e "s/sha256 \".*\"  # linux/sha256 \"$$LINUX_HASH\"  # linux/" \
			$(FORMULA_PATH)
	@rm -rf /tmp/yek-release
	@echo "Formula updated successfully!" 