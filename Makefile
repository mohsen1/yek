.PHONY: all macos linux clean test lint release major build-artifacts

CURRENT_PLATFORM := $(shell rustc -vV | grep host: | cut -d' ' -f2)

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

build-artifacts:
	@echo "Building for $(CURRENT_PLATFORM)..."
	cargo build --release
	mkdir -p "yek-$(CURRENT_PLATFORM)"
	if [ "$(OS)" = "Windows_NT" ]; then \
		cp "target/release/yek.exe" "yek-$(CURRENT_PLATFORM)/"; \
	else \
		cp "target/release/yek" "yek-$(CURRENT_PLATFORM)/"; \
	fi
	tar -czf "yek-$(CURRENT_PLATFORM).tar.gz" "yek-$(CURRENT_PLATFORM)"
	rm -rf "yek-$(CURRENT_PLATFORM)"

release: test lint
	@scripts/make-release.sh $(if $(filter major,$(MAKECMDGOALS)),major,$(if $(filter minor,$(MAKECMDGOALS)),minor,patch))

.PHONY: major minor
major: ;
minor: ;


 