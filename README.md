# `yek`

A simple tool to read text-based files in a repository or directory, chunk them, and serialize them for LLM consumption. By default, the tool:

- Uses `.gitignore` rules to skip unwanted files.
- Infers additional ignore patterns (binary, large, etc.).
- Splits content into chunks based on either approximate "token" count or byte size.
- Automatically detects if output is being piped and streams content instead of writing to files.
- Supports processing multiple directories in a single command.

## Installation

### Via Homebrew (recommended)

```bash
brew tap mohsen1/yek https://github.com/mohsen1/yek.git
brew install yek
```

### From Source

1. [Install Rust](https://www.rust-lang.org/tools/install).
2. Clone this repository.
3. Run `make macos` or `make linux` to build for your platform (both run `cargo build --release`).
4. Add to your PATH:

```bash
export PATH=$(pwd)/target/release:$PATH
```

## Usage

`yek` has sensible defaults, you can simply run `yek` in a directory to serialize the entire repository. It will serialize all files in the repository into chunks of 10MB by default. The file will be written to the temp directory and file path will be printed to the console.

### Examples

Process current directory:

```bash
yek
```

Process specific directories:

```bash
yek src/ tests/
```

Process multiple repositories:

```bash
yek ~/code/project1 ~/code/project2
```

Pipe output to clipboard:

```bash
yek src/ | pbcopy
```

### Run

```bash
yek --help

Serialize repository content for LLM context

Usage: yek [OPTIONS] [path]

Arguments:
  [path]  Path to repository [default: .]

Options:
  -x, --max-size <max-size>      Maximum size (e.g. '10MB', '128KB', '1GB') [default: 10MB]
  -c, --config <config>          Path to config file
  -o, --output-dir <output-dir>  Directory to write output files (overrides config file)
  -k, --tokens                   Count in tokens instead of bytes
  -p, --path-prefix <PREFIX>     Only process files under this path prefix
  -v, --debug                    Enable debug logging
  -h, --help                     Print help

```

## Examples

- Serialize entire repository into chunks of 10MB (default):

```bash
yek
```

- Split repository into chunks of 128MB:

```bash
yek --max-size 128MB
```

- Split into chunks by token count instead of bytes:

```bash
yek --tokens --max-size 128000
```

- Serialize only files under a specific path:

```bash
yek --path-prefix src/app
```

- Stream output to another command:

```bash
yek | pbcopy
```

## Configuration File

You can place a file called `yek.toml` at your project root or pass a custom path via `--config`. The configuration file allows you to:

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

## Planned Features

- [ ] Be smarter about finding out test files

## License

MIT
