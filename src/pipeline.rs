use crate::{
    models::{
        InputConfig, OutputConfig, ProcessedFile, ProcessingConfig, ProcessingStats, RepositoryInfo,
    },
    repository::{FileSystem, GitOperations, RepositoryFactory},
};
use anyhow::{anyhow, Result};
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
    git_operations: Option<Arc<dyn GitOperations>>,
}

impl FileDiscoveryStage {
    pub fn new(git_operations: Option<Arc<dyn GitOperations>>) -> Self {
        Self {
            repository_factory: RepositoryFactory::new(),
            git_operations,
        }
    }
}

impl ProcessingStage for FileDiscoveryStage {
    fn process(
        &self,
        _files: Vec<ProcessedFile>,
        context: &ProcessingContext,
    ) -> Result<Vec<ProcessedFile>> {
        let start_time = Instant::now();
        let mut discovered_files = Vec::new();

        for input_path in &context.input_config.input_paths {
            let path = Path::new(input_path);

            // Create repository info for this path
            let repo_info = self
                .repository_factory
                .create_repository_info(path, &context.input_config)?;

            // Discover files in this path
            let files = self.discover_files_in_path(path, &repo_info, context)?;
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
    fn discover_files_in_path(
        &self,
        path: &Path,
        repo_info: &RepositoryInfo,
        context: &ProcessingContext,
    ) -> Result<Vec<ProcessedFile>> {
        let mut files = Vec::new();

        if context.file_system.is_file(path) {
            // Single file
            if let Ok(processed_file) = self.process_single_file(path, repo_info, context) {
                files.push(processed_file);
            }
        } else if context.file_system.is_directory(path) {
            // Directory - walk recursively
            self.walk_directory(path, repo_info, context, &mut files)?;
        }

        Ok(files)
    }

    fn process_single_file(
        &self,
        file_path: &Path,
        repo_info: &RepositoryInfo,
        context: &ProcessingContext,
    ) -> Result<ProcessedFile> {
        // Check if file should be ignored
        if self.should_ignore_file(file_path, context) {
            return Err(anyhow!("File ignored: {}", file_path.display()));
        }

        // Read file content
        let content = crate::repository::convenience::read_file_content_safe(
            file_path,
            &*context.file_system,
        )?;

        // Create processed file
        let rel_path =
            crate::repository::convenience::get_relative_path(file_path, &repo_info.root_path)?
                .to_string_lossy()
                .to_string();

        // Calculate priority
        let priority = self.calculate_priority(&rel_path, repo_info, context);

        Ok(ProcessedFile::new(rel_path, content, priority, 0))
    }

    fn should_ignore_file(&self, path: &Path, context: &ProcessingContext) -> bool {
        let path_str = path.to_string_lossy();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();

        // Check ignore patterns - try both full path and filename
        let ignored_by_pattern = context
            .input_config
            .ignore_patterns
            .iter()
            .any(|pattern| pattern.matches(&path_str) || pattern.matches(&file_name));

        // Check binary extensions
        let is_binary = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| context.input_config.binary_extensions.contains(ext))
            .unwrap_or(false);

        ignored_by_pattern || is_binary
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
    git_operations: Option<Arc<dyn GitOperations>>,
}

impl ProcessingPipeline {
    pub fn new(context: ProcessingContext, git_operations: Option<Arc<dyn GitOperations>>) -> Self {
        let mut stages: Vec<Box<dyn ProcessingStage>> = Vec::new();

        // Add default stages
        stages.push(Box::new(FileDiscoveryStage::new(git_operations.clone())));
        stages.push(Box::new(ContentFilteringStage));
        stages.push(Box::new(OutputFormattingStage));

        Self {
            stages,
            context,
            git_operations,
        }
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
    git_operations: Option<Arc<dyn GitOperations>>,
}

impl ProcessingPipelineBuilder {
    pub fn new(context: ProcessingContext, git_operations: Option<Arc<dyn GitOperations>>) -> Self {
        Self {
            context,
            stages: Vec::new(),
            git_operations,
        }
    }

    pub fn add_stage(mut self, stage: Box<dyn ProcessingStage>) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn build(self) -> ProcessingPipeline {
        let mut pipeline = ProcessingPipeline::new(self.context, self.git_operations);
        for stage in self.stages {
            pipeline.add_stage(stage);
        }
        pipeline
    }
}
