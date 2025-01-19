# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.13.5] - 2025-01-19
[0.13.5]: https://github.com/bodo-run/yek/compare/v0.13.4...v0.13.5
### Bug Fixes

- Add aarch64-linux-gnu linker configuration

### Features

- Integrate git-cliff for changelog generation

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

