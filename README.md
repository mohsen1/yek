# `yek`

A [fast](#performance) Rust based tool to read text-based files in a repository or directory, chunk them, and serialize them for LLM consumption. By default, the tool:

- Uses `.gitignore` rules to skip unwanted files.
- Uses the Git history to infer what files are important.
- Infers additional ignore patterns (binary, large, etc.).
- Splits content into chunks based on either approximate "token" count or byte size.
- Automatically detects if output is being piped and streams content instead of writing to files.
- Supports processing multiple directories in a single command.
- Configurable via a `yek.toml` file.

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

<details>
<summary style="cursor: pointer;">or build from source</summary>

1. [Install Rust](https://www.rust-lang.org/tools/install).
2. Clone this repository.
3. Run `make macos` or `make linux` to build for your platform (both run `cargo build --release`).
4. Add to your PATH:

```bash
export PATH=$(pwd)/target/release:$PATH
```

</details>

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
yek --max-size 128K --tokens=deepseek-reasoner src/
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

### CLI Reference

```bash
yek --help

Repository content chunker and serializer for LLM consumption

Usage: yek [OPTIONS] [directories]...

Arguments:
  [directories]...  Directories to process [default: .]

Options:
      --max-size <max-size>      Maximum size per chunk (defaults to '10000' in token mode, '10MB' in byte mode)
      --tokens [<MODEL>]         Count size in tokens using specified model
      --debug                    Enable debug output
      --output-dir <output-dir>  Output directory for chunks
  -h, --help                     Print help

SUPPORTED MODELS:

Use with --tokens=MODEL

Available models:
  gpt-4o, gpt-4o-2024-08-06, chatgpt-4o-latest, gpt-4o-mini, gpt-4o-mini-2024-07-18, o1, o1-2024-12-17, o1-mini, o1-mini-2024-09-12, o1-preview, o1-preview-2024-09-12, gpt-4o-realtime-preview, gpt-4o-realtime-preview-2024-12-17, gpt-4o-mini-realtime-preview, gpt-4o-mini-realtime-preview-2024-12-17, gpt-4o-audio-preview, gpt-4o-audio-preview-2024-12-17, claude-3-5-sonnet-20241022, claude-3-5-sonnet-latest, claude-3-5-haiku-20241022, claude-3-5-haiku-latest, claude-3-opus-20240229, claude-3-opus-latest, claude-3-sonnet-20240229, claude-3-haiku-20240307, mistral-7b-v0-3, mistral-nemo-12b, mistral-openorca-7b, mistral-large-123b, mistral-small-22b, mistrallite-7b, mixtral-8x7b, mixtral-8x22b, llama-3-3-70b, llama-3-2-1b, llama-3-2-3b, llama-3-2-vision-11b, llama-3-2-vision-90b, llama-3-1-8b, llama-3-1-70b, llama-3-1-405b, llama-3-8b, llama-3-70b, llama-2-7b, llama-2-13b, llama-2-70b, codellama-7b, codellama-13b, codellama-34b, codellama-70b, tinyllama-1-1b
```

## Configuration File

You can place a file called `yek.toml` at your project root or pass a custom path via `--config`. The configuration file allows you to:

1. Add custom ignore patterns
2. Define file priority rules for processing order
3. Add additional binary file extensions to ignore (extends the built-in list)
4. Configure Git-based priority boost
5. Configure tokenizer model for token counting

### Example `yek.toml`

This is optional, you can configure the `yek.toml` file at the root of your project.

```toml
# Add patterns to ignore (in addition to .gitignore)
ignore_patterns = [
  "node_modules/",
  "\\.next/",
  "my_custom_folder/"
]

# Configure Git-based priority boost (optional)
git_boost_max = 50  # Maximum score boost based on Git history (default: 100)

# Configure default tokenizer model (optional, can be overridden via --tokens=<model>)
tokenizer_model = "deepseek-reasoner"  # Supported models: deepseek-reasoner, o1, claud

# Define priority rules for processing order
# Higher scores are processed first
[[priority_rules]]
score = 100
pattern = "^src/lib/"

[[priority_rules]]
score = 90
pattern = "^src/"

[[priority_rules]]
score = 80
pattern = "^docs/"

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

See [proposed features](https://github.com/bodo-run/yek/issues?q=type:%22Feature%22). I am open to accepting new feature requests. Please write a detailed proposal to discuss new features.

## License

MIT
