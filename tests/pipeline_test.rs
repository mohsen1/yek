use std::path::PathBuf;
use std::sync::Arc;
use yek::models::{InputConfig, OutputConfig, ProcessedFile, ProcessingConfig, RepositoryInfo};
use yek::pipeline::{
    ContentFilteringStage, FileDiscoveryStage, OutputFormattingStage, ProcessingContext,
    ProcessingPipeline, ProcessingPipelineBuilder, ProcessingStage,
};
use yek::repository::RealFileSystem;

#[cfg(test)]
mod pipeline_tests {
    use super::*;

    fn create_test_context() -> ProcessingContext {
        let input_config = InputConfig::default();
        let output_config = OutputConfig::default();
        let processing_config = ProcessingConfig::default();
        let repository_info = RepositoryInfo::new(PathBuf::from("/tmp"), false);
        let file_system = Arc::new(RealFileSystem);

        ProcessingContext::new(
            input_config,
            output_config,
            processing_config,
            repository_info,
            file_system,
        )
    }

    #[test]
    fn test_processing_context_new() {
        let context = create_test_context();
        assert!(context.input_config.input_paths.is_empty());
        assert_eq!(context.output_config.max_size, "10MB");
        assert_eq!(context.processing_config.batch_size, 1000);
        assert_eq!(context.repository_info.root_path, PathBuf::from("/tmp"));
        assert!(!context.repository_info.is_git_repo);
    }

    #[test]
    fn test_processing_pipeline_new() {
        let context = create_test_context();
        let _pipeline = ProcessingPipeline::new(context);
        // Should not panic
    }

    #[test]
    fn test_processing_pipeline_get_stats() {
        let context = create_test_context();
        let pipeline = ProcessingPipeline::new(context);

        let stats = pipeline.get_stats();
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.files_skipped, 0);
    }

    #[test]
    fn test_processing_pipeline_builder_new() {
        let context = create_test_context();
        let _builder = ProcessingPipelineBuilder::new(context);
        // Should not panic
    }

    #[test]
    fn test_processing_pipeline_builder_build() {
        let context = create_test_context();
        let _pipeline = ProcessingPipelineBuilder::new(context).build();
        // Should not panic
    }

    #[test]
    fn test_file_discovery_stage_process() {
        let stage = FileDiscoveryStage::new();
        let context = create_test_context();
        let files = stage.process(vec![], &context).unwrap();
        // Should return files or empty vec, depending on input paths
        // Since input_paths is empty, should return empty
        assert!(files.is_empty());
    }

    #[test]
    fn test_content_filtering_stage_process() {
        let stage = ContentFilteringStage;
        let context = create_test_context();
        let file = ProcessedFile::new("test.txt".to_string(), "content".to_string(), 0, 0);
        let files = stage.process(vec![file], &context).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_output_formatting_stage_process() {
        let stage = OutputFormattingStage;
        let context = create_test_context();
        let file = ProcessedFile::new("test.txt".to_string(), "line1\nline2".to_string(), 0, 0);
        let files = stage.process(vec![file], &context).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_processing_pipeline_process() {
        let context = create_test_context();
        let pipeline = ProcessingPipeline::new(context);
        let result = pipeline.process();
        // Should not panic, even if no files are found
        assert!(result.is_ok());
    }
}
