# `yek`

A [fast](#performance) Rust based tool to serialize text-based files in a repository or directory for LLM consumption.[^1]

By default:

- Uses `.gitignore` rules to skip unwanted files.
- Uses the Git history to infer what files are more important.
- Infers additional ignore patterns (binary, large, etc.).
- Automatically detects if output is being piped and streams content instead of writing to files.
- Supports processing multiple directories in a single command.
- Supports glob patterns and individual file selection.
- Configurable via a `yek.yaml` file.

Yek <a href="https://fa.wikipedia.org/wiki/۱">يک</a> means "One" in Farsi/Persian.

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
... content ...
>>>> tests/test.rs
... content ...
>>>> src/utils.rs
... content ...
>>>> src/main.rs
... content ...
```

> [!NOTE]  
> `yek` will prioritize more important files to come last in the output. This is useful for LLM consumption since LLMs tend to pay more attention to content that appears later in the context.

## Installation

Choose the installation method for your platform:

### Unix-like Systems (macOS, Linux)

<!-- UNIX_INSTALLATION_BEGIN -->

```bash
curl -fsSL https://bodo.run/yek.sh | bash
```

<!-- UNIX_INSTALLATION_END -->

For Windows (PowerShell):

<!-- WINDOWS_INSTALLATION_BEGIN -->

```powershell
irm https://bodo.run/yek.ps1 | iex
```

<!-- WINDOWS_INSTALLATION_END -->

<details>
<summary style="cursor: pointer;">Build from Source</summary>

```bash
git clone https://github.com/bodo-run/yek
cd yek
cargo install --path .
```

</details>

## Usage

`yek` has sensible defaults, you can simply run `yek` in a directory to serialize the entire repository. It will serialize all files in the repository and write them into a temporary file. The path to the file will be printed to the console.

### Examples

Process current directory and write to temp directory:

```bash
yek
```

Pipe output to clipboard (macOS):

```bash
yek src/ | pbcopy
```

Cap the max output size to 128K tokens:

```bash
yek --tokens 128k
```

> [!NOTE]
> `yek` will remove any files that won't fit in the capped context size. It will try to fit in more important files

```bash
yek --max-size 100KB --output-dir /tmp/yek src/
```

Process multiple directories:

```bash
yek src/ tests/
```

Process multiple files

```bash
yek file1.txt file2.txt file3.txt
```

Use glob patterns:

```bash
yek "src/**/*.ts"
```

```bash
yek "src/main.rs" "tests/*.rs" "docs/README.md"
```

> [!NOTE]
> When using glob patterns, make sure to quote them to prevent shell expansion.

### CLI Reference

```bash
yek --help
Usage: yek [OPTIONS] [input-paths]...

Arguments:
  [input-paths]...                Input files and/or directories to process

Options:
      --no-config                              Do not use a config file
      --config-file <CONFIG_FILE>              Path to the config file
  -V, --version                                Print version of yek
      --max-size <MAX_SIZE>                    Max size per chunk. e.g. "10MB" or "128K" or when using token counting mode, "100" or "128K" [default: 10MB]
      --tokens <TOKENS>                        Use token mode instead of byte mode
      --json                                   Enable JSON output
      --debug                                  Enable debug output
      --line-numbers                           Include line numbers in output
      --output-dir [<OUTPUT_DIR>]              Output directory. If none is provided & stdout is a TTY, we pick a temp dir
      --output-name [<OUTPUT_NAME>]            Output filename. If provided, write output to this file in current directory
      --output-template [<OUTPUT_TEMPLATE>]    Output template. Defaults to ">>>> FILE_PATH\nFILE_CONTENT"
      --ignore-patterns <IGNORE_PATTERNS>...  Ignore patterns
      --unignore-patterns <UNIGNORE_PATTERNS>... Unignore patterns. Yek has some built-in ignore patterns, but you can override them here.
  -t, --tree-header                            Include directory tree header in output (incompatible with JSON output)
      --tree-only                              Show only the directory tree (no file contents, incompatible with JSON output)
  -h, --help                                   Print help
```

#### CLI Options Detail

- `[input-paths]...` - Files or directories to process. Supports glob patterns (quote them to prevent shell expansion)
- `--no-config` - Skip loading any configuration file
- `--config-file <CONFIG_FILE>` - Use a specific configuration file path instead of searching for default config files
- `-V, --version` - Print version information and exit
- `--max-size <MAX_SIZE>` - Maximum size limit per output (e.g., "10MB", "128K"). Used in byte mode
- `--tokens <TOKENS>` - Use token-based counting instead of bytes (e.g., "128k", "100"). Enables token mode
- `--json` - Output results in JSON format instead of text
- `--debug` - Enable debug logging for troubleshooting
- `--line-numbers` - Include line numbers in the output for each file
- `--output-dir [<OUTPUT_DIR>]` - Directory to write output files. If not specified and not streaming, uses temp directory
- `--output-name [<OUTPUT_NAME>]` - Specific filename for output. If specified, writes to current directory with this name
- `--output-template [<OUTPUT_TEMPLATE>]` - Template for formatting output. Use `FILE_PATH` and `FILE_CONTENT` placeholders
- `--ignore-patterns <IGNORE_PATTERNS>...` - Additional patterns to ignore (extends .gitignore and defaults)
- `--unignore-patterns <UNIGNORE_PATTERNS>...` - Patterns to override built-in ignore rules
- `-t, --tree-header` - Include a directory tree at the beginning of output (incompatible with JSON)
- `--tree-only` - Show only the directory tree structure without file contents (incompatible with JSON)

## Configuration File

You can place a file called `yek.yaml` at your project root or pass a custom path via `--config-file`. The configuration file allows you to:

1. Add custom ignore patterns
2. Define file priority rules for processing order
3. Add additional binary file extensions to ignore (extends the built-in list)
4. Configure Git-based priority boost
5. Define output directory and output filename
6. Define output template and other output options

### Configurable Options

Most CLI options can be configured in the config file. The following options can be set:

**File Processing:**
- `max_size` - Size limit (same as `--max-size`)
- `tokens` - Token count limit (same as `--tokens`)
- `ignore_patterns` - Additional ignore patterns (same as `--ignore-patterns`)
- `unignore_patterns` - Override built-in ignores (same as `--unignore-patterns`)

**Output Configuration:**
- `json` - Enable JSON output (same as `--json`)
- `debug` - Enable debug mode (same as `--debug`)
- `line_numbers` - Include line numbers (same as `--line-numbers`)
- `output_dir` - Output directory (same as `--output-dir`)
- `output_name` - Output filename (same as `--output-name`)
- `output_template` - Output template (same as `--output-template`)
- `tree_header` - Include directory tree header (same as `--tree-header`)
- `tree_only` - Show only directory tree (same as `--tree-only`)

**Config-only Options:**
- `priority_rules` - File priority rules (config file only)
- `binary_extensions` - Additional binary file extensions (config file only)
- `git_boost_max` - Maximum Git-based priority boost (config file only)

> [!NOTE]
> Some CLI options like `--no-config`, `--config-file`, and `--version` are CLI-only and cannot be set in config files.

### Example `yek.yaml`

You can also use `yek.toml` or `yek.json` instead of `yek.yaml`.

This is optional, you can configure the `yek.yaml` file at the root of your project.

```yaml
# Add patterns to ignore (in addition to .gitignore)
ignore_patterns:
  - "ai-prompts/**"
  - "__generated__/**"

# Configure Git-based priority boost (optional)
git_boost_max: 50 # Maximum score boost based on Git history (default: 100)

# Define priority rules for processing order
# Higher scores are processed first
priority_rules:
  - score: 100
    pattern: "^src/lib/"
  - score: 90
    pattern: "^src/"
  - score: 80
    pattern: "^docs/"

# Add additional binary file extensions to ignore
# These extend the built-in list (.jpg, .png, .exe, etc.)
binary_extensions:
  - ".blend" # Blender files
  - ".fbx" # 3D model files
  - ".max" # 3ds Max files
  - ".psd" # Photoshop files

# Output configuration
max_size: "128K"           # Size limit (can also use tokens: "100k")
json: false                # Enable JSON output
debug: false               # Enable debug logging
line_numbers: false        # Include line numbers in output
tree_header: false         # Include directory tree at start

# Define output directory
output_dir: /tmp/yek

# Define output filename (writes to current directory with this name)
output_name: yek-output.txt

# Define output template.
# FILE_PATH and FILE_CONTENT are expected to be present in the template.
output_template: "FILE_PATH\n\nFILE_CONTENT"
```

## Performance

`yek` is fast. It's written in Rust and does many things in parallel to speed up processing.

Here is a benchmark comparing it to [Repomix](https://github.com/yamadashy/repomix) serializing the [Next.js](https://github.com/vercel/next.js) project:

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

## Roadmap

See [proposed features](https://github.com/mohsen1/yek/issues?q=type:%22Feature%22). I am open to accepting new feature requests. Please write a detailed proposal to discuss new features.

## Alternatives

- [Repomix](https://github.com/yamadashy/repomix): A tool to serialize a repository into a single file in a similar way to `yek`.
- [Aider](https://aider.chat): A full IDE like experience for coding using AI

## License

[MIT](LICENSE)

[^1]: `yek` is not "blazingly" fast. It's just fast, as fast as your computer can be.
