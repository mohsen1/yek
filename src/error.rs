use std::{
    fmt, io,
    path::{Path, PathBuf},
};

/// Custom error types for better error handling and user feedback
#[derive(Debug)]
pub enum YekError {
    /// File system related errors
    FileSystem {
        operation: String,
        path: PathBuf,
        source: io::Error,
    },

    /// Git operation errors
    Git {
        operation: String,
        repository: PathBuf,
        source: git2::Error,
    },

    /// Configuration errors
    Configuration {
        field: String,
        value: String,
        reason: String,
    },

    /// Processing errors
    Processing {
        stage: String,
        file: Option<PathBuf>,
        reason: String,
    },

    /// Memory errors
    Memory {
        operation: String,
        requested: usize,
        available: Option<usize>,
    },

    /// Path traversal/security errors
    Security {
        violation: String,
        path: PathBuf,
        attempted_by: String,
    },

    /// Validation errors
    Validation {
        field: String,
        value: String,
        constraint: String,
    },

    /// Tokenization errors
    Tokenization {
        content_type: String,
        size: usize,
        reason: String,
    },

    /// User input errors
    UserInput {
        input_type: String,
        value: String,
        suggestion: String,
    },
}

impl fmt::Display for YekError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YekError::FileSystem {
                operation,
                path,
                source,
            } => {
                write!(
                    f,
                    "File system error during '{}' on '{}': {}",
                    operation,
                    path.display(),
                    source
                )
            }
            YekError::Git {
                operation,
                repository,
                source,
            } => {
                write!(
                    f,
                    "Git error during '{}' in repository '{}': {}",
                    operation,
                    repository.display(),
                    source
                )
            }
            YekError::Configuration {
                field,
                value,
                reason,
            } => {
                write!(
                    f,
                    "Configuration error for field '{}' with value '{}': {}",
                    field, value, reason
                )
            }
            YekError::Processing {
                stage,
                file,
                reason,
            } => {
                if let Some(file_path) = file {
                    write!(
                        f,
                        "Processing error in stage '{}' for file '{}': {}",
                        stage,
                        file_path.display(),
                        reason
                    )
                } else {
                    write!(f, "Processing error in stage '{}': {}", stage, reason)
                }
            }
            YekError::Memory {
                operation,
                requested,
                available,
            } => match available {
                Some(avail) => write!(
                    f,
                    "Memory error during '{}' - requested: {} bytes, available: {} bytes",
                    operation, requested, avail
                ),
                None => write!(
                    f,
                    "Memory error during '{}' - requested: {} bytes",
                    operation, requested
                ),
            },
            YekError::Security {
                violation,
                path,
                attempted_by,
            } => {
                write!(
                    f,
                    "Security violation '{}' for path '{}' attempted by: {}",
                    violation,
                    path.display(),
                    attempted_by
                )
            }
            YekError::Validation {
                field,
                value,
                constraint,
            } => {
                write!(
                    f,
                    "Validation error for field '{}' with value '{}': violates constraint '{}'",
                    field, value, constraint
                )
            }
            YekError::Tokenization {
                content_type,
                size,
                reason,
            } => {
                write!(
                    f,
                    "Tokenization error for {} content (size: {}): {}",
                    content_type, size, reason
                )
            }
            YekError::UserInput {
                input_type,
                value,
                suggestion,
            } => {
                write!(
                    f,
                    "Invalid {} input '{}': {}",
                    input_type, value, suggestion
                )
            }
        }
    }
}

impl std::error::Error for YekError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            YekError::FileSystem { source, .. } => Some(source),
            YekError::Git { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Error context for better debugging and user feedback
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub additional_info: Vec<(String, String)>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            file: None,
            line: None,
            column: None,
            additional_info: Vec::new(),
        }
    }

    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_location(mut self, line: u32, column: u32) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_info.push((key.into(), value.into()));
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new("unknown_operation")
    }
}

/// Enhanced result type with context
pub type YekResult<T> = std::result::Result<T, Box<(YekError, ErrorContext)>>;

/// Error reporting utilities
pub struct ErrorReporter;

impl ErrorReporter {
    /// Report an error with context to the user
    pub fn report_error(error: &YekError, context: &ErrorContext, verbose: bool) {
        // Always show the main error
        eprintln!("Error: {}", error);

        // Show context information
        if verbose {
            if let Some(ref file) = context.file {
                eprintln!("  File: {}", file.display());
            }
            if let Some(line) = context.line {
                eprintln!("  Line: {}", line);
            }
            if let Some(column) = context.column {
                eprintln!("  Column: {}", column);
            }
            eprintln!("  Operation: {}", context.operation);

            // Show additional info
            for (key, value) in &context.additional_info {
                eprintln!("  {}: {}", key, value);
            }
        }

        // Show suggestions for common errors
        Self::show_suggestions(error, context);
    }

    /// Show helpful suggestions for common errors
    fn show_suggestions(error: &YekError, _context: &ErrorContext) {
        match error {
            YekError::FileSystem {
                operation, path, ..
            } => {
                if operation.contains("read") {
                    if !path.exists() {
                        eprintln!("Suggestion: Check if the file exists and the path is correct.");
                    } else if let Ok(metadata) = std::fs::metadata(path) {
                        if metadata.permissions().readonly() {
                            eprintln!("Suggestion: Check if the file is readable (permissions).");
                        }
                    }
                }
            }
            YekError::Configuration {
                field,
                value,
                reason,
            } => {
                eprintln!(
                    "Suggestion: Fix the '{}' configuration value '{}' - {}",
                    field, value, reason
                );
            }
            YekError::Memory {
                operation,
                requested: _,
                ..
            } => {
                eprintln!(
                    "Suggestion: Try reducing the '{}' size or use streaming mode.",
                    operation
                );
                eprintln!(
                    "Suggestion: Consider using token mode instead of byte mode for large files."
                );
            }
            YekError::Security { violation: _, .. } => {
                eprintln!("Suggestion: This appears to be a security violation. Please check your input paths.");
            }
            YekError::Validation {
                field, constraint, ..
            } => {
                eprintln!(
                    "Suggestion: The '{}' field violates the constraint '{}'.",
                    field, constraint
                );
            }
            _ => {}
        }
    }

    /// Create a user-friendly error message
    pub fn user_friendly_message(error: &YekError) -> String {
        match error {
            YekError::FileSystem {
                operation, path, ..
            } => {
                format!("Failed to {} file '{}'", operation, path.display())
            }
            YekError::Git {
                operation,
                repository,
                ..
            } => {
                format!(
                    "Git operation '{}' failed in repository '{}'",
                    operation,
                    repository.display()
                )
            }
            YekError::Configuration { field, reason, .. } => {
                format!("Configuration issue with '{}': {}", field, reason)
            }
            YekError::Processing {
                stage,
                file,
                reason,
            } => {
                if let Some(file_path) = file {
                    format!(
                        "Processing failed in '{}' stage for '{}': {}",
                        stage,
                        file_path.display(),
                        reason
                    )
                } else {
                    format!("Processing failed in '{}' stage: {}", stage, reason)
                }
            }
            YekError::Memory {
                operation,
                requested,
                ..
            } => {
                format!(
                    "Insufficient memory for '{}' (requested: {} bytes)",
                    operation, requested
                )
            }
            YekError::Security {
                violation, path, ..
            } => {
                format!(
                    "Security violation '{}' for path '{}'",
                    violation,
                    path.display()
                )
            }
            YekError::Validation {
                field, constraint, ..
            } => {
                format!(
                    "Validation failed for '{}': violates '{}'",
                    field, constraint
                )
            }
            YekError::Tokenization {
                content_type,
                size,
                reason,
            } => {
                format!(
                    "Failed to process {} content ({}): {}",
                    content_type, size, reason
                )
            }
            YekError::UserInput {
                input_type,
                suggestion,
                ..
            } => {
                format!("Invalid {}: {}", input_type, suggestion)
            }
        }
    }
}

/// Utilities for safe operations with error handling
pub mod safe_ops {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Safely read a file with comprehensive error handling
    pub fn safe_read_file(
        path: &Path,
        context: &ErrorContext,
        max_size: Option<usize>,
    ) -> YekResult<Vec<u8>> {
        // Check if file exists
        if !path.exists() {
            return Err(Box::new((
                YekError::FileSystem {
                    operation: "read".to_string(),
                    path: path.to_path_buf(),
                    source: io::Error::new(io::ErrorKind::NotFound, "File not found"),
                },
                context.clone(),
            )));
        }

        // Check if it's actually a file
        if !path.is_file() {
            return Err(Box::new((
                YekError::FileSystem {
                    operation: "read".to_string(),
                    path: path.to_path_buf(),
                    source: io::Error::new(io::ErrorKind::InvalidInput, "Path is not a file"),
                },
                context.clone(),
            )));
        }

        // Read file content
        match std::fs::read(path) {
            Ok(content) => {
                // Check size limits
                if let Some(max_size) = max_size {
                    if content.len() > max_size {
                        return Err(Box::new((
                            YekError::Memory {
                                operation: "file reading".to_string(),
                                requested: content.len(),
                                available: Some(max_size),
                            },
                            context.clone(),
                        )));
                    }
                }
                Ok(content)
            }
            Err(source) => Err(Box::new((
                YekError::FileSystem {
                    operation: "read".to_string(),
                    path: path.to_path_buf(),
                    source,
                },
                context.clone(),
            ))),
        }
    }

    /// Safely validate UTF-8 content with fallback
    pub fn safe_validate_utf8(bytes: &[u8], _context: &ErrorContext) -> YekResult<String> {
        match String::from_utf8(bytes.to_vec()) {
            Ok(content) => Ok(content),
            Err(_utf8_err) => {
                // Try to recover by replacing invalid sequences
                let recovered = String::from_utf8_lossy(bytes);
                if recovered.contains('\u{FFFD}') {
                    // Contains replacement characters, report as warning
                    eprintln!("Warning: File contains invalid UTF-8 sequences, replaced with � characters");
                }
                Ok(recovered.to_string())
            }
        }
    }

    /// Safely check path traversal attempts
    pub fn safe_validate_path(
        input_path: &Path,
        base_path: &Path,
        context: &ErrorContext,
    ) -> YekResult<PathBuf> {
        // Normalize the path
        let normalized = std::fs::canonicalize(input_path).map_err(|source| {
            Box::new((
                YekError::FileSystem {
                    operation: "canonicalize".to_string(),
                    path: input_path.to_path_buf(),
                    source,
                },
                context.clone(),
            ))
        })?;

        // Check for path traversal attempts
        if let Ok(_relative) = normalized.strip_prefix(base_path) {
            // Path is within base directory, safe
            Ok(normalized)
        } else {
            // Path tries to escape base directory, potential security issue
            Err(Box::new((
                YekError::Security {
                    violation: "Path traversal attempt".to_string(),
                    path: normalized,
                    attempted_by: context.operation.clone(),
                },
                context.clone(),
            )))
        }
    }

    /// Safely handle mutex operations with poison error recovery
    pub fn safe_mutex_access<T, F, R>(
        mutex: &Arc<Mutex<T>>,
        operation: F,
        context: &ErrorContext,
    ) -> YekResult<R>
    where
        F: FnOnce(&mut T) -> R,
        T: fmt::Debug,
    {
        match mutex.lock() {
            Ok(mut guard) => Ok(operation(&mut guard)),
            Err(poison_err) => {
                // Mutex was poisoned, try to recover
                eprintln!(
                    "Warning: Mutex was poisoned, attempting recovery for operation: {}",
                    context.operation
                );

                // Try to recover the mutex
                let mut guard = poison_err.into_inner();
                Ok(operation(&mut guard))
            }
        }
    }
}
