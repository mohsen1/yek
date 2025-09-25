use crate::{
    models::{InputConfig, OutputConfig, ProcessedFile, ProcessingConfig},
    pipeline::ProcessingContext,
};
use anyhow::{anyhow, Result};
use content_inspector::{inspect, ContentType};
use ignore::gitignore::GitignoreBuilder;
use path_slash::PathBufExt;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};
use tracing::debug;

/// Thread-safe file processor that fixes race conditions
pub struct ParallelFileProcessor {
    context: Arc<ProcessingContext>,
    file_counter: Arc<Mutex<HashMap<i32, usize>>>,
}

impl ParallelFileProcessor {
    pub fn new(context: ProcessingContext) -> Self {
        Self {
            context: Arc::new(context),
            file_counter: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Process files in parallel with proper synchronization
    pub fn process_files_parallel(&self, base_path: &Path) -> Result<Vec<ProcessedFile>> {
        let start_time = Instant::now();
        let mut all_processed_files = Vec::new();

        // Expand globs into a list of paths
        let expanded_paths = self.expand_globs(base_path)?;

        // Determine the base directory for relative path calculation
        let base_dir = self.determine_base_dir(base_path, &expanded_paths);

        // Process each expanded path
        for path in expanded_paths {
            if self.context.file_system.is_file(&path) {
                let files = self.process_single_file(&path, &base_dir)?;
                all_processed_files.extend(files);
            } else if self.context.file_system.is_directory(&path) {
                let files = self.process_directory(&path, &base_dir)?;
                all_processed_files.extend(files);
            }
        }

        // Sort final results by priority (asc) then file_index (asc)
        all_processed_files.par_sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.file_index.cmp(&b.file_index))
        });

        if self.context.processing_config.debug {
            debug!(
                "Processed {} files in parallel for base_path: {} in {:?}",
                all_processed_files.len(),
                base_path.display(),
                start_time.elapsed()
            );
        }

        Ok(all_processed_files)
    }

    /// Expand glob patterns into concrete paths
    fn expand_globs(&self, base_path: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut expanded_paths = Vec::new();
        let path_str = base_path.to_string_lossy();

        for entry in glob::glob(&path_str)? {
            match entry {
                Ok(path) => {
                    // Resolve symlinks to prevent issues
                    let resolved_path = if self.context.file_system.is_symlink(&path) {
                        self.context
                            .file_system
                            .resolve_symlink(&path)
                            .unwrap_or(path)
                    } else {
                        path
                    };
                    expanded_paths.push(resolved_path);
                }
                Err(e) => debug!("Glob entry error: {:?}", e),
            }
        }

        Ok(expanded_paths)
    }

    /// Determine the base directory for relative path calculations
    fn determine_base_dir(
        &self,
        base_path: &Path,
        _expanded_paths: &[std::path::PathBuf],
    ) -> std::path::PathBuf {
        let path_str = base_path.to_string_lossy();

        if path_str.contains('*') || path_str.contains('?') {
            // For glob patterns, use current directory to ensure unique paths across different sources
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
        } else if base_path.is_file() {
            // For single files, use the parent directory
            base_path.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            // For directories, use the directory itself
            base_path.to_path_buf()
        }
    }

    /// Process a single file
    fn process_single_file(&self, file_path: &Path, base_dir: &Path) -> Result<Vec<ProcessedFile>> {
        let rel_path = self.normalize_path(file_path, base_dir);

        // Check if file should be ignored
        if self.should_ignore_file(file_path, &rel_path) {
            debug!("Skipping ignored file: {rel_path}");
            return Ok(Vec::new());
        }

        // Read and process file content
        match self.context.file_system.read_file(file_path) {
            Ok(content) => {
                if inspect(&content) == ContentType::BINARY {
                    debug!("Skipping binary file: {rel_path}");
                    Ok(Vec::new())
                } else {
                    let processed_file = self.create_processed_file(&rel_path, &content)?;
                    Ok(vec![processed_file])
                }
            }
            Err(e) => {
                debug!("Failed to read {rel_path}: {e}");
                // Skip files that can't be read instead of failing
                Ok(Vec::new())
            }
        }
    }

    /// Process all files in a directory
    fn process_directory(&self, dir_path: &Path, base_dir: &Path) -> Result<Vec<ProcessedFile>> {
        let mut processed_files = Vec::new();

        // Build gitignore patterns
        let gitignore = self.build_gitignore(dir_path)?;

        // Use parallel processing for directory contents
        let files_to_process: Vec<_> = self.collect_files_to_process(dir_path, &gitignore)?;

        // Process files in parallel with proper synchronization
        let results: Vec<Result<ProcessedFile>> = files_to_process
            .par_iter()
            .map(|(path, rel_path)| self.process_file_with_priority(path, rel_path, base_dir))
            .collect();

        // Filter out errors (e.g., binary files) and collect successful results
        processed_files.extend(results.into_iter().filter_map(|r| r.ok()));

        Ok(processed_files)
    }

    /// Collect all files that need to be processed from a directory
    fn collect_files_to_process(
        &self,
        dir_path: &Path,
        gitignore: &Arc<ignore::gitignore::Gitignore>,
    ) -> Result<Vec<(std::path::PathBuf, String)>> {
        let mut files_to_process = Vec::new();

        // Use ignore's walker for efficient directory traversal
        let mut walk_builder = ignore::WalkBuilder::new(dir_path);
        walk_builder
            .follow_links(false)
            .standard_filters(true)
            .require_git(false);

        let base_dir = dir_path.to_path_buf();
        let gitignore = Arc::clone(gitignore);

        // Use sequential walking instead of parallel to avoid closure issues
        for result in walk_builder.build() {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Only process files
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let path = entry.path().to_path_buf();
            let rel_path = crate::repository::convenience::get_relative_path(&path, &base_dir)
                .unwrap_or_else(|_| path.to_string_lossy().to_string().into());

            // Check gitignore
            if gitignore.matched(&path, false).is_ignore() {
                debug!("Skipping ignored file: {}", rel_path.display());
                continue;
            }

            // Send to processing
            files_to_process.push((path, rel_path.to_string_lossy().to_string()));
        }

        Ok(files_to_process)
    }

    /// Process a single file with priority calculation and thread-safe index assignment
    fn process_file_with_priority(
        &self,
        file_path: &Path,
        rel_path: &str,
        _base_dir: &Path,
    ) -> Result<ProcessedFile> {
        // Read file content
        let content = self.context.file_system.read_file(file_path)?;

        if inspect(&content) == ContentType::BINARY {
            return Err(anyhow!("Binary file: {}", rel_path));
        }

        // Calculate priority with category
        let (priority, category) = self.calculate_priority_with_category(rel_path);

        // Get thread-safe file index
        let file_index = self.get_next_file_index(priority);

        Ok(ProcessedFile::new_with_category(
            rel_path.to_string(),
            String::from_utf8_lossy(&content).to_string(),
            priority,
            file_index,
            category,
        ))
    }

    /// Calculate priority for a file (legacy method for backward compatibility)
    #[allow(dead_code)]
    fn calculate_priority(&self, rel_path: &str) -> i32 {
        let mut priority = 0;

        // Apply priority rules
        for rule in &self.context.processing_config.priority_rules {
            if let Ok(regex) = regex::Regex::new(&rule.pattern) {
                if regex.is_match(rel_path) {
                    priority += rule.score;
                }
            }
        }

        // Apply git boost if available
        if let Some(commit_time) = self.context.repository_info.commit_times.get(rel_path) {
            let max_boost = self.context.input_config.git_boost_max.unwrap_or(100);
            priority += self.calculate_git_boost(
                *commit_time,
                &self.context.repository_info.commit_times,
                max_boost,
            );
        }

        priority
    }

    /// Calculate priority for a file including category-based offset
    fn calculate_priority_with_category(
        &self,
        rel_path: &str,
    ) -> (i32, crate::category::FileCategory) {
        use crate::priority::get_file_priority_with_category;

        // Get base priority from rules and category
        let (mut priority, category) = get_file_priority_with_category(
            rel_path,
            &self.context.processing_config.priority_rules,
            &self.context.processing_config.category_weights,
        );

        // Apply git boost if available
        if let Some(commit_time) = self.context.repository_info.commit_times.get(rel_path) {
            let max_boost = self.context.input_config.git_boost_max.unwrap_or(100);
            priority += self.calculate_git_boost(
                *commit_time,
                &self.context.repository_info.commit_times,
                max_boost,
            );
        }

        (priority, category)
    }

    /// Calculate git boost for a file
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

    /// Get next file index for a priority level in a thread-safe manner
    fn get_next_file_index(&self, priority: i32) -> usize {
        let mut counter = self.file_counter.lock().unwrap();
        let next_index = counter.entry(priority).or_insert(0);
        let index = *next_index;
        *next_index += 1;
        index
    }

    /// Check if a file should be ignored
    fn should_ignore_file(&self, file_path: &Path, _rel_path: &str) -> bool {
        // Check ignore patterns
        let path_str = file_path.to_string_lossy();
        let ignored_by_pattern = self
            .context
            .input_config
            .ignore_patterns
            .iter()
            .any(|pattern| pattern.matches(&path_str));

        // Check binary extensions
        let is_binary = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| self.context.input_config.binary_extensions.contains(ext))
            .unwrap_or(false);

        ignored_by_pattern || is_binary
    }

    /// Build gitignore for a directory
    fn build_gitignore(&self, dir_path: &Path) -> Result<Arc<ignore::gitignore::Gitignore>> {
        let mut gitignore_builder = GitignoreBuilder::new(dir_path);

        // Add custom patterns
        for pattern in &self.context.input_config.ignore_patterns {
            gitignore_builder.add_line(None, &pattern.to_string())?;
        }

        // Add .gitignore file if it exists
        let gitignore_file = dir_path.join(".gitignore");
        if self.context.file_system.path_exists(&gitignore_file) {
            gitignore_builder.add(&gitignore_file);
        }

        Ok(Arc::new(gitignore_builder.build()?))
    }

    /// Create a processed file with proper metadata
    fn create_processed_file(&self, rel_path: &str, content: &[u8]) -> Result<ProcessedFile> {
        let (priority, category) = self.calculate_priority_with_category(rel_path);
        let file_index = self.get_next_file_index(priority);

        Ok(ProcessedFile::new_with_category(
            rel_path.to_string(),
            String::from_utf8_lossy(content).to_string(),
            priority,
            file_index,
            category,
        ))
    }

    /// Normalize path to relative, slash-normalized form
    fn normalize_path(&self, path: &Path, base: &Path) -> String {
        path.strip_prefix(base)
            .unwrap_or(path)
            .to_path_buf()
            .to_slash()
            .unwrap_or_default()
            .to_string()
    }
}

/// Create a relative, slash-normalized path
pub fn normalize_path(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_path_buf()
        .to_slash()
        .unwrap_or_default()
        .to_string()
}

/// Legacy function for backward compatibility - delegates to new implementation
pub fn process_files_parallel(
    base_path: &Path,
    config: &crate::config::YekConfig,
    _boost_map: &HashMap<String, i32>,
) -> Result<Vec<ProcessedFile>> {
    // This is a temporary bridge - in the final implementation,
    // this would be replaced with the new pipeline-based approach
    let processor = ParallelFileProcessor::new(ProcessingContext::new(
        InputConfig {
            input_paths: vec![], // Not used in this context
            ignore_patterns: config
                .ignore_patterns
                .iter()
                .map(|s| glob::Pattern::new(s).unwrap())
                .collect(),
            binary_extensions: config.binary_extensions.iter().cloned().collect(),
            max_git_depth: config.max_git_depth,
            git_boost_max: config.git_boost_max,
        },
        OutputConfig::default(), // TODO: Convert from YekConfig
        ProcessingConfig {
            priority_rules: config.priority_rules.clone(),
            category_weights: config.category_weights.clone().unwrap_or_default(),
            debug: config.debug,
            parallel: true,
            max_threads: None,
            memory_limit_mb: None,
            batch_size: 1000,
        },
        crate::models::RepositoryInfo::new(base_path.to_path_buf(), false), // TODO: Proper repo info
        Arc::new(crate::repository::RealFileSystem),
    ));

    processor.process_files_parallel(base_path)
}
