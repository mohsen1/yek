use std::path::PathBuf;
use yek::models::{InputConfig, OutputConfig, ProcessedFile, ProcessingConfig, RepositoryInfo};
use yek::pipeline::{ProcessingContext, ProcessingPipeline, ProcessingPipelineBuilder};
use yek::repository::RealFileSystem;
use std::sync::Arc;

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
        let pipeline = ProcessingPipeline::new(context);
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
        let builder = ProcessingPipelineBuilder::new(context);
        // Should not panic
    }

    #[test]
    fn test_processing_pipeline_builder_build() {
        let context = create_test_context();
        let pipeline = ProcessingPipelineBuilder::new(context).build();
        // Should not panic
    }
}