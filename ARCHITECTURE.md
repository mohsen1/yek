# Yek Architecture Documentation

## Overview

Yek is a tool for serializing and processing file contents from repositories with intelligent prioritization and size management. This document outlines the architectural improvements made to address performance, maintainability, and robustness concerns.

## Core Architecture

### Modular Design

The architecture has been refactored into focused, single-responsibility modules:

```
src/
├── lib.rs           # Main library interface and tokenization
├── main.rs          # CLI application entry point
├── config.rs        # Configuration management (legacy)
├── models.rs        # Domain models and data structures
├── repository.rs    # File system and Git abstraction layer
├── pipeline.rs      # Processing pipeline and middleware
├── parallel_fixed.rs # Thread-safe parallel processing
├── error.rs         # Comprehensive error handling
├── priority.rs      # Priority computation and Git analysis
├── tree.rs          # Directory tree generation
└── defaults.rs      # Default configurations
```

### Domain Models

#### ProcessedFile
```rust
pub struct ProcessedFile {
    pub priority: i32,           // Priority score for ordering
    pub file_index: usize,       // Index within priority group
    pub rel_path: String,        // Relative path from repo root
    pub content: String,         // File content
    pub size_bytes: usize,       // Size in bytes
    pub token_count: Option<usize>, // Cached token count
    pub formatted_content: Option<String>, // Cached formatted content
}
```

Key improvements:
- **Lazy token counting**: Tokens are only computed when needed
- **Memory optimization**: Content formatting is cached to avoid recomputation
- **Size-aware processing**: Efficient size checking in both byte and token modes

#### RepositoryInfo
```rust
pub struct RepositoryInfo {
    pub root_path: PathBuf,      // Repository root directory
    pub is_git_repo: bool,       // Whether this is a Git repository
    pub commit_times: HashMap<String, u64>, // File -> last commit time
}
```

### Repository Pattern

The repository pattern abstracts file system and Git operations behind traits:

```rust
pub trait FileSystem {
    fn path_exists(&self, path: &Path) -> bool;
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    fn is_directory(&self, path: &Path) -> bool;
    fn resolve_symlink(&self, path: &Path) -> Result<PathBuf>;
    // ... other operations
}

pub trait GitOperations {
    fn is_git_repository(&self, path: &Path) -> bool;
    fn get_file_commit_times(&self, max_commits: usize) -> Result<HashMap<String, u64>>;
    fn get_repository_root(&self) -> Result<PathBuf>;
}
```

Benefits:
- **Testability**: Easy to mock file system operations in tests
- **Security**: Built-in path traversal protection
- **Flexibility**: Can swap implementations for different environments

### Processing Pipeline

The middleware pipeline separates concerns into distinct stages:

```rust
pub struct ProcessingPipeline {
    stages: Vec<Box<dyn ProcessingStage>>,
    context: ProcessingContext,
}
```

#### Pipeline Stages

1. **FileDiscoveryStage**: Finds and filters files to process
2. **ContentFilteringStage**: Applies size limits and content filtering
3. **OutputFormattingStage**: Applies templates and formatting

Each stage implements the `ProcessingStage` trait:

```rust
pub trait ProcessingStage: Send + Sync {
    fn process(&self, files: Vec<ProcessedFile>, context: &ProcessingContext) -> Result<Vec<ProcessedFile>>;
    fn name(&self) -> &'static str;
}
```

### Configuration Architecture

The monolithic `YekConfig` has been broken down into focused configurations:

```rust
pub struct InputConfig {
    pub input_paths: Vec<String>,
    pub ignore_patterns: Vec<glob::Pattern>,
    pub binary_extensions: HashSet<String>,
    pub max_git_depth: i32,
    pub git_boost_max: Option<i32>,
}

pub struct OutputConfig {
    pub max_size: String,
    pub token_mode: bool,
    pub output_template: String,
    pub line_numbers: bool,
    pub json_output: bool,
    pub tree_header: bool,
    pub tree_only: bool,
    pub output_dir: Option<String>,
    pub stream: bool,
}

pub struct ProcessingConfig {
    pub priority_rules: Vec<PriorityRule>,
    pub debug: bool,
    pub parallel: bool,
    pub max_threads: Option<usize>,
    pub memory_limit_mb: Option<usize>,
    pub batch_size: usize,
}
```

## Performance Optimizations

### Lazy Token Counting

Token counting is now performed lazily and cached:

```rust
impl ProcessedFile {
    pub fn get_token_count(&mut self) -> usize {
        if let Some(count) = self.token_count {
            count
        } else {
            let count = self.compute_token_count();
            self.token_count = Some(count);
            count
        }
    }
}
```

### Memory-Efficient String Operations

All string operations now use `String::with_capacity` for better memory allocation:

```rust
let mut result = String::with_capacity(self.content.len() + total_lines * (width + 3));
```

### Caching Strategy

- **Git commit times**: Cached per repository to avoid repeated traversals
- **Token counts**: Cached per file to avoid recomputation
- **Formatted content**: Cached to avoid reformatting

### Parallel Processing Improvements

Fixed race conditions in parallel processing:

```rust
pub struct ParallelFileProcessor {
    context: Arc<ProcessingContext>,
    file_counter: Arc<Mutex<HashMap<i32, usize>>>, // Thread-safe counters
}
```

## Error Handling

### Comprehensive Error Types

```rust
pub enum YekError {
    FileSystem { operation: String, path: PathBuf, source: io::Error },
    Git { operation: String, repository: PathBuf, source: git2::Error },
    Configuration { field: String, value: String, reason: String },
    Processing { stage: String, file: Option<PathBuf>, reason: String },
    Memory { operation: String, requested: usize, available: Option<usize> },
    Security { violation: String, path: PathBuf, attempted_by: String },
    // ... other error types
}
```

### Error Context

Rich error context for better debugging:

```rust
pub struct ErrorContext {
    pub operation: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub additional_info: Vec<(String, String)>,
}
```

### User-Friendly Error Reporting

```rust
impl ErrorReporter {
    pub fn report_error(error: &YekError, context: &ErrorContext, verbose: bool) {
        // Contextual error reporting with suggestions
    }
}
```

## Security Improvements

### Path Traversal Protection

```rust
pub fn safe_validate_path(
    input_path: &Path,
    base_path: &Path,
    context: &ErrorContext,
) -> YekResult<PathBuf> {
    let normalized = std::fs::canonicalize(input_path)?;
    if let Ok(relative) = normalized.strip_prefix(base_path) {
        Ok(normalized) // Safe path
    } else {
        Err((YekError::Security { /* ... */ }, context))
    }
}
```

### Symlink Loop Detection

```rust
fn resolve_symlink(&self, path: &Path) -> Result<PathBuf> {
    let mut visited = HashSet::new();
    let mut current = path.to_path_buf();

    for _ in 0..100 { // Prevent infinite loops
        if !self.is_symlink(&current) {
            break;
        }
        if !visited.insert(current.clone()) {
            return Err(anyhow!("Symlink loop detected"));
        }
        current = fs::read_link(&current)?;
    }
    Ok(current)
}
```

### UTF-8 Validation with Fallback

```rust
pub fn safe_validate_utf8(bytes: &[u8], context: &ErrorContext) -> YekResult<String> {
    match String::from_utf8(bytes.to_vec()) {
        Ok(content) => Ok(content),
        Err(utf8_err) => {
            let recovered = String::from_utf8_lossy(bytes);
            if recovered.contains('\u{FFFD}') {
                eprintln!("Warning: Invalid UTF-8 sequences replaced with �");
            }
            Ok(recovered.to_string())
        }
    }
}
```

## Testing Strategy

### Comprehensive Error Testing

The new architecture enables better testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::MockFileSystem;

    #[test]
    fn test_file_discovery_with_missing_files() {
        let mock_fs = MockFileSystem::new();
        // Test error handling for missing files
    }

    #[test]
    fn test_path_traversal_protection() {
        // Test security protections
    }

    #[test]
    fn test_memory_limit_enforcement() {
        // Test memory safety
    }
}
```

### Performance Benchmarks

```rust
#[cfg(test)]
mod benches {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn benchmark_large_file_processing(c: &mut Criterion) {
        // Benchmark performance with large files
    }

    fn benchmark_parallel_processing(c: &mut Criterion) {
        // Benchmark parallel processing efficiency
    }
}
```

## Migration Guide

### For Users

The new architecture is backward compatible. No changes are required for existing usage:

```bash
# All existing commands work the same
yek --input src/ --max-size 10MB --output output.txt
yek --input . --tokens 1000 --json
```

### For Developers

#### Accessing New Features

```rust
use yek::{models::ProcessedFile, pipeline::ProcessingPipeline, error::YekResult};

// Create processing context
let context = ProcessingContext::new(
    input_config,
    output_config,
    processing_config,
    repository_info,
    Arc::new(RealFileSystem),
    git_operations,
);

// Use the pipeline
let pipeline = ProcessingPipeline::new(context);
let files = pipeline.process()?;
```

#### Error Handling

```rust
use yek::error::{YekError, ErrorContext, ErrorReporter};

match some_operation() {
    Ok(result) => result,
    Err((error, context)) => {
        ErrorReporter::report_error(&error, &context, verbose);
        // Handle error appropriately
    }
}
```

## Future Enhancements

### Planned Improvements

1. **Streaming Processing**: True streaming for memory-constrained environments
2. **Plugin Architecture**: Extensible processing stages
3. **Distributed Processing**: Multi-node file processing
4. **Advanced Caching**: Redis/in-memory cache backends
5. **Metrics Collection**: Prometheus metrics integration

### Extension Points

The architecture provides several extension points:

- **Custom Processing Stages**: Implement `ProcessingStage` trait
- **Custom File Systems**: Implement `FileSystem` trait
- **Custom Error Handlers**: Extend error reporting
- **Custom Repository Operations**: Implement `GitOperations` trait

## Performance Guidelines

### For Large Repositories

1. **Use token mode** for more accurate size management
2. **Enable parallel processing** for better CPU utilization
3. **Set appropriate memory limits** to prevent OOM
4. **Use streaming output** for very large outputs

### For Memory-Constrained Environments

1. **Enable streaming mode** to avoid loading everything into memory
2. **Use smaller batch sizes** for processing
3. **Enable lazy token counting** to avoid unnecessary computation
4. **Set memory limits** to prevent system issues

### For High-Performance Needs

1. **Maximize parallel processing** with appropriate thread counts
2. **Use caching** to avoid redundant Git operations
3. **Optimize ignore patterns** to reduce file system traversal
4. **Use batch processing** for large numbers of files

## Troubleshooting

### Common Issues

#### Memory Issues
```
Error: Memory error during 'file reading' - requested: 500MB, available: 100MB
Suggestion: Try reducing the file size or use streaming mode.
```

**Solutions:**
- Enable streaming mode: `--stream`
- Use token mode instead of byte mode
- Set memory limits: `--memory-limit 512MB`
- Process files in smaller batches

#### Path Traversal Errors
```
Error: Security violation 'Path traversal attempt' for path '/outside/repo'
Suggestion: This appears to be a security violation. Please check your input paths.
```

**Solutions:**
- Ensure all input paths are within the intended directory
- Use absolute paths when necessary
- Check for symlinks that might point outside the repository

#### Git Repository Issues
```
Error: Git error during 'commit analysis' in repository '/path/to/repo'
Suggestion: Check if the repository is a valid Git repository.
```

**Solutions:**
- Ensure the directory is a Git repository
- Check Git repository permissions
- Reduce `max_git_depth` for large repositories

### Debug Mode

Enable debug mode for detailed information:

```bash
yek --debug --input src/ --max-size 10MB
```

This provides detailed logging of:
- File discovery process
- Processing stage timing
- Memory usage statistics
- Cache hit rates
- Error details with context

## Contributing

### Architecture Principles

When contributing to Yek, please follow these principles:

1. **Single Responsibility**: Each module should have one clear purpose
2. **Dependency Injection**: Use traits for external dependencies
3. **Error Context**: Provide rich error information for debugging
4. **Performance Awareness**: Consider memory and CPU impact
5. **Security First**: Validate all inputs and prevent path traversal
6. **Testability**: Design code to be easily testable

### Adding New Processing Stages

```rust
pub struct CustomProcessingStage;

impl ProcessingStage for CustomProcessingStage {
    fn process(&self, files: Vec<ProcessedFile>, context: &ProcessingContext) -> Result<Vec<ProcessedFile>> {
        // Custom processing logic
        Ok(files)
    }

    fn name(&self) -> &'static str {
        "CustomProcessing"
    }
}

// Usage
pipeline.add_stage(Box::new(CustomProcessingStage));
```

This architecture provides a solid foundation for extending Yek's capabilities while maintaining performance, security, and maintainability.