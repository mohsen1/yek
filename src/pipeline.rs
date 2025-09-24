use crate::{
    models::{
        InputConfig, OutputConfig, ProcessedFile, ProcessingConfig, ProcessingStats, RepositoryInfo,
    },
    repository::{FileSystem, RepositoryFactory},
};
use anyhow::{anyhow, Result};
use glob;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

/// Processing stage trait for the middleware pipeline
pub trait ProcessingStage {
    /// Process a batch of files
    fn process(
        &self,
        files: Vec<ProcessedFile>,
        context: &ProcessingContext,
    ) -> Result<Vec<ProcessedFile>>;

    /// Get stage name for logging and debugging
    fn name(&self) -> &'static str;
}

/// Context passed through the processing pipeline
#[derive(Clone)]
pub struct ProcessingContext {
    pub input_config: Arc<InputConfig>,
    pub output_config: Arc<OutputConfig>,
    pub processing_config: Arc<ProcessingConfig>,
    pub repository_info: Arc<RepositoryInfo>,
    pub stats: Arc<Mutex<ProcessingStats>>,
    pub file_system: Arc<dyn FileSystem + Send + Sync>,
}

impl ProcessingContext {
    pub fn new(
        input_config: InputConfig,
        output_config: OutputConfig,
        processing_config: ProcessingConfig,
        repository_info: RepositoryInfo,
        file_system: Arc<dyn FileSystem + Send + Sync>,
    ) -> Self {
        Self {
            input_config: Arc::new(input_config),
            output_config: Arc::new(output_config),
            processing_config: Arc::new(processing_config),
            repository_info: Arc::new(repository_info),
            stats: Arc::new(Mutex::new(ProcessingStats::new())),
            file_system,
        }
    }
}

/// File discovery stage - finds and filters files to process
pub struct FileDiscoveryStage {
    repository_factory: RepositoryFactory,
}

impl Default for FileDiscoveryStage {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDiscoveryStage {
    pub fn new() -> Self {
        Self {
            repository_factory: RepositoryFactory::new(),
        }
    }
}

impl ProcessingStage for FileDiscoveryStage {
    fn process(
        &self,
        _files: Vec<ProcessedFile>,
        context: &ProcessingContext,
    ) -> Result<Vec<ProcessedFile>> {
        // eprintln!("DEBUG: FileDiscoveryStage::process called");
        let start_time = Instant::now();
        let mut discovered_files = Vec::new();

        // Calculate optimal base directory for mixed inputs
        let base_dir = self.calculate_base_directory(&context.input_config.input_paths)?;
        // eprintln!("DEBUG: Base directory calculated: {}", base_dir.display());

        // Create repository info for the base directory
        let repo_info = self
            .repository_factory
            .create_repository_info(&base_dir, &context.input_config)?;

        // Process all inputs with the same repository context
        for input_path in &context.input_config.input_paths {
            let path = Path::new(input_path);
            // eprintln!("DEBUG: Processing input path: {}", path.display());

            // Discover files in this path
            let files = self.discover_files_in_path(path, &repo_info, context)?;
            // eprintln!("DEBUG: Found {} files for path: {}", files.len(), path.display());
            discovered_files.extend(files);
        }

        // Update stats
        if let Ok(mut stats) = context.stats.lock() {
            stats.processing_time_ms += start_time.elapsed().as_millis();
            stats.files_processed = discovered_files.len();
        }

        Ok(discovered_files)
    }

    fn name(&self) -> &'static str {
        "FileDiscovery"
    }
}

impl FileDiscoveryStage {
    /// Calculate the optimal base directory for mixed inputs
    fn calculate_base_directory(&self, input_paths: &[String]) -> Result<std::path::PathBuf> {
        if input_paths.is_empty() {
            return Ok(std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()));
        }

        // Convert all paths to absolute paths for comparison, handling glob patterns
        let mut absolute_paths = Vec::new();
        for path_str in input_paths {
            let path = Path::new(path_str);

            // For glob patterns, extract the directory part
            let actual_path =
                if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
                    // This is a glob pattern, extract the directory part
                    if let Some(parent) = path.parent() {
                        if parent == Path::new("") {
                            // Pattern like "*.txt" - use current directory
                            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
                        } else {
                            // Pattern like "src/*.txt" - use the parent directory
                            if parent.is_absolute() {
                                parent.to_path_buf()
                            } else {
                                std::env::current_dir()
                                    .unwrap_or_else(|_| Path::new(".").to_path_buf())
                                    .join(parent)
                            }
                        }
                    } else {
                        // Pattern with no directory part - use current directory
                        std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
                    }
                } else {
                    // Regular path
                    if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        std::env::current_dir()
                            .unwrap_or_else(|_| Path::new(".").to_path_buf())
                            .join(path)
                    }
                };
            absolute_paths.push(actual_path);
        }

        // Check if all paths are files (not directories)
        let all_files = absolute_paths.iter().all(|path| {
            // Check if the path exists and is a file, or if it doesn't exist but has a file extension
            path.exists() && path.is_file() || (!path.exists() && path.extension().is_some())
        });

        // If all paths are files, use the parent directory of the first file as the base
        if all_files && !absolute_paths.is_empty() {
            let first_file = &absolute_paths[0];
            if let Some(parent) = first_file.parent() {
                return Ok(parent.to_path_buf());
            }
        }

        // Find the common base directory
        let first_path = &absolute_paths[0];
        let mut common_base = first_path.as_path();

        for path in &absolute_paths[1..] {
            common_base = self.find_common_base(common_base, path);
        }

        Ok(common_base.to_path_buf())
    }

    /// Find the common base directory between two paths
    fn find_common_base<'a>(&self, path1: &'a Path, path2: &'a Path) -> &'a Path {
        let ancestors1 = path1.ancestors().collect::<Vec<_>>();
        let ancestors2 = path2.ancestors().collect::<Vec<_>>();

        // Find the last common ancestor
        for (a, b) in ancestors1.iter().zip(ancestors2.iter()) {
            if a != b {
                // Return the parent of the first mismatch
                return ancestors1
                    .iter()
                    .position(|&x| x == *a)
                    .and_then(|pos| ancestors1.get(pos + 1)).copied()
                    .unwrap_or_else(|| Path::new("."));
            }
        }

        // If one path is a parent of the other, return the shorter one
        if ancestors1.len() <= ancestors2.len() {
            path1
        } else {
            path2
        }
    }

    /// Expand glob patterns into concrete paths
    fn expand_globs(&self, path: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut expanded_paths = Vec::new();
        let path_str = path.to_string_lossy();

        // Check if the path contains glob patterns
        // eprintln!("DEBUG: Checking path for glob patterns: {}", path_str);
        if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            // eprintln!("DEBUG: Found glob pattern, expanding: {}", path_str);
            // Expand glob pattern
            for entry in glob::glob(&path_str)? {
                match entry {
                    Ok(expanded_path) => {
                        // eprintln!("DEBUG: Found glob match: {}", expanded_path.display());
                        // Convert to absolute path to ensure consistency
                        let absolute_path = if expanded_path.is_absolute() {
                            expanded_path
                        } else {
                            std::env::current_dir()
                                .unwrap_or_else(|_| Path::new(".").to_path_buf())
                                .join(expanded_path)
                        };
                        expanded_paths.push(absolute_path);
                    }
                    Err(e) => {
                        // Log the error but continue with other matches
                        eprintln!("Warning: Glob pattern error for '{}': {}", path_str, e);
                    }
                }
            }
        } else {
            // No glob patterns, use the path as-is, converting to absolute
            let absolute_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| Path::new(".").to_path_buf())
                    .join(path)
            };
            expanded_paths.push(absolute_path);
        }

        Ok(expanded_paths)
    }

    fn discover_files_in_path(
        &self,
        path: &Path,
        repo_info: &RepositoryInfo,
        context: &ProcessingContext,
    ) -> Result<Vec<ProcessedFile>> {
        // eprintln!("DEBUG: discover_files_in_path called with: {}", path.display());
        let mut files = Vec::new();

        // First, expand glob patterns
        let expanded_paths = self.expand_globs(path)?;

        for expanded_path in expanded_paths {
            // eprintln!("DEBUG: Processing expanded path: {}", expanded_path.display());
            // eprintln!("DEBUG: is_file: {}, is_directory: {}", context.file_system.is_file(&expanded_path), context.file_system.is_directory(&expanded_path));
            if context.file_system.is_file(&expanded_path) {
                // Single file
                // eprintln!("DEBUG: Processing single file: {}", expanded_path.display());
                if let Ok(processed_file) =
                    self.process_single_file(&expanded_path, repo_info, context)
                {
                    // eprintln!("DEBUG: Successfully processed file: {} with content length: {}", processed_file.rel_path, processed_file.content.len());
                    files.push(processed_file);
                } else {
                    // eprintln!("DEBUG: Failed to process file: {}", expanded_path.display());
                }
            } else if context.file_system.is_directory(&expanded_path) {
                // Directory - walk recursively
                // eprintln!("DEBUG: Processing directory: {}", expanded_path.display());
                self.walk_directory(&expanded_path, repo_info, context, &mut files)?;
            } else {
                // eprintln!("DEBUG: Path is neither file nor directory: {}", expanded_path.display());
            }
        }

        Ok(files)
    }

    fn process_single_file(
        &self,
        file_path: &Path,
        repo_info: &RepositoryInfo,
        context: &ProcessingContext,
    ) -> Result<ProcessedFile> {
        // eprintln!("DEBUG: process_single_file called for: {}", file_path.display());
        // Check if file should be ignored
        if self.should_ignore_file(file_path, context) {
            // eprintln!("DEBUG: File ignored: {}", file_path.display());
            return Err(anyhow!("File ignored: {}", file_path.display()));
        }

        // Read file content
        // eprintln!("DEBUG: Reading file content for: {}", file_path.display());
        let content = match crate::repository::convenience::read_file_content_safe(
            file_path,
            &*context.file_system,
        ) {
            Ok(content) => {
                // eprintln!("DEBUG: Successfully read file content, length: {}", content.len());
                content
            }
            Err(e) => {
                // eprintln!("DEBUG: Failed to read file content: {}", e);
                return Err(anyhow!(
                    "Failed to read file '{}': {}",
                    file_path.display(),
                    e
                ));
            }
        };

        // Create processed file
        // eprintln!("DEBUG: Creating relative path for: {} with base: {}", file_path.display(), repo_info.root_path.display());
        let rel_path = match crate::repository::convenience::get_relative_path(
            file_path,
            &repo_info.root_path,
        ) {
            Ok(path) => {
                let rel_path_str = path.to_string_lossy().to_string();
                // eprintln!("DEBUG: Successfully created relative path: {}", rel_path_str);
                rel_path_str
            }
            Err(e) => {
                // eprintln!("DEBUG: Failed to create relative path: {}", e);
                return Err(anyhow!(
                    "Failed to create relative path for '{}': {}",
                    file_path.display(),
                    e
                ));
            }
        };

        // Calculate priority
        let priority = self.calculate_priority(&rel_path, repo_info, context);

        Ok(ProcessedFile::new(rel_path, content, priority, 0))
    }

    fn should_ignore_file(&self, path: &Path, context: &ProcessingContext) -> bool {
        // eprintln!("DEBUG: should_ignore_file called for: {}", path.display());
        let _path_str = path.to_string_lossy();
        let _file_name = path.file_name().unwrap_or_default().to_string_lossy();
        // eprintln!("DEBUG: path_str: {}, file_name: {}", path_str, file_name);

        // First check binary extensions - these always take precedence
        let is_binary = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| context.input_config.binary_extensions.contains(ext))
            .unwrap_or(false);

        if is_binary {
            // eprintln!("DEBUG: File ignored due to binary extension");
            return true;
        }

        // Check if file should be ignored by .gitignore files
        let ignored_by_gitignore = self.is_ignored_by_gitignore(path, context);
        // eprintln!("DEBUG: ignored_by_gitignore: {}", ignored_by_gitignore);

        // Check if file should be ignored by default patterns (like LICENSE)
        let ignored_by_default = self.is_ignored_by_default_patterns(path, context);
        // eprintln!("DEBUG: ignored_by_default: {}", ignored_by_default);

        // Check if file is allowlisted by any source (.gitignore or config)
        let allowlisted = self.is_allowlisted(path, context);
        // eprintln!("DEBUG: allowlisted: {}", allowlisted);

        // A file is ignored if it's ignored by any source AND not allowlisted
        
        // eprintln!("DEBUG: final should_ignore: {}", should_ignore);

        (ignored_by_gitignore || ignored_by_default) && !allowlisted
    }

    /// Check if a file should be ignored by default patterns (like LICENSE)
    fn is_ignored_by_default_patterns(&self, path: &Path, context: &ProcessingContext) -> bool {
        let path_str = path.to_string_lossy();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();

        // Get the relative path from the repository root
        let rel_path = match path.strip_prefix(&context.repository_info.root_path) {
            Ok(rel_path) => rel_path.to_string_lossy().to_string(),
            Err(_) => path_str.to_string(), // Fallback to full path if we can't make it relative
        };

        // Check default ignore patterns (these are built into the config)
        for pattern in &context.input_config.ignore_patterns {
            let pattern_str = pattern.as_str();
            // Skip allowlist patterns (starting with !) for default pattern matching
            if !pattern_str.starts_with('!') {
                let matches_path = pattern.matches(&path_str)
                    || pattern.matches(&file_name)
                    || pattern.matches(&rel_path);
                if matches_path {
                    // eprintln!("DEBUG: File ignored by default pattern: {} (matched path: {})", pattern_str, rel_path);
                    return true;
                }
            }
        }

        false
    }

    /// Check if a file is allowlisted by any source (.gitignore or config patterns)
    fn is_allowlisted(&self, path: &Path, context: &ProcessingContext) -> bool {
        let path_str = path.to_string_lossy();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        // eprintln!("DEBUG: Checking if file is allowlisted: {}", path_str);

        // Check allowlist patterns from config
        for pattern in &context.input_config.ignore_patterns {
            let pattern_str = pattern.as_str();
            if pattern_str.starts_with('!') {
                let matches_path = pattern.matches(&path_str) || pattern.matches(&file_name);
                // eprintln!("DEBUG: Checking config allowlist pattern: {} against {} -> {}", pattern_str, path_str, matches_path);
                if matches_path {
                    // eprintln!("DEBUG: File allowlisted by config pattern: {}", pattern_str);
                    return true;
                }
            }
        }

        // Check allowlist patterns from .gitignore
        let root_path = &context.repository_info.root_path;
        // eprintln!("DEBUG: Checking .gitignore in: {}", root_path.display());

        // Manual .gitignore parsing for allowlist patterns
        let gitignore_path = root_path.join(".gitignore");
        if gitignore_path.exists() {
            // eprintln!("DEBUG: .gitignore file exists at: {}", gitignore_path.display());
            if let Ok(contents) = std::fs::read_to_string(&gitignore_path) {
                // eprintln!("DEBUG: .gitignore contents: {:?}", contents);

                // Parse .gitignore manually for allowlist patterns
                for line in contents.lines() {
                    let line = line.trim();
                    if line.starts_with('!') && !line.starts_with("!#") {
                        let pattern = &line[1..]; // Remove the '!' prefix
                                                  // eprintln!("DEBUG: Found allowlist pattern in .gitignore: {}", pattern);

                        // Check if this pattern matches our file
                        let path_str = path.to_string_lossy();
                        let file_name = path.file_name().unwrap_or_default().to_string_lossy();

                        // Simple pattern matching (basic glob support)
                        if Self::matches_pattern(pattern, &path_str)
                            || Self::matches_pattern(pattern, &file_name)
                        {
                            // eprintln!("DEBUG: File allowlisted by .gitignore pattern: {}", pattern);
                            return true;
                        }
                    }
                }
            } else {
                // eprintln!("DEBUG: Failed to read .gitignore file");
            }
        } else {
            // eprintln!("DEBUG: .gitignore file does not exist");
        }

        false
    }

    /// Check if a file should be ignored based on .gitignore files
    fn is_ignored_by_gitignore(&self, path: &Path, context: &ProcessingContext) -> bool {
        let root_path = &context.repository_info.root_path;
        // eprintln!("DEBUG: Checking .gitignore for: {} (root: {})", path.display(), root_path.display());

        // Create a gitignore matcher for the root directory
        let (gi, error) = ignore::gitignore::Gitignore::new(root_path);

        if let Some(_e) = error {
            // eprintln!("DEBUG: Failed to create gitignore matcher: {}", e);
            return false;
        }

        // Get the relative path from root to the file
        match path.strip_prefix(root_path) {
            Ok(rel_path) => {
                // eprintln!("DEBUG: Relative path: {}", rel_path.display());
                let matched = gi.matched(rel_path, false);
                
                // eprintln!("DEBUG: .gitignore matched: {:?}, is_ignore: {}", matched, should_ignore);
                matched.is_ignore()
            }
            Err(_) => {
                // eprintln!("DEBUG: Could not make path relative: {} relative to {}", path.display(), root_path.display());
                false
            }
        }
    }

    fn calculate_priority(
        &self,
        rel_path: &str,
        repo_info: &RepositoryInfo,
        context: &ProcessingContext,
    ) -> i32 {
        let mut priority = 0;

        // Apply priority rules
        for rule in &context.processing_config.priority_rules {
            if let Ok(regex) = regex::Regex::new(&rule.pattern) {
                if regex.is_match(rel_path) {
                    priority += rule.score;
                }
            }
        }

        // Apply git boost if available
        if let Some(commit_time) = repo_info.commit_times.get(rel_path) {
            let max_boost = context.input_config.git_boost_max.unwrap_or(100);
            let boost = self.calculate_git_boost(*commit_time, &repo_info.commit_times, max_boost);
            priority += boost;
        }

        priority
    }

    fn calculate_git_boost(
        &self,
        file_time: u64,
        all_times: &HashMap<String, u64>,
        max_boost: i32,
    ) -> i32 {
        if all_times.is_empty() {
            return 0;
        }

        let times: Vec<&u64> = all_times.values().collect();
        let min_time = times.iter().min().map_or(file_time, |&&t| t);
        let max_time = times.iter().max().map_or(file_time, |&&t| t);

        if max_time == min_time {
            return 0; // All files have same timestamp
        }

        let normalized = (file_time - min_time) as f64 / (max_time - min_time) as f64;
        (normalized * max_boost as f64).round() as i32
    }

    fn walk_directory(
        &self,
        dir_path: &Path,
        repo_info: &RepositoryInfo,
        context: &ProcessingContext,
        files: &mut Vec<ProcessedFile>,
    ) -> Result<()> {
        let entries = context.file_system.read_directory(dir_path)?;

        for entry in entries {
            let entry_path = &entry;

            if context.file_system.is_directory(entry_path) {
                // Recurse into subdirectory
                self.walk_directory(entry_path, repo_info, context, files)?;
            } else if context.file_system.is_file(entry_path) {
                // Process file
                if let Ok(processed_file) = self.process_single_file(entry_path, repo_info, context)
                {
                    files.push(processed_file);
                }
            }
        }

        Ok(())
    }

    /// Simple pattern matching for .gitignore patterns
    fn matches_pattern(pattern: &str, text: &str) -> bool {
        // Handle exact matches
        if pattern == text {
            return true;
        }

        // Handle simple glob patterns
        if pattern.contains('*') {
            // Simple wildcard matching
            let pattern_parts: Vec<&str> = pattern.split('*').collect();
            if pattern_parts.len() == 2 {
                let (prefix, suffix) = (pattern_parts[0], pattern_parts[1]);
                return text.starts_with(prefix) && text.ends_with(suffix);
            }
        }

        // Handle patterns ending with /
        if pattern.ends_with('/') && text.starts_with(pattern) {
            return true;
        }

        false
    }
}

/// Content filtering stage - applies size limits and content filtering
pub struct ContentFilteringStage;

impl ProcessingStage for ContentFilteringStage {
    fn process(
        &self,
        files: Vec<ProcessedFile>,
        context: &ProcessingContext,
    ) -> Result<Vec<ProcessedFile>> {
        let start_time = Instant::now();
        let mut filtered_files = Vec::new();
        let mut skipped_count = 0;

        for file in &files {
            let mut file_copy = file.clone();
            if self.should_include_file(&mut file_copy, context) {
                filtered_files.push(file.clone());
            } else {
                skipped_count += 1;
            }
        }

        // Update stats
        if let Ok(mut stats) = context.stats.lock() {
            stats.processing_time_ms += start_time.elapsed().as_millis();
            stats.files_skipped = skipped_count;
        }

        Ok(filtered_files)
    }

    fn name(&self) -> &'static str {
        "ContentFiltering"
    }
}

impl ContentFilteringStage {
    fn should_include_file(&self, file: &mut ProcessedFile, context: &ProcessingContext) -> bool {
        // Check size limits
        if context.output_config.token_mode {
            let token_count = file.get_token_count();
            if let Some(limit) = &context.output_config.token_limit {
                if let Ok(limit_num) = crate::parse_token_limit(limit) {
                    if token_count > limit_num {
                        return false;
                    }
                }
            }
        } else {
            // Byte-based limit
            if let Ok(limit) = context.output_config.max_size.parse::<bytesize::ByteSize>() {
                if file.size_bytes > limit.as_u64() as usize {
                    return false;
                }
            }
        }

        true
    }
}

/// Output formatting stage - applies templates and formatting
pub struct OutputFormattingStage;

impl ProcessingStage for OutputFormattingStage {
    fn process(
        &self,
        files: Vec<ProcessedFile>,
        context: &ProcessingContext,
    ) -> Result<Vec<ProcessedFile>> {
        let start_time = Instant::now();
        let mut formatted_files = Vec::new();

        for mut file in files {
            self.apply_formatting(&mut file, context);
            formatted_files.push(file);
        }

        // Update stats
        if let Ok(mut stats) = context.stats.lock() {
            stats.processing_time_ms += start_time.elapsed().as_millis();
        }

        Ok(formatted_files)
    }

    fn name(&self) -> &'static str {
        "OutputFormatting"
    }
}

impl OutputFormattingStage {
    fn apply_formatting(&self, file: &mut ProcessedFile, context: &ProcessingContext) {
        // Apply line numbers if requested
        if context.output_config.line_numbers {
            file.content = self.add_line_numbers(&file.content);
        }

        // Token counting is handled lazily in the ProcessedFile model
    }

    fn add_line_numbers(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let width = if total_lines == 0 {
            3
        } else {
            std::cmp::max(3, total_lines.to_string().len())
        };

        lines
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:width$} | {}", i + 1, line, width = width))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Processing pipeline orchestrator
pub struct ProcessingPipeline {
    stages: Vec<Box<dyn ProcessingStage>>,
    context: ProcessingContext,
}

impl ProcessingPipeline {
    pub fn new(context: ProcessingContext) -> Self {
        let stages: Vec<Box<dyn ProcessingStage>> = vec![
            Box::new(FileDiscoveryStage::new()),
            Box::new(ContentFilteringStage),
            Box::new(OutputFormattingStage),
        ];

        Self { stages, context }
    }

    pub fn add_stage(&mut self, stage: Box<dyn ProcessingStage>) {
        self.stages.push(stage);
    }

    pub fn process(&self) -> Result<Vec<ProcessedFile>> {
        let mut files: Vec<ProcessedFile> = Vec::new();

        for stage in &self.stages {
            files = stage.process(files, &self.context)?;
        }

        // Final sorting by priority and file_index
        files.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.file_index.cmp(&b.file_index))
        });

        Ok(files)
    }

    pub fn get_stats(&self) -> ProcessingStats {
        self.context.stats.lock().unwrap().clone()
    }
}

/// Pipeline builder for fluent configuration
pub struct ProcessingPipelineBuilder {
    context: ProcessingContext,
    stages: Vec<Box<dyn ProcessingStage>>,
}

impl ProcessingPipelineBuilder {
    pub fn new(context: ProcessingContext) -> Self {
        Self {
            context,
            stages: Vec::new(),
        }
    }

    pub fn add_stage(mut self, stage: Box<dyn ProcessingStage>) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn build(self) -> ProcessingPipeline {
        let mut pipeline = ProcessingPipeline::new(self.context);
        for stage in self.stages {
            pipeline.add_stage(stage);
        }
        pipeline
    }
}
