# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.16.0] - 2025-01-30
[0.16.0]: https://github.com/bodo-run/yek/compare/v0.15.0...v0.16.0
### Bug Fixes

- Add explicit target installation to build action

### Documentation

- Fix arguments help in README
- Update README.md

### Features

- Print version with --version

### Miscellaneous Tasks

- Ci

### Ci

- Also skip strss in main
- Refactor build action to use explicit inputs
- Remove unused upload_artifacts from build action
- Simplify build action inputs
- Optimize build pipeline
- Consolidate workflows into ci.yml
- Add concurrency control to perf jobs
- Add source-based caching for builds
- Rewrite most of the CI automation
- Use QEMU for ARM binary stress tests
- Do not wait for test to finish in build
- Remove musl arm
- Fix logic on which job to run based on changes

## [0.15.0] - 2025-01-29
[0.15.0]: https://github.com/bodo-run/yek/compare/v0.14.0...v0.15.0
### Bug Fixes

- Add musl toolchain support for Linux builds
- Improve aarch64-musl cross-compilation setup
- Use musl cross-compiler for aarch64-musl builds

### Documentation

- Update readme to reflect recent changes and more

### Miscellaneous Tasks

- Pr feedback
- Make max_git_depth a configuration (read only)
- Safer type casting
- Cargo fmt

### Performance

- Do not go beyond 100 commits reading git history

### Refactor

- Do checksum and serializing in parallel
- Simplify GitHub Actions workflow structure

### Ci

- If crates is already published, skip
- Separate bench + stress testing
- Install openssl for bench workflow too
- Use checkout v4
- Organize ci better to not DRY too much
- Organize build into its own action

### Release

- V0.15.0

## [0.14.0] - 2025-01-29
[0.14.0]: https://github.com/bodo-run/yek/compare/v0.13.8...v0.14.0
### Bug Fixes

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

- Remove unnecessary info log
- Streamline defaults.rs, remove old binary checks
- Update lib and parallel code to rely on FullYekConfig
- Update main.rs to use new config system
- Replace map_or with is_some_and

### Testing

- Remove legacy integration tests, add new e2e config tests

### Bench

- Add bench.toml
- Fix single small file benchmark

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

### Release

- V0.13.9
- V0.14.0

## [0.13.8] - 2025-01-20
[0.13.8]: https://github.com/bodo-run/yek/compare/v0.13.7...v0.13.8
### Bug Fixes

- Ensure files are processed only once and fix priority test
- Use WalkBuilder in streaming mode to respect gitignore
- Include hidden files in WalkBuilder configuration

### Miscellaneous Tasks

- Move big lists to defaults.rs
- Organization
- Fix the release script

### Refactor

- Move size parsing tests to dedicated test file
- Move normalize_path tests to dedicated file
- Improve gitignore handling and fix clippy warnings
- Improve binary file handling and remove duplicate gitignore checks

### Testing

- Add lots of e2e and integration tests
- Add comprehensive gitignore end-to-end tests
- Fix binary file test assertion

### Cargo

- Add git 2

### Ci

- Simpler release script

### E2e

- Fix e2e tests to pass

### Git

- Ignore temp txt files

### Release

- V0.13.8

## [0.13.7] - 2025-01-19
[0.13.7]: https://github.com/bodo-run/yek/compare/v0.13.5...v0.13.7
### Bug Fixes

- Ensure most important chunks are output last when streaming
- Handle Windows paths correctly in gitignore matching
- Make chunk order test platform-independent
- Make chunk order test more robust across platforms
- Improve test robustness and error handling
- Handle logging initialization gracefully in tests
- Manually update Formula version
- Handle Windows paths correctly in gitignore matching
- Update Formula version to match project version
- Get version using cargo pkgid
- Add aarch64-linux-gnu linker configuration
- Use configured max_size in aggregator instead of hardcoded value
- Ensure files with different priorities are in separate chunks
- Normalize Windows paths for priority calculation
- Standardize path handling using PathBuf and normalize_path
- Handle Windows path normalization correctly
- Handle Windows UNC paths correctly
- Normalize paths consistently in gitignore matching and priority sorting
- Add --no-verify flag to cargo publish

### Documentation

- Add roadmap
- Make chunking clear in README
- Fix note formatting
- Update changelog for v0.13.2
- Improve readme
- Add documentation for file_index field

### Features

- Prioritize high-priority files in streaming mode
- Integrate git-cliff for changelog generation

### Miscellaneous Tasks

- Bump version to 0.13.1
- Remove semantic release and sync versions

### Performance

- Improve benchmark configuration and accuracy

### Refactor

- Remove duplicate formula update from release workflow
- Optimize chunk priority check and improve debug logging
- Use consistent chunk size constants

### Testing

- Add test to verify chunk ordering behavior
- Normalize Windows paths in chunk order test

### Ci

- Publish to crates.io
- Improve benchmark job configuration
- Parallelize benchmark groups in serialization tests

### Release

- V0.13.5
- V0.7.5
- V0.13.1
- V0.13.3
- V0.13.4
- V0.13.5
- V0.13.5
- V0.13.6

## [0.13.5] - 2025-01-19
[0.13.5]: https://github.com/bodo-run/yek/compare/v0.13.4...v0.13.5
### Bug Fixes

- Add aarch64-linux-gnu linker configuration

### Features

- Integrate git-cliff for changelog generation

### Release

- V0.13.5

## [0.13.4] - 2025-01-19
[0.13.4]: https://github.com/bodo-run/yek/compare/v0.13.3...v0.13.4
### Bug Fixes

- Lint

### Release

- V0.13.4

## [0.13.3] - 2025-01-19
[0.13.3]: https://github.com/bodo-run/yek/compare/v0.13.2...v0.13.3
### Bug Fixes

- Update Formula version to match project version
- Get version using cargo pkgid

### Documentation

- Update changelog for v0.13.2

### Miscellaneous Tasks

- Remove semantic release and sync versions

### Refactor

- Remove duplicate formula update from release workflow

### Release

- V0.13.3

## [0.13.2] - 2025-01-19
[0.13.2]: https://github.com/bodo-run/yek/compare/v0.13.1...v0.13.2
### Bug Fixes

- Handle Windows paths correctly in gitignore matching
- Handle Windows paths correctly in gitignore matching

## [0.13.1] - 2025-01-19
[0.13.1]: https://github.com/bodo-run/yek/compare/v0.7.5...v0.13.1
### Miscellaneous Tasks

- Bump version to 0.13.1

### Release

- V0.13.1

## [0.7.5] - 2025-01-19
[0.7.5]: https://github.com/bodo-run/yek/compare/v0.13.0...v0.7.5
### Bug Fixes

- Manually update Formula version

### Release

- V0.7.5

## [0.13.0] - 2025-01-19
[0.13.0]: https://github.com/bodo-run/yek/compare/v0.7.4...v0.13.0
### Bug Fixes

- Make tag cleanup cross-platform compatible

## [0.7.4] - 2025-01-19
[0.7.4]: https://github.com/bodo-run/yek/compare/v0.12.5...v0.7.4
### Bug Fixes

- Pr feedback
- Cross-platform SHA256 computation and artifact handling
- Improve version parsing and changelog handling
- Include all files in release commit

### Features

- Implement tag-based release workflow

### Miscellaneous Tasks

- Update Cargo.lock and gitignore

### Refactor

- Improve Makefile cross-platform support and remove redundant target

### Testing

- Use HTML comments for installation script extraction

### Ci

- Remove automatic script path updating

### Release

- V0.7.3
- V0.7.4

## [0.12.4] - 2025-01-19
[0.12.4]: https://github.com/bodo-run/yek/compare/v0.12.3...v0.12.4
### Bug Fixes

- Use GITHUB_TOKEN for authentication in CI workflow

## [0.12.3] - 2025-01-19
[0.12.3]: https://github.com/bodo-run/yek/compare/v0.12.2...v0.12.3
### Bug Fixes

- Update GitHub authentication in CI workflow

## [0.12.2] - 2025-01-19
[0.12.2]: https://github.com/bodo-run/yek/compare/v0.12.1...v0.12.2
### Bug Fixes

- Add PAT token to git push command in CI workflow

## [0.12.1] - 2025-01-19
[0.12.1]: https://github.com/bodo-run/yek/compare/v0.12.0...v0.12.1
### Bug Fixes

- Improve git change detection in CI workflow

## [0.12.0] - 2025-01-19
[0.12.0]: https://github.com/bodo-run/yek/compare/v0.11.0...v0.12.0
### Bug Fixes

- Add aarch64 Linux target configurations
- Install cross-compilation tools for ARM64 Linux targets
- Add linker configuration for ARM64 Linux targets

### Features

- Add linux-musl target support
- Add ARM64 Linux support

## [0.11.0] - 2025-01-19
[0.11.0]: https://github.com/bodo-run/yek/compare/v0.10.0...v0.11.0
### Documentation

- Update README with K suffix example

### Features

- Support K suffix for token count

## [0.10.0] - 2025-01-19
[0.10.0]: https://github.com/bodo-run/yek/compare/v0.9.0...v0.10.0
### Bug Fixes

- Remove default priority list
- Update benchmark comparison parameter to use --noise-threshold
- Output directory handling in non-streaming mode

### Features

- Run serialization in parallel

### Miscellaneous Tasks

- Remove unused import from benchmark

### Performance

- Optimize file processing performance
- Optimize file processing with single-pass reads and smart parallelization

### Ci

- Reduce benchmarking threshold

## [0.9.0] - 2025-01-19
[0.9.0]: https://github.com/bodo-run/yek/compare/v0.8.1...v0.9.0
### Styling

- Use tempfile::tempdir() for performance tests

## [0.8.1] - 2025-01-19
[0.8.1]: https://github.com/bodo-run/yek/compare/v0.8.0...v0.8.1
### Bug Fixes

- Update readme
- Grammatical error

### Features

- Add performance test
- Parallel execution for better perf
- Add benchmark regression test with 5% threshold
- Add comprehensive benchmarks for serialization

### Styling

- Fix linting issues in parallel.rs

### Git

- Undo parallel execution in test branch

## [0.8.0] - 2025-01-19
[0.8.0]: https://github.com/bodo-run/yek/compare/v0.7.0...v0.8.0
### Bug Fixes

- Improve installation test workflow
- Improve installation test error handling and diagnostics
- Fix YAML linting in installation test workflow
- Improve Windows installation test
- Add macOS support to installation test
- Remove ARM64 Windows target and cleanup CI workflow
- Improve file chunking and debug output
- Correct file priority sorting order
- Ensure higher priority files come last in output
- Ensure consistent priority boost across platforms
- Ensure consistent commit timestamps in tests
- Ensure consistent commit timestamps in tests
- Ensure consistent path handling in tests
- Skip test_git_priority_boost_with_path_prefix in windows
- Skip test_git_priority_boost
- Remove redundant ignore pattern check
- Normalize path separators on Windows for consistent pattern matching
- Type inference issue in HashMap::get
- Normalize path separators for gitignore matching on Windows
- Normalize path separators for custom ignore patterns on Windows
- Normalize path separators in output on Windows

### Features

- Add installer script and update README with installation instructions
- Add Windows installer script and update README
- Add semantic release and GitHub Pages deployment
- Add installation testing to CI workflow
- Add build optimizations and improve CI caching
- Add installation test workflow and update README markers
- Update installation URLs to use bodo.run

### Miscellaneous Tasks

- Remove unused import

### Refactor

- Remove unused functions and imports

### Styling

- Remove unnecessary mut declarations

### Testing

- Add installer tests

## [0.6.0] - 2025-01-15
[0.6.0]: https://github.com/bodo-run/yek/compare/v0.5.0...v0.6.0
### Features

- Remove --stream flag in favor of automatic pipe detection
- Add user-friendly size input format

### Miscellaneous Tasks

- Prepare for v0.6.0

## [0.5.0] - 2025-01-15
[0.5.0]: https://github.com/bodo-run/yek/compare/v0.4.0...v0.5.0
### Documentation

- Update README to match actual CLI implementation

## [0.4.0] - 2025-01-13
[0.4.0]: https://github.com/bodo-run/yek/compare/v0.3.0...v0.4.0
### Features

- Add git-based priority boost for recently changed files

## [0.3.0] - 2025-01-13
[0.3.0]: https://github.com/bodo-run/yek/compare/v0.2.0...v0.3.0
### Bug Fixes

- Improve file processing and error handling
- Swap -s and -x flags for stream and max-size options

### Features

- Add homebrew formula and release automation

### Styling

- Fix formatting and linting issues

## [0.2.0] - 2025-01-13
[0.2.0]: https://github.com/bodo-run/yek/compare/v0.1.0...v0.2.0
### Bug Fixes

- Fix token counting flag handling
- Fix token counting flag handling
- Fix clippy warnings
- Resolve -d flag conflict and clarify stream flag behavior - Change delay flag from -d to -w to avoid conflict with debug flag - Update stream flag help text to clarify it disables output directory

### Features

- Add debug logging and fix warnings - Add --debug flag and tracing, add detailed debug logs throughout code, clean up build warnings
- Update default ignore patterns
- Add configurable output directory - Add output_dir field to LlmSerializeConfig - Add -o/--output-dir CLI flag - Support output directory configuration in yek.toml - Implement output directory override logic with CLI precedence

### Miscellaneous Tasks

- Bump version to 0.2.0

### Refactor

- Fix clippy warning about redundant closure

### Styling

- Apply cargo fmt suggestions

