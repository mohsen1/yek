# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.25.0] - 2025-09-25
[0.25.0]: https://github.com/bodo-run/yek/compare/v0.24.0...v0.25.0

### Features

- Add `yek --update` command for self-updating functionality

### Miscellaneous Tasks

- Bump version to 0.25.0

## [0.24.0] - 2025-09-25

## [0.23.0] - 2025-09-17
[0.23.0]: https://github.com/bodo-run/yek/compare/v0.21.0...v0.23.0

### Bug Fixes

- Fix `Windows` installation script
- Improve token parsing to handle multi-byte characters and emojis

### Features

- Add --output-name option to specify output filename

### Miscellaneous Tasks

- Bump serde from 1.0.217 to 1.0.218
- Bump serde_json from 1.0.138 to 1.0.139
- Bump bytesize from 1.3.2 to 2.0.0
- Bump anyhow from 1.0.95 to 1.0.96
- Bump clap from 4.5.30 to 4.5.31
- Bump serde from 1.0.218 to 1.0.219
- Bump config from 0.15.8 to 0.15.11
- Bump anyhow from 1.0.96 to 1.0.97
- Bump chrono from 0.4.39 to 0.4.40
- Bump serde_json from 1.0.139 to 1.0.140
- Bump bytesize from 2.0.0 to 2.0.1
- Bump grcov from 0.8.20 to 0.8.24
- Bump time from 0.3.37 to 0.3.41
- Bump tempfile from 3.17.1 to 3.19.1

### Ci

- Add version-based release check

## [0.21.0] - 2025-02-23
[0.21.0]: https://github.com/bodo-run/yek/compare/v0.20.0...v0.21.0

### Bug Fixes

- Glob pattern handling in e2e tests

### Documentation

- Update README with glob pattern and file selection support

### Features

- Handle glob patterns in input paths

### Testing

- Add comprehensive tests for glob pattern support

### Ci

- Run release and publish jobs on main branch
- Only run release and publish on tag pushes
- Trigger release on tag merge to main

## [0.20.0] - 2025-02-22

## [0.19.0] - 2025-02-19

## [0.18.0] - 2025-02-14

## [0.17.0] - 2025-02-06

## [0.16.0] - 2025-01-30

## [0.15.0] - 2025-01-29

## [0.14.0] - 2025-01-29

### Bug Fixes

- Ensure files are processed only once and fix priority test
- Clean up imports and remove duplicates
- Add #[allow(dead_code)] to is_effectively_absolute
- Add musl-tools installation for MUSL targets
- Remove leading slash from Windows drive path in test
- Add OpenSSL setup for macOS builds
- Add OpenSSL setup for MUSL builds
- Use muslrust container for MUSL builds
- Add OpenSSL static build for MUSL targets
- Add output directory for benchmarks
- Improve OpenSSL configuration for macOS builds
- Remove unsupported --output-dir flag from benchmark commands
- Add OpenSSL setup for all Linux targets
- Add YEK_OUTPUT_DIR env var for benchmarks
- Add output directory config to benchmarks
- Update OpenSSL setup for MUSL builds
- Resolve dead code warnings and MUSL cross-compilation issues
- Update benchmark groups in CI to match actual benchmark definitions
- Correct TOML format in yek.toml
- Add required pattern field to yek.toml
- Correct priority_rules format in yek.toml
- Ensure output_dir takes precedence and properly sets stream flag
- Add git config in tests

### Documentation

- Update README to reflect YAML config usage
- Fix yek.yaml example

### Features

- Add multi-arch support and fix OpenSSL issues
- Print output directory path when not streaming
- Improve output messages and logging
- Introduce config.rs with ClapConfigFile integration
- Add priority.rs for advanced file scoring

### Miscellaneous Tasks

- Move big lists to defaults.rs
- Organization
- Delete dead code
- Bump git-cliff from 1.4.0 to 2.3.0
- Bump clap from 4.5.26 to 4.5.27
- Bump byte-unit from 4.0.19 to 5.1.6
- Clean up git leftovers from the tokenizer branch
- PR review
- Update .gitignore and add VSCode launch config
- Revamp Cargo deps (config-file support, JSON, YAML, etc.)
- Add sample yek.yaml config
- Fix clippy issues in parallel.rs
- Add majo/minor to make release

### Performance

- Add new serialization bench with FullYekConfig

### Refactor

- Move size parsing tests to dedicated test file
- Move normalize_path tests to dedicated file
- Remove unnecessary info log
- Streamline defaults.rs, remove old binary checks
- Update lib and parallel code to rely on FullYekConfig
- Update main.rs to use new config system
- Replace map_or with is_some_and

### Testing

- Add lots of e2e and integration tests
- Remove legacy integration tests, add new e2e config tests

### Bench

- Add bench.toml
- Fix single small file benchmark

### Cargo

- Add git 2

### Ci

- Force release for now
- Improve release action
- Reuse build from ci.yaml in release
- Attempt #2, fix release action
- Add fail-fast: false to build job
- Allow manual invocation of release
- Merge build and release actions
- Add rustup target add before building for each target
- Add bench back
- Use cross to build in CI
- Fix build
- Introduce the AI loop
- Fix AI Loop
- Bring new changes from tokenizer work to main (ai loop)
- Improve AI loop
- Install yek in ai loop
- Fix release
- Add unique names to artifacts
- Add unique names to artifacts

### E2e

- Fix e2e tests to pass

### Git

- Ignore temp txt files

### Release

- V0.13.9
- V0.14.0

## [0.13.9] - 2025-01-29

## [0.13.8] - 2025-01-20

**Full Changelog**: https://github.com/bodo-run/yek/compare/v0.13.6...v0.13.8

### What's Changed
* ci: simpler release script by @mohsen1 in https://github.com/bodo-run/yek/pull/22
* Use gitignore in streaming mode as well by @mohsen1 in https://github.com/bodo-run/yek/pull/28

## [0.13.6] - 2025-01-19

**Full Changelog**: https://github.com/bodo-run/yek/compare/v0.13.5...v0.13.6

### What's Changed
* Fix: broken link to repomix repo by @bbrewington in https://github.com/bodo-run/yek/pull/21
* Fix chunk priority in stream mode by @mohsen1 in https://github.com/bodo-run/yek/pull/16

### New Contributors
* @bbrewington made their first contribution in https://github.com/bodo-run/yek/pull/21

## [0.13.5] - 2025-01-19

**Full Changelog**: https://github.com/bodo-run/yek/compare/v0.13.4...v0.13.5

## [0.13.4] - 2025-01-19

**Full Changelog**: https://github.com/bodo-run/yek/compare/v0.13.3...v0.13.4

## [0.13.3] - 2025-01-19

### Bug Fixes

* update Formula version to match project version ([54142a7](https://github.com/bodo-run/yek/commit/54142a797642e8ce3eab8ed9b971ea70cf416f64))

## [0.13.2] - 2025-01-19

### Bug Fixes

* handle Windows paths correctly in gitignore matching ([78a384c](https://github.com/bodo-run/yek/commit/78a384c0a1ace4e24adabcc3e2a79a8804f6b9d5))
* handle Windows paths correctly in gitignore matching ([b21d45a](https://github.com/bodo-run/yek/commit/b21d45a006ff66279dac30e9319771d69696ae5b))

## [0.13.0] - 2025-01-19

### Bug Fixes

* cross-platform SHA256 computation and artifact handling ([c729f28](https://github.com/bodo-run/yek/commit/c729f28c18266aca021cc26b001f257a1526ecab))
* improve version parsing and changelog handling ([375c4e2](https://github.com/bodo-run/yek/commit/375c4e2976dfc49b0dc0f74d8da36a53907214e9))
* include all files in release commit ([e2ce188](https://github.com/bodo-run/yek/commit/e2ce188b3534713633831f3321082571d04b321b))
* make tag cleanup cross-platform compatible ([57bdcaf](https://github.com/bodo-run/yek/commit/57bdcaf3618b73488d938c9164dc065b91f2c291))
* pr feedback ([f1f43f9](https://github.com/bodo-run/yek/commit/f1f43f928e15a171e0a30a19b3b95ffdf9e5e7ee))

### Features

* implement tag-based release workflow ([1c0d386](https://github.com/bodo-run/yek/commit/1c0d3867002ae4a98ba51ee7e76ab05800684453))

## [0.12.5] - 2025-01-19

### Reverts

* Revert "fix: use GITHUB_TOKEN for authentication in CI workflow" ([2c8e28a](https://github.com/bodo-run/yek/commit/2c8e28a19ae4327291c8cd46b9fbea887b520b0c))

## [0.12.4] - 2025-01-19

### Bug Fixes

* use GITHUB_TOKEN for authentication in CI workflow ([5f6dca2](https://github.com/bodo-run/yek/commit/5f6dca28e3dc8313f3f0f56de29a16a8f619593e))