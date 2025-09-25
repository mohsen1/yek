use yek::models::{FilePriority, ProcessedFile, ProcessingStats};

#[cfg(test)]
mod models_tests {
    use super::*;

    #[test]
    fn test_processed_file_new() {
        let file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);
        assert_eq!(file.rel_path, "test.txt");
        assert_eq!(file.content, "Hello world");
        assert_eq!(file.priority, 10);
        assert_eq!(file.file_index, 0);
        assert_eq!(file.size_bytes, 11); // "Hello world".len()
        assert!(file.token_count.get().is_none());
        assert!(file.formatted_content.is_none());
        // Category should be automatically determined from file path
        assert_eq!(file.category, yek::category::FileCategory::Documentation); // .txt files are Documentation
    }

    #[test]
    fn test_processed_file_clone() {
        let mut file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);
        // Set token count
        file.token_count.set(5).unwrap();
        file.formatted_content = Some("formatted".to_string());

        let cloned = file.clone();
        assert_eq!(cloned.rel_path, file.rel_path);
        assert_eq!(cloned.content, file.content);
        assert_eq!(cloned.priority, file.priority);
        assert_eq!(cloned.file_index, file.file_index);
        assert_eq!(cloned.size_bytes, file.size_bytes);
        // Clone creates a new OnceLock, so token_count is empty
        assert!(cloned.token_count.get().is_none());
        assert_eq!(cloned.formatted_content, file.formatted_content);
        // Category should be preserved in clone
        assert_eq!(cloned.category, file.category);
    }

    #[test]
    fn test_processed_file_new_with_category() {
        use yek::category::FileCategory;

        let file = ProcessedFile::new_with_category(
            "some_file.data".to_string(),
            "Hello world".to_string(),
            10,
            0,
            FileCategory::Source,
        );
        assert_eq!(file.rel_path, "some_file.data");
        assert_eq!(file.content, "Hello world");
        assert_eq!(file.priority, 10);
        assert_eq!(file.file_index, 0);
        assert_eq!(file.size_bytes, 11); // "Hello world".len()
        assert!(file.token_count.get().is_none());
        assert!(file.formatted_content.is_none());
        // Category should be explicitly set to Source
        assert_eq!(file.category, FileCategory::Source);
    }

    #[test]
    fn test_processed_file_category_detection() {
        use yek::category::FileCategory;

        // Test various file types to ensure category detection works
        let source_file =
            ProcessedFile::new("src/main.rs".to_string(), "fn main() {}".to_string(), 10, 0);
        assert_eq!(source_file.category, FileCategory::Source);

        let test_file = ProcessedFile::new(
            "tests/unit.test.js".to_string(),
            "test()".to_string(),
            10,
            0,
        );
        assert_eq!(test_file.category, FileCategory::Test);

        let config_file = ProcessedFile::new("package.json".to_string(), "{}".to_string(), 10, 0);
        assert_eq!(config_file.category, FileCategory::Configuration);

        let doc_file = ProcessedFile::new("README.md".to_string(), "# Title".to_string(), 10, 0);
        assert_eq!(doc_file.category, FileCategory::Documentation);

        let other_file =
            ProcessedFile::new("image.png".to_string(), "binary data".to_string(), 10, 0);
        assert_eq!(other_file.category, FileCategory::Other);
    }

    #[test]
    fn test_processed_file_get_token_count_lazy() {
        let file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);

        // First call should compute and cache
        let count1 = file.get_token_count();
        assert!(count1 > 0); // Should have computed some token count
        assert_eq!(file.token_count.get(), Some(&count1));

        // Second call should return cached value
        let count2 = file.get_token_count();
        assert_eq!(count1, count2);
    }

    #[test]
    fn test_processed_file_get_formatted_content_no_line_numbers() {
        let file = ProcessedFile::new("test.txt".to_string(), "Hello\nworld".to_string(), 10, 0);

        let content = file.get_formatted_content(false);
        assert_eq!(content, "Hello\nworld");
    }

    #[test]
    fn test_processed_file_get_formatted_content_with_line_numbers() {
        let mut file =
            ProcessedFile::new("test.txt".to_string(), "Hello\nworld".to_string(), 10, 0);
        file.formatted_content = Some("1 | Hello\n2 | world".to_string());

        let content = file.get_formatted_content(true);
        assert_eq!(content, "1 | Hello\n2 | world");
    }

    #[test]
    fn test_processed_file_get_size_bytes_mode() {
        let file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);

        let size = file.get_size(false, false); // bytes mode, no line numbers
        assert_eq!(size, 11); // "Hello world".len()
    }

    #[test]
    fn test_processed_file_get_size_token_mode() {
        let file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);
        file.token_count.set(5).unwrap();

        let size = file.get_size(true, false);
        assert_eq!(size, 5);
    }

    #[test]
    fn test_processed_file_get_size_with_line_numbers() {
        let mut file =
            ProcessedFile::new("test.txt".to_string(), "Hello\nworld".to_string(), 10, 0);
        file.formatted_content = Some("1 | Hello\n2 | world".to_string());

        let size = file.get_size(false, true);
        assert_eq!(size, 19); // Length of "1 | Hello\n2 | world"
    }

    #[test]
    fn test_processed_file_exceeds_limit_bytes() {
        let file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);

        assert!(!file.exceeds_limit(20, false, false)); // 11 < 20
        assert!(file.exceeds_limit(5, false, false)); // 11 > 5
    }

    #[test]
    fn test_processed_file_exceeds_limit_tokens() {
        let file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);
        file.token_count.set(10).unwrap();

        assert!(!file.exceeds_limit(15, true, false)); // 10 < 15
        assert!(file.exceeds_limit(5, true, false)); // 10 > 5
    }

    #[test]
    fn test_processed_file_clear_caches() {
        let mut file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);
        file.token_count.set(5).unwrap();
        file.formatted_content = Some("formatted".to_string());

        file.clear_caches();
        assert!(file.token_count.get().is_none());
        assert!(file.formatted_content.is_none());
    }

    #[test]
    fn test_file_priority_new() {
        let priority = FilePriority::new(10, 5);
        assert_eq!(priority.rule_priority, 10);
        assert_eq!(priority.git_boost, 5);
        assert_eq!(priority.combined, 15);
    }

    #[test]
    fn test_processing_stats_new() {
        let stats = ProcessingStats::new();
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.files_skipped, 0);
        assert_eq!(stats.bytes_processed, 0);
        assert_eq!(stats.tokens_processed, 0);
        assert_eq!(stats.processing_time_ms, 0);
        assert_eq!(stats.memory_usage_bytes, 0);
        assert_eq!(stats.cache_hit_rate, 0.0);
    }

    #[test]
    fn test_processing_stats_add_file() {
        let mut stats = ProcessingStats::new();
        let file = ProcessedFile::new("test.txt".to_string(), "Hello world".to_string(), 10, 0);
        file.token_count.set(5).unwrap();

        stats.add_file(&file, false);
        assert_eq!(stats.files_processed, 1);
        assert_eq!(stats.bytes_processed, 11);
        assert_eq!(stats.tokens_processed, 5);
    }

    #[test]
    fn test_processing_stats_add_skipped_file() {
        let mut stats = ProcessingStats::new();

        stats.add_skipped_file(100);
        assert_eq!(stats.files_skipped, 1);
        assert_eq!(stats.bytes_processed, 100);
    }
}
