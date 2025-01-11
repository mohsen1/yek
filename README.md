# yek

A simple tool to read text-based files in a repository or directory, chunk them, and serialize them for LLM consumption. By default, the tool:
- Uses `.gitignore` rules to skip unwanted files.
- Infers additional ignore patterns (binary, large, etc.).
- Prioritizes files by patterns (like `src/lib`, `prisma/schema.prisma`, etc.).
- Splits content into chunks based on either approximate "token" count or byte size.

## Installation

1. [Install Rust](https://www.rust-lang.org/tools/install).
2. Clone this repository.
3. Run `make macos` or `make linux` to build for your platform (both run `cargo build --release`).

## Usage

```bash
./target/release/yek --help

yek 0.1.0
Serialize a repo or subdirectory's text files into chunked text with optional token counting.

Usage: yek [OPTIONS]

Options:
  -t, --tokens <MAX_SIZE>        Maximum tokens/bytes per chunk (defaults to Infinity if omitted or 0)
  -p, --path <PATH>              Base path to serialize (optional)
  -m, --model <MODEL>            Model name, not actually used for real token counting, but accepted for parity
  -c, --count-tokens             Count tokens in a naive way rather than bytes
  -s, --stream                   Stream output to stdout instead of writing to files
      --config-file <CONFIGFILE> Path to optional llm-serialize config TOML
  -h, --help                     Print help
  -V, --version                  Print version
```

## Examples
- Serialize entire repository into a single file (infinite chunk size)
```bash
./target/release/yek
```

- Split repository into chunks of ~128KB:
```bash
./target/release/yek -t 128000
```

- Split into chunks of ~128k tokens (naive)
```bash
./target/release/yek -t 128000 -c
```

- Serialize only the src/app directory
```bash
./target/release/yek -p src/app
```

- Stream output to stdout instead of writing files
```bash
./target/release/yek -s
```

## Optional Configuration File

You can place a file called `llm-serialize.toml` at your project root or pass a custom path via `--config-file`. The tool will parse and apply custom rules to override or extend priorities and ignore patterns. Example:

```toml
[ignore_patterns]
patterns = [
  "node_modules/",
  "\\.next/",
  "my_custom_folder/"
]

[[priority_rules]]
score = 101
patterns = ["^my_special_folder/"]
```

All keys are optional. The `ignore_patterns.patterns` array adds to the global ignore. The `priority_rules` array adds or overrides existing priority rules. 