.PHONY: all macos linux clean test lint

all: macos

macos:
	cargo build --release

linux:
	cargo build --release

clean:
	cargo clean

test:
	cargo test

lint:
	cargo clippy -- -D warnings
	cargo fmt --check 