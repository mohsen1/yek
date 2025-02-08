#[cfg(test)]
mod lib_tests {
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    use tracing_subscriber::{EnvFilter, FmtSubscriber};
    use yek::config::YekConfig;
    use yek::is_text_file;
    use yek::priority::PriorityRule;
    use yek::serialize_repo;

    // Initialize tracing subscriber for tests
    fn init_tracing() {
        let _ = FmtSubscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    }

    fn create_test_config(input_dirs: Vec<String>) -> YekConfig {
        let mut config = YekConfig::extend_config_with_defaults(
            input_dirs,
            std::env::temp_dir().to_string_lossy().to_string(),
        );
        config.ignore_patterns = vec!["*.log".to_string()];
        config.priority_rules = vec![PriorityRule {
            pattern: "src/.*\\.rs".to_string(),
            score: 100,
        }];
        config.binary_extensions = vec!["bin".to_string()];
        config.output_template = ">>>> FILE_PATH\nFILE_CONTENT".to_string();
        config
    }

    #[test]
    fn test_serialize_repo_empty_dir() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_repo_with_files() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_repo_multiple_dirs() {
        init_tracing();
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
        init_tracing();
        let temp_dir = tempdir().unwrap();

        // Initialize git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Create and commit a file
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();
        std::process::Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "test commit"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_text_file_with_extension() {
        let temp_dir = tempdir().unwrap();
        let text_file = temp_dir.path().join("test.txt");
        let binary_file = temp_dir.path().join("test.bin");

        fs::write(&text_file, "This is a text file.").unwrap();
        fs::write(&binary_file, b"\x00\x01\x02\x03").unwrap();

        assert!(is_text_file(&text_file, &[]).unwrap());
        assert!(!is_text_file(&binary_file, &[]).unwrap());

        // Test with a custom binary extension
        let custom_binary_file = temp_dir.path().join("test.custom");
        fs::write(&custom_binary_file, "This is a text file.").unwrap();
        assert!(!is_text_file(&custom_binary_file, &["custom".to_string()]).unwrap());
    }

    #[test]
    fn test_is_text_file_no_extension() {
        let dir = tempdir().unwrap();
        let text_file = dir.path().join("text_no_ext");
        let binary_file = dir.path().join("binary_no_ext");

        fs::write(&text_file, "This is text.").unwrap();
        fs::write(&binary_file, [0, 1, 2, 3, 4, 5]).unwrap(); // Binary content

        assert!(is_text_file(&text_file, &[]).unwrap());
        assert!(!is_text_file(&binary_file, &[]).unwrap());
    }

    #[test]
    fn test_is_text_file_empty_file() {
        let dir = tempdir().unwrap();
        let empty_file = dir.path().join("empty");

        fs::File::create(&empty_file).unwrap();

        assert!(is_text_file(&empty_file, &[]).unwrap()); // Empty file is considered text
    }

    #[test]
    fn test_is_text_file_with_user_binary_extensions() {
        let dir = tempdir().unwrap();
        let custom_bin_file = dir.path().join("data.dat");

        fs::write(&custom_bin_file, "binary data").unwrap();

        assert!(
            !is_text_file(&custom_bin_file, &["dat".to_string()]).unwrap(),
            "Custom binary extension should be detected as binary"
        );
    }

    #[test]
    fn test_is_text_file_mixed_content() {
        let dir = tempdir().unwrap();
        let mixed_file = dir.path().join("mixed.xyz");

        // Create a file with mostly text but one null byte
        let mut file = fs::File::create(&mixed_file).unwrap();
        file.write_all(b"This is mostly text.\0But with a null byte.")
            .unwrap();

        assert!(!is_text_file(&mixed_file, &[]).unwrap());
    }

    #[test]
    fn test_is_text_file_utf8_content() {
        let dir = tempdir().unwrap();
        let utf8_file = dir.path().join("utf8.txt");

        fs::write(&utf8_file, "こんにちは世界").unwrap(); // Japanese characters

        assert!(is_text_file(&utf8_file, &[]).unwrap());
    }

    #[test]
    fn test_is_text_file_large_text_file() {
        let dir = tempdir().unwrap();
        let large_text_file = dir.path().join("large.txt");

        // Create a 1MB text file
        let content = "a".repeat(1024 * 1024);
        fs::write(&large_text_file, &content).unwrap();

        assert!(is_text_file(&large_text_file, &[]).unwrap());
    }

    #[test]
    fn test_is_text_file_with_shebang() {
        let dir = tempdir().unwrap();
        let script_file = dir.path().join("script.sh");

        // Write a shebang as the first line
        fs::write(&script_file, "#!/bin/bash\necho 'Hello'").unwrap();

        assert!(is_text_file(&script_file, &[]).unwrap());
    }

    #[test]
    fn test_serialize_repo_json_output() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;
        assert!(output_string.contains(r#""filename": "test.txt""#));
        assert!(output_string.contains(r#""content": "test content""#));
    }

    #[test]
    fn test_serialize_repo_template_output() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.output_template =
            "Custom template:\nPath: FILE_PATH\nContent: FILE_CONTENT".to_string();
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;
        assert!(output_string.contains("Custom template:"));
        assert!(output_string.contains("Path: test.txt"));
        assert!(output_string.contains("Content: test content"));
    }
}
