#[cfg(test)]
mod symlink_tests {
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;
    use yek::{config::YekConfig, parallel::process_files_parallel};

    #[cfg(unix)]
    #[test]
    fn test_symlink_is_skipped() {
        // Create a temporary directory.
        let temp_dir = tempdir().expect("failed to create temp dir");
        let base_path = temp_dir.path();

        // Create a regular file.
        let regular_file = base_path.join("regular.txt");
        fs::write(&regular_file, "hello").expect("failed to write regular file");

        // Create a symlink pointing to the regular file.
        let symlink_file = base_path.join("symlink.txt");
        std::os::unix::fs::symlink(&regular_file, &symlink_file).expect("failed to create symlink");

        // Build a default configuration.
        let config = YekConfig::extend_config_with_defaults(
            vec![base_path.to_string_lossy().to_string()],
            ".".to_string(),
        );
        let boost_map = HashMap::new();
        let processed =
            process_files_parallel(base_path, &config, &boost_map).expect("processing failed");

        // Collect the relative paths of processed files.
        let files: Vec<_> = processed.into_iter().map(|pf| pf.rel_path).collect();

        // The regular file should be processed and the symlink should be skipped.
        assert!(
            files.contains(&"regular.txt".to_string()),
            "Expected regular.txt to be processed"
        );
        assert!(
            !files.contains(&"symlink.txt".to_string()),
            "Expected symlink.txt to be skipped"
        );
    }

    // For non-unix systems, we skip the symlink test.
    #[cfg(not(unix))]
    #[test]
    fn test_symlink_skip_not_applicable() {
        eprintln!("Symlink test is not applicable on non-Unix platforms.");
    }
}
