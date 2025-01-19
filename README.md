# `yek`

A [fast](#performance) Rust based tool to read text-based files in a repository or directory, chunk them, and serialize them for LLM consumption. By default, the tool:

- Uses `.gitignore` rules to skip unwanted files.
- Uses the Git history to infer what files are important.
- Infers additional ignore patterns (binary, large, etc.).
- Splits content into chunks based on either approximate "token" count or byte size.
- Automatically detects if output is being piped and streams content instead of writing to files.
- Supports processing multiple directories in a single command.
- Configurable via a `yek.toml` file.

Yek ([يک](https://fa.wikipedia.org/wiki/۱)) means "One" in Farsi/Persian.

Consider having a simple repo like this:

```
.
├── README.md
├── src
│   ├── main.rs
│   └── utils.rs
└── tests
    └── test.rs
```

Running `yek` in this directory will produce a single file and write it to the temp directory with the following content:

```txt
>>>> README.md
... content of README.md ...
>>>> tests/test.rs
... content of tests/test.rs ...
>>>> src/utils.rs
... content of src/utils.rs ...
>>>> src/main.rs
... rest of the file ...
```

> [!NOTE]
> `yek` will prioritize more important files to come last in the output. This is useful for LLM consumption.

## Installation

### Via Homebrew (recommended for macOS)

<!-- HOMEBREW_INSTALLATION_BEGIN -->

```bash
brew tap bodo-run/yek https://github.com/bodo-run/yek.git
brew install yek
```

<!-- HOMEBREW_INSTALLATION_END -->

### Via Install Script

For Unix-like systems (macOS, Linux):

<!-- LINUX_INSTALLATION_BEGIN -->

```bash
curl -fsSL https://bodo.run/yek.sh | bash
```

<!-- LINUX_INSTALLATION_END -->

For Windows (PowerShell):

<!-- WINDOWS_INSTALLATION_BEGIN -->

```powershell
irm https://bodo.run/yek.ps1 | iex
```

<!-- WINDOWS_INSTALLATION_END -->

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

Process current directory and write to temp directory:

```bash
yek
```

Pipe output to clipboard (macOS):

```bash
yek src/ | pbcopy
```

Cap the max size to 128K tokens and only process the `src` directory:

```bash
yek --max-size 128K --tokens src/
```

> [!NOTE]
> When multiple chunks are written, the last chunk will contain the highest-priority files.

Cap the max size to 100KB and only process the `src` directory, writing to a specific directory:

```bash
yek --max-size 100KB --output-dir /tmp/yek src/
```

Process multiple directories:

```bash
yek src/ tests/
```

Process multiple repositories:

```bash
yek ~/code/project1 ~/code/project2
```

### Help

```bash
yek --help

Repository content chunker and serializer for LLM consumption

Usage: yek [OPTIONS] [directories]...

Arguments:
  [directories]...  Directories to process [default: .]

Options:
      --max-size <max-size>      Maximum size per chunk (e.g. '10MB', '128KB', '1GB') [default: 10MB]
      --tokens                   Count size in tokens instead of bytes
      --debug                    Enable debug output
      --output-dir <output-dir>  Output directory for chunks
  -h, --help                     Print help
  -V, --version                  Print version
```

## Configuration File

You can place a file called `yek.toml` at your project root or pass a custom path via `--config`. The configuration file allows you to:

1. Add custom ignore patterns
2. Define file priority rules for processing order
3. Add additional binary file extensions to ignore (extends the built-in list)
4. Configure Git-based priority boost

### Example `yek.toml`

This is optional, you can configure the `yek.toml` file at the root of your project.

```toml
# Add patterns to ignore (in addition to .gitignore)
[ignore_patterns]
patterns = [
  "node_modules/",
  "\\.next/",
  "my_custom_folder/"
]

# Configure Git-based priority boost (optional)
git_boost_max = 50  # Maximum score boost based on Git history (default: 100)

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
- Git-based priority boost maximum is 100
- Common binary file extensions are ignored (.jpg, .png, .exe, etc. - see source for full list)

## Performance

`yek` is fast. It's written in Rust and does many things in parallel to speed up processing.

Here is a benchmark comparing it to [Repomix](https://github.com/jxnl/repomix) serializing the [Next.js](https://github.com/vercel/next.js) project:

```bash
time yek
Executed in    5.19 secs    fish           external
   usr time    2.85 secs   54.00 micros    2.85 secs
   sys time    6.31 secs  629.00 micros    6.31 secs
```

```bash
time repomix
Executed in   22.24 mins    fish           external
   usr time   21.99 mins    0.18 millis   21.99 mins
   sys time    0.23 mins    1.72 millis    0.23 mins
```

`yek` is **230x faster** than `repomix`.

## Planned Features

- [ ] Be smarter about finding out test files

## License

MIT
