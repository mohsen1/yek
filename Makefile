.PHONY: all macos linux clean

all: macos

macos:
	cargo build --release

linux:
	cargo build --release

clean:
	cargo clean 