#[cfg(test)]
mod integration_tests {
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;
    use yek::{config::YekConfig, serialize_repo};

    // Helper function to create test files and directories
    fn setup_test_environment() -> (TempDir, Vec<String>) {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");
        let nested_file = dir1.join("nested.txt");

        fs::create_dir(&dir1).unwrap();
        fs::create_dir(&dir2).unwrap();
        File::create(&file1)
            .unwrap()
            .write_all(b"file1 content")
            .unwrap();
        File::create(&file2)
            .unwrap()
            .write_all(b"file2 content")
            .unwrap();
        File::create(&nested_file)
            .unwrap()
            .write_all(b"nested content")
            .unwrap();

        let paths = vec![
            file1.to_string_lossy().to_string(),
            file2.to_string_lossy().to_string(),
            dir1.to_string_lossy().to_string(),
            dir2.to_string_lossy().to_string(),
        ];
        (temp_dir, paths)
    }

    #[test]
    fn test_mixed_files_and_directories() {
        let (temp_dir, paths) = setup_test_environment();
        let output_dir = temp_dir.path().join("output");
        let config =
            YekConfig::extend_config_with_defaults(paths, output_dir.to_string_lossy().to_string());

        let result = serialize_repo(&config);
        assert!(result.is_ok());
        let (output, files) = result.unwrap();
        assert!(output.contains("file1 content"));
        assert!(output.contains("file2 content"));
        assert!(output.contains("nested content"));
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_only_files() {
        let (temp_dir, paths) = setup_test_environment();
        let output_dir = temp_dir.path().join("output");
        let file_paths = paths[0..2].to_vec(); // Only the files
        let config = YekConfig::extend_config_with_defaults(
            file_paths,
            output_dir.to_string_lossy().to_string(),
        );

        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_only_directories() {
        let (temp_dir, paths) = setup_test_environment();
        let output_dir = temp_dir.path().join("output");
        let dir_paths = paths[2..4].to_vec(); // Only the directories
        let config = YekConfig::extend_config_with_defaults(
            dir_paths,
            output_dir.to_string_lossy().to_string(),
        );

        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nonexistent_paths() {
        let (temp_dir, mut paths) = setup_test_environment();
        let output_dir = temp_dir.path().join("output");
        paths.push("nonexistent_file.txt".to_string());
        paths.push("nonexistent_dir".to_string());
        let config =
            YekConfig::extend_config_with_defaults(paths, output_dir.to_string_lossy().to_string());

        // Should not panic, even with non-existent paths
        let result = serialize_repo(&config);
        assert!(result.is_ok());
        let (output, files) = result.unwrap();
        assert!(output.contains("file1 content"));
        assert!(output.contains("file2 content"));
        assert!(output.contains("nested content"));
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_empty_input_defaults_to_cwd() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("output");
        fs::create_dir(&output_dir).unwrap(); // Ensure output directory exists

        // Create a file in the current directory (which will be the temp_dir)
        let current_dir_file = temp_dir.path().join("current_dir_file.txt");
        File::create(&current_dir_file)
            .unwrap()
            .write_all(b"current dir file content")
            .unwrap();

        // Use the absolute path of the temp_dir as input
        let config = YekConfig::extend_config_with_defaults(
            vec![temp_dir.path().to_string_lossy().to_string()], // Use temp_dir as input
            output_dir.to_string_lossy().to_string(),
        );

        let result = serialize_repo(&config);
        assert!(result.is_ok());
        let (output, files) = result.unwrap();
        assert!(output.contains("current dir file content"));
        assert_eq!(files.len(), 1);

        // No need to change and restore the directory anymore
    }

    #[test]
    fn test_file_as_output_dir_error() {
        let temp_dir = TempDir::new().unwrap();
        let existing_file = temp_dir.path().join("existing_file.txt");
        File::create(&existing_file).unwrap(); // Create a file

        let config = YekConfig {
            input_paths: vec![".".to_string()],
            output_dir: Some(existing_file.to_string_lossy().to_string()),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err()); // Expect an error
    }
    #[test]
    fn test_get_checksum_with_mixed_paths() {
        let (temp_dir, paths) = setup_test_environment();
        let file1 = temp_dir.path().join("file1.txt");
        let dir1 = temp_dir.path().join("dir1");
        // Get checksum with mixed files and directories
        let checksum_mixed = YekConfig::get_checksum(&paths);

        // Get checksum with only files
        let checksum_files = YekConfig::get_checksum(&[file1.to_string_lossy().to_string()]);

        // Get checksum with only directories
        let checksum_dirs = YekConfig::get_checksum(&[dir1.to_string_lossy().to_string()]);

        // Checksums should be different
        assert_ne!(checksum_mixed, checksum_files);
        assert_ne!(checksum_mixed, checksum_dirs);
    }
}
