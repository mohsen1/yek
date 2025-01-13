.PHONY: all macos linux clean test lint update-formula build-release

FORMULA_PATH := $(shell pwd)/Formula/yek.rb

all: macos

macos:
	cargo build --release

linux:
	cargo build --release

clean:
	cargo clean
	rm -rf dist

test:
	cargo test

lint:
	cargo clippy -- -D warnings
	cargo fmt --check 
