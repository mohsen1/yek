#[cfg(test)]
mod lib_tests {
    use tempfile::tempdir;
    use yek::config::FullYekConfig;
    use yek::priority::PriorityRule;
    use yek::serialize_repo;

    fn create_test_config(input_dirs: Vec<String>) -> FullYekConfig {
        FullYekConfig {
            input_dirs,
            max_size: "10MB".to_string(),
            tokens: "".to_string(),
            debug: false,
            output_dir: std::env::temp_dir().to_string_lossy().to_string(),
            ignore_patterns: vec!["*.log".to_string()],
            priority_rules: vec![PriorityRule {
                pattern: "src/.*\\.rs".to_string(),
                score: 100,
            }],
            binary_extensions: vec!["bin".to_string()],
            stream: false,
            token_mode: false,
            output_file_full_path: "output.txt".to_string(),
            git_boost_max: 100,
        }
    }

    #[test]
    fn test_serialize_repo_empty_dir() {
        let temp_dir = tempdir().unwrap();
        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_repo_with_files() {
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_repo_multiple_dirs() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        std::fs::write(dir1.path().join("test1.txt"), "content1").unwrap();
        std::fs::write(dir2.path().join("test2.txt"), "content2").unwrap();

        let config = create_test_config(vec![
            dir1.path().to_string_lossy().to_string(),
            dir2.path().to_string_lossy().to_string(),
        ]);

        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_repo_with_git() {
        let temp_dir = tempdir().unwrap();

        // Initialize git repo
        std::process::Command::new("git")
            .args(&["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Create and commit a file
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();
        std::process::Command::new("git")
            .args(&["add", "test.txt"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(&["commit", "-m", "test commit"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }
}
