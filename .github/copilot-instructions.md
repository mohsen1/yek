# yek - Fast Rust Repository Serializer

yek is a high-performance Rust CLI tool that serializes text-based files in a repository or directory for LLM consumption. It uses Git history, `.gitignore` rules, and configurable priority rules to intelligently process and prioritize files.

**ALWAYS reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.**

## Working Effectively

### Bootstrap and Build
- **NEVER CANCEL BUILDS OR LONG-RUNNING COMMANDS** - All builds may take 3+ minutes
- Initial dev build: `cargo build` -- takes ~3 minutes. NEVER CANCEL. Set timeout to 300+ seconds.
- Release build: `cargo build --release` -- takes ~3 minutes. NEVER CANCEL. Set timeout to 300+ seconds.
- Quick incremental builds: typically take under 30 seconds after initial build

### Testing
- **NEVER CANCEL TEST RUNS** - Full test suite takes ~1-2 minutes
- Run all tests: `cargo test` -- takes ~1-2 minutes. NEVER CANCEL. Set timeout to 180+ seconds.
- Alternative: `make test` (same as cargo test)
- Tests cover: configuration, e2e scenarios, integration, unit tests, and parallel processing

### Linting and Formatting
- Lint check: `cargo clippy -- -D warnings` -- takes ~2 minutes. NEVER CANCEL. Set timeout to 180+ seconds.
- Format check: `cargo fmt --check` -- takes ~1 second
- Alternative: `make lint` (runs both clippy and fmt check)
- **ALWAYS run `make lint` before committing** or CI will fail

### Running the Application
- Build first: `cargo build --release`
- Basic usage: `./target/release/yek .` (processes current directory)
- With output directory: `./target/release/yek . --output-dir /tmp/output`
- Streaming output: `./target/release/yek . | head -20` (pipes to stdout)
- Help: `./target/release/yek --help`
- Version: `./target/release/yek --version`

## Validation

### Manual Testing Requirements
- **ALWAYS test end-to-end functionality after making changes**
- Create a test directory with multiple file types: `.md`, `.rs`, `.txt`
- Run yek on the test directory and verify output contains expected files
- Test with both output directory and streaming modes
- Verify Git integration works by creating a git repo and checking file prioritization

### Example Validation Scenario
```bash
# Create test scenario
cd /tmp && rm -rf test_yek && mkdir test_yek && cd test_yek
git init
echo "# Test Project" > README.md
mkdir src && echo 'fn main() { println!("Hello!"); }' > src/main.rs
echo "test content" > src/utils.rs
echo "target/" > .gitignore
git add . && git config user.email "test@example.com" && git config user.name "Test User"
git commit -m "Initial commit"

# Test yek functionality (should create output in /tmp/yek-output/)
/home/runner/work/yek/yek/target/release/yek --max-size 1KB
# Expected: prints path like "/tmp/yek-output/yek-output-XXXXXXXX.txt"

# Test streaming (should show ">>>> filename" format)
/home/runner/work/yek/yek/target/release/yek . | head -10
# Expected output:
# >>>> README.md
# # Test Project
# 
# >>>> src/main.rs
# fn main() { println!("Hello!"); }

# Test glob patterns
/home/runner/work/yek/yek/target/release/yek "src/**/*.rs" | head -5
# Expected: only shows .rs files from src directory

# Test JSON mode
/home/runner/work/yek/yek/target/release/yek --json . | head -10
# Expected: JSON array with filename/content objects
```

### CI Requirements
- All CI steps are defined in `.github/workflows/ci.yml`
- CI includes: lint, test, build for multiple platforms, stress tests, benchmarks
- **ALWAYS ensure your changes pass local lint and test before pushing**

## Project Structure

### Key Directories and Files
```
/home/runner/work/yek/yek/           # Repository root
├── src/                             # Rust source code
│   ├── main.rs                      # CLI entry point
│   ├── lib.rs                       # Library interface
│   ├── config.rs                    # Configuration handling
│   ├── parallel.rs                  # Parallel processing
│   ├── priority.rs                  # File priority logic
│   └── defaults.rs                  # Default values
├── tests/                           # Comprehensive test suite
├── .github/workflows/ci.yml         # CI/CD pipeline
├── Cargo.toml                       # Rust project configuration
├── Makefile                         # Build shortcuts
├── yek.yaml                         # Default configuration
└── scripts/                         # Installation and release scripts
```

### Configuration
- yek supports `yek.yaml`, `yek.toml`, or `yek.json` configuration files
- Configuration includes: ignore patterns, priority rules, binary extensions, Git boost settings
- Example config file is at project root: `yek.yaml`

## Common Tasks

### Development Workflow
```bash
# 1. Make changes to source code
# 2. Build and test iteratively
cargo build                          # ~3 minutes first time, ~30s incremental
cargo test                           # ~1-2 minutes
cargo clippy -- -D warnings          # ~2 minutes
cargo fmt --check                    # ~1 second

# 3. Test functionality manually
cargo build --release
./target/release/yek --help
./target/release/yek /tmp/test_scenario

# 4. Final validation before commit
make lint                            # Runs clippy + fmt
make test                            # Runs full test suite
```

### Release Process
- Version management: Edit `Cargo.toml` version field
- Release script: `scripts/make-release.sh [patch|minor|major]`
- CI handles building cross-platform binaries and publishing to crates.io

### Troubleshooting
- **Build failures**: Check Rust version (requires recent stable), ensure OpenSSL dev libraries installed
- **Test failures**: Most tests create temporary directories and files, ensure /tmp is writable
- **Performance issues**: yek is optimized for speed, typical repos process in seconds
- **Git integration**: Some features require Git repository, ensure `.git` directory exists
- **"Broken pipe" errors**: Normal when piping output (e.g., `yek . | head -10`)
- **Empty output**: Check if files are being ignored by .gitignore or default ignore patterns
- **Token counting errors**: Ensure valid token limit format (e.g., "128k", "1000")

### Known Working Configurations
- **Ubuntu/Linux**: All functionality works, including Git integration
- **Rust version**: Works with Rust 1.89+ (current CI uses stable)
- **Git repositories**: Full Git integration including priority boosting based on commit history
- **File types**: Supports all text-based files, automatically detects and skips binary files
- **Configuration**: All three formats work: `yek.yaml`, `yek.toml`, `yek.json`

## Key Command Reference

### Build Commands (with measured timing)
```bash
# First build (cold cache)
cargo build                          # ~167 seconds
cargo build --release               # ~161 seconds

# Incremental builds
cargo build                          # ~5-30 seconds
cargo build --release               # ~5-30 seconds
```

### Test Commands (with measured timing)
```bash
cargo test                           # ~65 seconds
make test                            # Same as cargo test
```

### Lint Commands (with measured timing)
```bash
cargo clippy -- -D warnings         # ~98 seconds
cargo fmt --check                   # ~1 second
make lint                            # Runs both (total ~99 seconds)
```

### Functional Commands
```bash
# Basic usage patterns
./target/release/yek .               # Process current directory to temp file
./target/release/yek src/            # Process specific directory
./target/release/yek "src/**/*.rs"   # Use glob patterns (ALWAYS quote them!)
./target/release/yek . | head -20    # Stream output to stdout
./target/release/yek --tokens 128k   # Use token-based size limits
./target/release/yek --json          # JSON output format
./target/release/yek --debug         # Debug output
./target/release/yek --output-dir /tmp/output .  # Specify output directory

# Configuration options
./target/release/yek --config-file custom.yaml
./target/release/yek --ignore-patterns "*.tmp" "build/**"
./target/release/yek --unignore-patterns "!important.tmp"
./target/release/yek --max-size 10MB
./target/release/yek --no-config     # Skip config file loading

# Advanced usage
./target/release/yek file1.txt file2.txt        # Process specific files
./target/release/yek src/ tests/                # Process multiple directories
./target/release/yek --output-template "=== {{{FILE_PATH}}} ===\\nFILE_CONTENT"
```

## CRITICAL REMINDERS

### Timeout and Cancellation Rules
- **NEVER CANCEL builds, tests, or long-running commands**
- Initial builds: 300+ second timeout
- Tests: 180+ second timeout  
- Linting: 180+ second timeout
- If a command appears to hang, wait at least 3 minutes before considering alternatives

### Validation Requirements
- **ALWAYS manually test your changes** with real scenarios
- **ALWAYS run complete end-to-end validation** after making changes
- **ALWAYS run `make lint` before committing** - CI will fail without it
- **ALWAYS test both streaming and file output modes**

### Performance Expectations
- yek is designed to be fast - most repositories process in under 10 seconds
- Large repositories (like VSCode) should process in under 1 minute
- If processing takes longer, investigate for infinite loops or performance regressions
