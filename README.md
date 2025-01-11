# `yek`

A simple tool to read text-based files in a repository or directory, chunk them, and serialize them for LLM consumption. By default, the tool:
- Uses `.gitignore` rules to skip unwanted files.
- Infers additional ignore patterns (binary, large, etc.).
- Splits content into chunks based on either approximate "token" count or byte size.

## Installation

1. [Install Rust](https://www.rust-lang.org/tools/install).
2. Clone this repository.
3. Run `make macos` or `make linux` to build for your platform (both run `cargo build --release`).

## Usage

### Add to your PATH

```bash
export PATH=$(pwd)/target/release:$PATH
```

### Run
```bash
yek --help

yek 0.1.0
Serialize a repo or subdirectory's text files into chunked text with optional token counting.

Usage: yek [OPTIONS]

Options:
  -t, --tokens <MAX_SIZE>        Maximum tokens/bytes per chunk (defaults to Infinity if omitted or 0)
  -p, --path <PATH>              Base path to serialize (optional)
  -m, --model <MODEL>            Model name, not actually used for real token counting, but accepted for parity
  -c, --count-tokens             Count tokens in a naive way rather than bytes
  -s, --stream                   Stream output to stdout instead of writing to files
      --config-file <CONFIGFILE> Path to optional yek.toml config file
  -h, --help                     Print help
  -V, --version                  Print version
```

## Examples
- Serialize entire repository into a single file (infinite chunk size)
```bash
yek
```

- Split repository into chunks of ~128KB:
```bash
yek -t 128000
```

- Split into chunks of ~128k tokens (naive)
```bash
yek -t 128000 -c
```

- Serialize only the src/app directory
```bash
yek -p src/app
```

- Stream output to stdout instead of writing files
```bash
yek -s
```

## Configuration File

You can place a file called `yek.toml` at your project root or pass a custom path via `--config-file`. The configuration file allows you to:

1. Add custom ignore patterns
2. Define file priority rules for processing order
3. Add additional binary file extensions to ignore (extends the built-in list)

Example configuration:

```toml
# Add patterns to ignore (in addition to .gitignore)
[ignore_patterns]
patterns = [
  "node_modules/",
  "\\.next/",
  "my_custom_folder/"
]

# Define priority rules for processing order
# Higher scores are processed first
[[priority_rules]]
score = 100
patterns = ["^src/lib/"]

[[priority_rules]]
score = 90
patterns = ["^src/"]

[[priority_rules]]
score = 80
patterns = ["^docs/"]

# Add additional binary file extensions to ignore
# These extend the built-in list (.jpg, .png, .exe, etc.)
binary_extensions = [
  ".blend",  # Blender files
  ".fbx",    # 3D model files
  ".max",    # 3ds Max files
  ".psd",    # Photoshop files
]
```

All configuration keys are optional. By default:
- No extra ignore patterns
- All files have equal priority (score: 1)
- Common binary file extensions are ignored (.jpg, .png, .exe, etc. - see source for full list) 