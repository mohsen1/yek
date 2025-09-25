use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;
use yek::models::{InputConfig, OutputConfig, ProcessedFile, ProcessingConfig, RepositoryInfo};
use yek::pipeline::{
    ContentFilteringStage, FileDiscoveryStage, OutputFormattingStage, ProcessingContext,
    ProcessingPipeline, ProcessingPipelineBuilder, ProcessingStage,
};
use yek::priority::PriorityRule;
use yek::repository::RealFileSystem;

#[cfg(test)]
mod pipeline_tests {
    use super::*;

    fn create_test_context_with_configs(
        input_config: InputConfig,
        output_config: OutputConfig,
        processing_config: ProcessingConfig,
        repository_info: RepositoryInfo,
    ) -> ProcessingContext {
        ProcessingContext::new(
            input_config,
            output_config,
            processing_config,
            repository_info,
            Arc::new(RealFileSystem),
        )
    }

    fn create_baseline_context() -> ProcessingContext {
        create_test_context_with_configs(
            InputConfig::default(),
            OutputConfig::default(),
            ProcessingConfig::default(),
            RepositoryInfo::new(PathBuf::from("/tmp"), false),
        )
    }

    fn input_config_with_paths(paths: Vec<String>) -> InputConfig {
        InputConfig {
            input_paths: paths,
            ignore_patterns: Vec::new(),
            binary_extensions: HashSet::new(),
            max_git_depth: 100,
            git_boost_max: Some(100),
        }
    }

    fn repository_info_for(path: &Path) -> RepositoryInfo {
        RepositoryInfo::new(path.to_path_buf(), false)
    }

    #[test]
    fn test_processing_context_new() {
        let context = create_baseline_context();
        assert!(context.input_config.input_paths.is_empty());
        assert_eq!(context.output_config.max_size, "10MB");
        assert_eq!(context.processing_config.batch_size, 1000);
        assert_eq!(context.repository_info.root_path, PathBuf::from("/tmp"));
        assert!(!context.repository_info.is_git_repo);
    }

    #[test]
    fn test_processing_pipeline_new() {
        let context = create_baseline_context();
        let _pipeline = ProcessingPipeline::new(context);
        // Should not panic
    }

    #[test]
    fn test_processing_pipeline_get_stats() {
        let context = create_baseline_context();
        let pipeline = ProcessingPipeline::new(context);

        let stats = pipeline.get_stats();
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.files_skipped, 0);
    }

    #[test]
    fn test_processing_pipeline_builder_new() {
        let context = create_baseline_context();
        let _builder = ProcessingPipelineBuilder::new(context);
        // Should not panic
    }

    #[test]
    fn test_processing_pipeline_builder_build() {
        let context = create_baseline_context();
        let _pipeline = ProcessingPipelineBuilder::new(context).build();
        // Should not panic
    }

    #[test]
    fn test_file_discovery_stage_process() {
        let stage = FileDiscoveryStage::new();
        let context = create_baseline_context();
        let files = stage.process(vec![], &context).unwrap();
        // Should return files or empty vec, depending on input paths
        // Since input_paths is empty, should return empty
        assert!(files.is_empty());
    }

    #[test]
    fn test_file_discovery_stage_with_files_and_globs() {
        let temp = tempdir().unwrap();
        let base_dir = temp.path();

        fs::write(base_dir.join("include.txt"), "include").unwrap();
        fs::create_dir(base_dir.join("src")).unwrap();
        fs::write(base_dir.join("src/lib.rs"), "fn main() {}").unwrap();
        fs::write(base_dir.join("skip.bin"), [0u8; 4]).unwrap();

        let mut input_config = input_config_with_paths(vec![
            base_dir.join("include.txt").to_string_lossy().to_string(),
            base_dir.join("skip.bin").to_string_lossy().to_string(),
            format!("{}/**/*.rs", base_dir.display()),
        ]);
        input_config.binary_extensions.insert("bin".to_string());

        let context = create_test_context_with_configs(
            input_config,
            OutputConfig::default(),
            ProcessingConfig::default(),
            repository_info_for(base_dir),
        );

        let stage = FileDiscoveryStage::new();
        let files = stage.process(Vec::new(), &context).unwrap();

        let rel_paths: Vec<&str> = files.iter().map(|f| f.rel_path.as_str()).collect();
        assert!(
            rel_paths.iter().any(|path| path.ends_with("include.txt")),
            "expected include.txt in {:?}",
            rel_paths
        );
        assert!(
            rel_paths.iter().any(|path| path.ends_with("src/lib.rs")),
            "expected src/lib.rs in {:?}",
            rel_paths
        );
        assert!(
            !rel_paths.iter().any(|path| path.ends_with("skip.bin")),
            "binary file should be ignored, got {:?}",
            rel_paths
        );
    }

    #[test]
    fn test_file_discovery_stage_applies_priority_rules() {
        let temp = tempdir().unwrap();
        let base_dir = temp.path();

        fs::write(base_dir.join("plain.txt"), "text").unwrap();
        fs::write(base_dir.join("highlight.rs"), "fn main() {}").unwrap();

        let input_config = input_config_with_paths(vec![base_dir.to_string_lossy().to_string()]);

        let processing_config = ProcessingConfig {
            priority_rules: vec![PriorityRule {
                pattern: ".*\\.rs$".to_string(),
                score: 42,
            }],
            ..Default::default()
        };

        let context = create_test_context_with_configs(
            input_config,
            OutputConfig::default(),
            processing_config,
            repository_info_for(base_dir),
        );

        let stage = FileDiscoveryStage::new();
        let files = stage.process(Vec::new(), &context).unwrap();

        let priorities: Vec<(&str, i32)> = files
            .iter()
            .map(|file| (file.rel_path.as_str(), file.priority))
            .collect();

        let rs_priority = priorities
            .iter()
            .find(|(path, _)| path.ends_with(".rs"))
            .unwrap_or_else(|| panic!("expected .rs file in results: {:?}", priorities))
            .1;
        assert_eq!(rs_priority, 42);

        let txt_priority = priorities
            .iter()
            .find(|(path, _)| path.ends_with(".txt"))
            .unwrap_or_else(|| panic!("expected .txt file in results: {:?}", priorities))
            .1;
        assert_eq!(txt_priority, 0);
    }

    #[test]
    fn test_content_filtering_stage_process() {
        let stage = ContentFilteringStage;
        let context = create_baseline_context();
        let file = ProcessedFile::new("test.txt".to_string(), "content".to_string(), 0, 0);
        let files = stage.process(vec![file], &context).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_content_filtering_stage_enforces_byte_limit() {
        let output_config = OutputConfig {
            max_size: "1B".to_string(),
            ..Default::default()
        };

        let context = create_test_context_with_configs(
            InputConfig::default(),
            output_config,
            ProcessingConfig::default(),
            repository_info_for(Path::new("/tmp")),
        );

        let stage = ContentFilteringStage;
        let file = ProcessedFile::new("too_big.txt".into(), "abcd".into(), 0, 0);
        let files = stage.process(vec![file], &context).unwrap();
        assert!(files.is_empty());

        let stats = context.stats.lock().unwrap();
        assert_eq!(stats.files_skipped, 1);
    }

    #[test]
    fn test_content_filtering_stage_enforces_token_limit() {
        let output_config = OutputConfig {
            token_mode: true,
            token_limit: Some("1".to_string()),
            ..Default::default()
        };

        let context = create_test_context_with_configs(
            InputConfig::default(),
            output_config,
            ProcessingConfig::default(),
            repository_info_for(Path::new("/tmp")),
        );

        let stage = ContentFilteringStage;
        let file = ProcessedFile::new("tokens.txt".into(), "hello world token test".into(), 0, 0);
        let files = stage.process(vec![file], &context).unwrap();
        assert!(files.is_empty());

        let stats = context.stats.lock().unwrap();
        assert_eq!(stats.files_skipped, 1);
    }

    #[test]
    fn test_output_formatting_stage_process() {
        let stage = OutputFormattingStage;
        let context = create_baseline_context();
        let file = ProcessedFile::new("test.txt".to_string(), "line1\nline2".to_string(), 0, 0);
        let files = stage.process(vec![file], &context).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_output_formatting_stage_adds_line_numbers() {
        let output_config = OutputConfig {
            line_numbers: true,
            ..Default::default()
        };

        let context = create_test_context_with_configs(
            InputConfig::default(),
            output_config,
            ProcessingConfig::default(),
            repository_info_for(Path::new("/tmp")),
        );

        let stage = OutputFormattingStage;
        let file = ProcessedFile::new("test.txt".to_string(), "first\nsecond".to_string(), 0, 0);
        let files = stage.process(vec![file], &context).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("  1 | first"));
        assert!(files[0].content.contains("  2 | second"));
    }

    #[test]
    fn test_processing_pipeline_process() {
        let context = create_baseline_context();
        let pipeline = ProcessingPipeline::new(context);
        let result = pipeline.process();
        // Should not panic, even if no files are found
        assert!(result.is_ok());
    }
}
