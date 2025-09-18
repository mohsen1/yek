#[cfg(test)]
mod extra_tests {
    use std::collections::HashMap;
    use std::fs;
    use std::io::Write;

    use assert_cmd::Command;
    use tempfile::tempdir;
    use yek::{
        concat_files,
        config::YekConfig,
        is_text_file,
        parallel::process_files_parallel,
        priority::{compute_recentness_boost, get_file_priority},
        serialize_repo,
    };

    // Test that concatenating an empty slice of ProcessedFiles produces an empty string.
    #[test]
    fn test_empty_concat_files() {
        let config =
            YekConfig::extend_config_with_defaults(vec![".".to_string()], "output".to_string());
        let output = concat_files(&[], &config).unwrap();
        assert_eq!(output, "");
    }

    // Test is_text_file on an empty file, which should be considered text.
    #[test]
    fn test_is_text_file_empty_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        fs::File::create(&file_path).unwrap();
        let result = is_text_file(&file_path, &[]).unwrap();
        assert!(result, "Empty file should be considered text");
    }

    // Test get_file_priority with no rules returns 0.
    #[test]
    fn test_get_file_priority_no_rules() {
        let rules = Vec::new();
        let priority = get_file_priority("nofile.xyz", &rules);
        assert_eq!(priority, 0);
    }

    // Test compute_recentness_boost when all timestamps are identical.
    #[test]
    fn test_compute_recentness_boost_zero_range() {
        let mut commit_times = HashMap::new();
        commit_times.insert("file1.txt".to_string(), 1000);
        commit_times.insert("file2.txt".to_string(), 1000);
        let boosts = compute_recentness_boost(&commit_times, 50);
        // When all times are same, boost should be 0 for all files.
        assert_eq!(boosts.get("file1.txt"), Some(&0));
        assert_eq!(boosts.get("file2.txt"), Some(&0));
    }

    // Test that ensure_output_dir returns an empty string when stream is true.
    #[test]
    fn test_ensure_output_dir_streaming() {
        let config = YekConfig {
            stream: true,
            ..YekConfig::default()
        };
        let output_dir = config.ensure_output_dir().unwrap();
        assert_eq!(output_dir, "");
    }

    // Test serialize_repo when given a non-existent input directory.
    #[test]
    fn test_serialize_repo_nonexistent_input_dir() {
        let config = YekConfig::extend_config_with_defaults(
            vec!["nonexistent_directory_xyz".to_string()],
            "output".to_string(),
        );
        let (_output, files) = serialize_repo(&config).unwrap();
        // Should yield no processed files for non-existent directory
        assert_eq!(
            files.len(),
            0,
            "No files should be processed for a non-existent directory"
        );
    }

    // Test that warnings are displayed for non-existent paths by capturing stderr.
    #[test]
    fn test_warning_for_nonexistent_paths() {
        // Run yek with a non-existent path and capture stderr
        let output = Command::cargo_bin("yek")
            .expect("Failed to find yek binary")
            .arg("definitely_nonexistent_path_12345")
            .output()
            .expect("Failed to execute yek");

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should contain both warnings
        assert!(stderr.contains("Warning: Path 'definitely_nonexistent_path_12345' does not exist"));
        assert!(stderr.contains("Warning: No files were processed. All specified paths were non-existent or contained no valid files."));
    }

    // Test process_files_parallel with an empty directory.
    #[test]
    fn test_process_files_parallel_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let config = YekConfig::extend_config_with_defaults(
            vec![temp_dir.path().to_string_lossy().to_string()],
            "output".to_string(),
        );
        let boosts = HashMap::new();
        let result = process_files_parallel(temp_dir.path(), &config, &boosts)
            .expect("process_files_parallel should not error on an empty directory");
        assert_eq!(
            result.len(),
            0,
            "No files should be processed in an empty directory"
        );
    }

    // Test is_text_file on a file that contains a mix of text and a null byte.
    #[test]
    fn test_is_text_file_mixed_content_case() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("mixed.txt");
        let mut file = fs::File::create(&file_path).unwrap();
        // Write some text with an embedded null byte.
        file.write_all(b"Hello, world!\0This is binary?").unwrap();
        let result = is_text_file(&file_path, &[]).unwrap();
        assert!(
            !result,
            "File with a null byte should be detected as binary"
        );
    }
}
