use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Represents a processed file with its metadata and content
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedFile {
    /// Priority score for file ordering
    pub priority: i32,
    /// Index within the same priority group for stable sorting
    pub file_index: usize,
    /// Relative path from the repository root
    pub rel_path: String,
    /// File content as string
    pub content: String,
    /// File size in bytes
    pub size_bytes: usize,
    /// Token count (computed lazily with caching)
    #[serde(skip)]
    pub token_count: OnceLock<usize>,
    /// Cached formatted content (for line numbers)
    pub formatted_content: Option<String>,
}

impl Clone for ProcessedFile {
    fn clone(&self) -> Self {
        Self {
            priority: self.priority,
            file_index: self.file_index,
            rel_path: self.rel_path.clone(),
            content: self.content.clone(),
            size_bytes: self.size_bytes,
            token_count: OnceLock::new(),
            formatted_content: self.formatted_content.clone(),
        }
    }
}

impl ProcessedFile {
    /// Create a new ProcessedFile with basic information
    pub fn new(rel_path: String, content: String, priority: i32, file_index: usize) -> Self {
        let size_bytes = content.len();
        Self {
            priority,
            file_index,
            rel_path,
            content,
            size_bytes,
            token_count: OnceLock::new(),
            formatted_content: None,
        }
    }

    /// Get token count, computing it lazily if not already computed
    pub fn get_token_count(&self) -> usize {
        *self.token_count.get_or_init(|| self.compute_token_count())
    }

    /// Get formatted content with line numbers if requested
    pub fn get_formatted_content(&self, include_line_numbers: bool) -> &str {
        if !include_line_numbers {
            return &self.content;
        }

        self.formatted_content.as_deref().unwrap_or("")
    }

    /// Compute token count for the content
    fn compute_token_count(&self) -> usize {
        // If we have formatted content cached, use that for token counting
        // as it represents the final output format
        if let Some(ref formatted) = self.formatted_content {
            crate::count_tokens(formatted)
        } else {
            // Only count tokens if we actually need them (lazy evaluation)
            // This avoids expensive tokenization for files that won't be included
            crate::count_tokens(&self.content)
        }
    }

    /// Format content with line numbers
    #[allow(dead_code)]
    fn format_content_with_line_numbers(&self) -> String {
        if self.content.is_empty() {
            return String::new();
        }

        let lines: Vec<&str> = self.content.lines().collect();
        let total_lines = lines.len();

        // Calculate the width needed for the largest line number, with minimum width of 3
        let width = if total_lines == 0 {
            3
        } else {
            std::cmp::max(3, total_lines.to_string().len())
        };

        // Use String::with_capacity for better memory allocation
        let mut result = String::with_capacity(self.content.len() + total_lines * (width + 3));

        for (i, line) in lines.iter().enumerate() {
            result.push_str(&format!("{:width$} | {}\n", i + 1, line, width = width));
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Get the size in the specified mode (bytes or tokens)
    pub fn get_size(&self, token_mode: bool, include_line_numbers: bool) -> usize {
        if token_mode {
            self.get_token_count()
        } else {
            // Use formatted content size if line numbers are requested
            if include_line_numbers {
                self.get_formatted_content(true).len()
            } else {
                self.size_bytes
            }
        }
    }

    /// Check if file would exceed size limit
    pub fn exceeds_limit(
        &self,
        limit: usize,
        token_mode: bool,
        include_line_numbers: bool,
    ) -> bool {
        self.get_size(token_mode, include_line_numbers) > limit
    }

    /// Clear caches to free memory
    pub fn clear_caches(&mut self) {
        self.token_count = OnceLock::new();
        self.formatted_content = None;
    }
}

/// Represents file priority information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePriority {
    /// Base priority from rules
    pub rule_priority: i32,
    /// Boost from git history recency
    pub git_boost: i32,
    /// Final combined priority
    pub combined: i32,
}

impl FilePriority {
    pub fn new(rule_priority: i32, git_boost: i32) -> Self {
        Self {
            rule_priority,
            git_boost,
            combined: rule_priority + git_boost,
        }
    }
}

/// Represents repository information
#[derive(Debug, Clone)]
pub struct RepositoryInfo {
    /// Root path of the repository
    pub root_path: PathBuf,
    /// Whether this is a git repository
    pub is_git_repo: bool,
    /// Git commit times for files (path -> timestamp)
    pub commit_times: std::collections::HashMap<String, u64>,
}

impl RepositoryInfo {
    pub fn new(root_path: PathBuf, is_git_repo: bool) -> Self {
        Self {
            root_path,
            is_git_repo,
            commit_times: std::collections::HashMap::new(),
        }
    }
}

/// Configuration for input processing
#[derive(Debug, Clone)]
pub struct InputConfig {
    /// Input file and directory paths
    pub input_paths: Vec<String>,
    /// Ignore patterns (compiled globs)
    pub ignore_patterns: Vec<glob::Pattern>,
    /// Binary file extensions to skip
    pub binary_extensions: std::collections::HashSet<String>,
    /// Maximum depth for git history traversal
    pub max_git_depth: i32,
    /// Maximum git boost value
    pub git_boost_max: Option<i32>,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            input_paths: Vec::new(),
            ignore_patterns: Vec::new(),
            binary_extensions: std::collections::HashSet::new(),
            max_git_depth: 100,
            git_boost_max: Some(100),
        }
    }
}

/// Configuration for output processing
#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Maximum size limit (bytes or tokens)
    pub max_size: String,
    /// Whether to use token mode instead of byte mode
    pub token_mode: bool,
    /// Token limit when in token mode
    pub token_limit: Option<String>,
    /// Output template string
    pub output_template: String,
    /// Whether to include line numbers
    pub line_numbers: bool,
    /// Whether to enable JSON output
    pub json_output: bool,
    /// Whether to include tree header
    pub tree_header: bool,
    /// Whether to show only tree (no content)
    pub tree_only: bool,
    /// Output directory (if not streaming)
    pub output_dir: Option<String>,
    /// Output filename (if not streaming)
    pub output_name: Option<String>,
    /// Whether to stream output to stdout
    pub stream: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_size: "10MB".to_string(),
            token_mode: false,
            token_limit: None,
            output_template: ">>>> FILE_PATH\nFILE_CONTENT".to_string(),
            line_numbers: false,
            json_output: false,
            tree_header: false,
            tree_only: false,
            output_dir: None,
            output_name: None,
            stream: false,
        }
    }
}

/// Configuration for processing behavior
#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    /// Priority rules for file ordering
    pub priority_rules: Vec<crate::priority::PriorityRule>,
    /// Whether to enable debug output
    pub debug: bool,
    /// Whether to enable parallel processing
    pub parallel: bool,
    /// Maximum number of concurrent threads
    pub max_threads: Option<usize>,
    /// Memory limit for processing
    pub memory_limit_mb: Option<usize>,
    /// Batch size for processing
    pub batch_size: usize,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            priority_rules: Vec::new(),
            debug: false,
            parallel: true,
            max_threads: None,
            memory_limit_mb: None,
            batch_size: 1000,
        }
    }
}

/// Processing statistics for monitoring and optimization
#[derive(Debug, Clone, Default)]
pub struct ProcessingStats {
    /// Total number of files processed
    pub files_processed: usize,
    /// Total number of files skipped
    pub files_skipped: usize,
    /// Total bytes processed
    pub bytes_processed: usize,
    /// Total tokens processed
    pub tokens_processed: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u128,
    /// Memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
}

impl ProcessingStats {
    /// Create a new stats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Add file processing statistics
    pub fn add_file(&mut self, file: &ProcessedFile, was_cached: bool) {
        self.files_processed += 1;
        self.bytes_processed += file.size_bytes;
        if let Some(token_count) = file.token_count.get() {
            self.tokens_processed += *token_count;
        }
        if was_cached {
            // This is a simplified cache hit tracking
            // In a real implementation, you'd track actual cache hits
        }
    }

    /// Add skipped file statistics
    pub fn add_skipped_file(&mut self, size_bytes: usize) {
        self.files_skipped += 1;
        self.bytes_processed += size_bytes;
    }
}
