#[cfg(test)]
mod lib_tests {
    use std::fs;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    use yek::{
        concat_files, config::YekConfig, count_tokens, is_text_file, parallel::ProcessedFile,
        parse_token_limit, priority::PriorityRule, serialize_repo,
    };

    #[cfg(unix)]
    fn make_unreadable(path: &std::path::Path) -> std::io::Result<()> {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(path, permissions)
    }

    #[cfg(not(unix))]
    fn make_unreadable(_path: &std::path::Path) -> std::io::Result<()> {
        // On Windows, we can't easily make files unreadable in the same way
        // Skip this test functionality on Windows
        Ok(())
    }

    #[cfg(unix)]
    fn make_readable(path: &std::path::Path) -> std::io::Result<()> {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(path, permissions)
    }

    #[cfg(not(unix))]
    fn make_readable(_path: &std::path::Path) -> std::io::Result<()> {
        // On Windows, files are readable by default
        Ok(())
    }

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
        config.output_template = Some(">>>> FILE_PATH\nFILE_CONTENT".to_string());
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

    // Output format tests
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
        assert!(output_string.contains(r##""content": "test content"##));
    }

    #[test]
    fn test_serialize_repo_template_output() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.output_template =
            Some("Custom template:\nPath: FILE_PATH\nContent: FILE_CONTENT".to_string());
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;
        assert!(output_string.contains("Custom template:"));
        assert!(output_string.contains("Path: test.txt"));
        assert!(output_string.contains("Content: test content"));
    }

    #[test]
    fn test_serialize_repo_json_output_multiple_files() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;
        assert!(output_string.contains(r#""filename": "file1.txt""#));
        assert!(output_string.contains(r##""content": "content1"##));
        assert!(output_string.contains(r#""filename": "file2.txt""#));
        assert!(output_string.contains(r##""content": "content2"##));
    }

    #[test]
    fn test_serialize_repo_template_output_no_files() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;
        assert_eq!(output_string, ""); // Should be empty string when no files
    }

    #[test]
    fn test_serialize_repo_json_output_no_files() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;
        assert_eq!(output_string, "[]"); // Should be empty JSON array when no files
    }

    #[test]
    fn test_serialize_repo_template_output_special_chars() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let file_path = "file with spaces and ünicöde.txt";
        let file_content = "content with <special> & \"chars\"\nand newlines";
        std::fs::write(temp_dir.path().join(file_path), file_content).unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.output_template = Some("Path: FILE_PATH\nContent:\nFILE_CONTENT".to_string());
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;

        assert!(output_string.contains(&format!("Path: {}", file_path)));
        assert!(output_string.contains(&format!("Content:\n{}", file_content)));
    }

    #[test]
    fn test_serialize_repo_json_output_special_chars() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let file_path = "file with spaces and ünicöde.txt";
        let file_content = "content with <special> & \"chars\"\nand newlines";
        std::fs::write(temp_dir.path().join(file_path), file_content).unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;

        assert!(output_string.contains(r#""filename": "file with spaces and ünicöde.txt""#));
        assert!(output_string
            .contains(r##""content": "content with <special> & \"chars\"\nand newlines"##));
    }

    #[test]
    fn test_serialize_repo_template_backslash_n_replace() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.output_template = Some("Path: FILE_PATH\\nContent: FILE_CONTENT".to_string()); // Using literal "\\n"
        let result = serialize_repo(&config).unwrap();
        let output_string = result.0;
        assert!(output_string.contains("Path: test.txt\\nContent: test content")); // Should not replace "\\n" literally

        let mut config_replace =
            create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config_replace.output_template =
            Some("Path: FILE_PATH\\\\nContent: FILE_CONTENT".to_string()); // Using literal "\\\\n" to represent escaped backslash n
        let result_replace = serialize_repo(&config_replace).unwrap();
        let output_string_replace = result_replace.0;
        assert!(output_string_replace.contains("Path: test.txt\nContent: test content"));
        // Should replace "\\\\n" with newline
    }

    // Sort order tests
    #[test]
    fn test_serialize_repo_sort_order() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        // Create files with different priorities and names to check sort order
        std::fs::write(temp_dir.path().join("file_b.txt"), "content").unwrap(); // Default priority 0, index 1
        std::fs::write(temp_dir.path().join("file_a.txt"), "content").unwrap(); // Default priority 0, index 0
        std::fs::create_dir(temp_dir.path().join("src")).unwrap();
        std::fs::write(temp_dir.path().join("src/file_c.rs"), "content").unwrap(); // Priority 100, index 0

        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config).unwrap();

        // print results
        for file in result.1.iter() {
            println!("{}: {}", file.rel_path, file.priority);
        }
        let files = result.1;

        assert_eq!(files.len(), 3);
        assert_eq!(files[0].rel_path, "file_a.txt"); // Priority 0, index 0
        assert_eq!(files[1].rel_path, "file_b.txt"); // Priority 0, index 1
        assert_eq!(files[2].rel_path, "src/file_c.rs"); // Highest priority (100) comes last
    }

    // Error handling tests

    #[test]
    fn test_serialize_repo_file_read_error() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test content").unwrap();
        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);

        if cfg!(unix) {
            // Make the file unreadable (Unix only)
            make_unreadable(&file_path).unwrap();

            let result = serialize_repo(&config);
            // In case of read error, it should still return Ok but skip the file
            assert!(result.is_ok());
            let output = result.unwrap();
            assert_eq!(output.1.len(), 0); // No files processed due to read error

            // Restore permissions so temp dir can be deleted
            make_readable(&file_path).unwrap();
        } else {
            // On Windows, just test normal processing
            let result = serialize_repo(&config);
            assert!(result.is_ok());
            let output = result.unwrap();
            assert_eq!(output.1.len(), 1); // File should be processed normally
        }
    }

    #[test]
    fn test_serialize_repo_json_error() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;
        // Simulate a JSON serialization error by making files empty, which might cause issues if content is not handled properly
        let result = serialize_repo(&config);
        assert!(result.is_ok(), "serialize_repo should not error even if JSON serialization might have issues with empty content");
    }

    #[test]
    fn test_is_text_file_io_error() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("unreadable.txt");
        fs::write(&file_path, "test content").unwrap();

        if cfg!(unix) {
            // Make the file unreadable (Unix only)
            make_unreadable(&file_path).unwrap();

            let result = is_text_file(&file_path, &[]);
            assert!(
                result.is_err(),
                "is_text_file should return Err for unreadable file"
            );

            // Restore permissions so temp dir can be deleted
            make_readable(&file_path).unwrap();
        } else {
            // On Windows, just test that the function works normally
            let result = is_text_file(&file_path, &[]);
            assert!(result.is_ok(), "is_text_file should succeed on Windows");
        }
    }

    #[test]
    fn test_serialize_repo_with_priority_rules() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), "content").unwrap();
        std::fs::write(temp_dir.path().join("src_file.rs"), "content").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.priority_rules = vec![PriorityRule {
            pattern: "src_.*".to_string(),
            score: 500,
        }];
        let result = serialize_repo(&config).unwrap();
        let files = result.1;
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].rel_path, "file.txt");
        assert_eq!(files[0].priority, 0);
        assert_eq!(files[1].rel_path, "src_file.rs"); // Highest priority comes last
        assert_eq!(files[1].priority, 500);
    }

    #[test]
    fn test_serialize_repo_with_ignore_patterns_config() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), "content").unwrap();
        std::fs::write(temp_dir.path().join("log.log"), "log content").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.ignore_patterns = vec!["*.log".to_string()];
        let result = serialize_repo(&config).unwrap();
        let files = result.1;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, "file.txt"); // log.log should be ignored
    }

    #[test]
    fn test_serialize_repo_with_binary_extensions_config() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), "content").unwrap();
        std::fs::write(temp_dir.path().join("data.bin"), [0u8, 1u8, 2u8]).unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.binary_extensions = vec!["bin".to_string()];
        let result = serialize_repo(&config).unwrap();
        let files = result.1;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, "file.txt"); // data.bin should be ignored
    }

    #[test]
    fn test_concat_files_empty_files() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let files = vec![];
        let output = yek::concat_files(&files, &config).unwrap();
        assert_eq!(output, "");
    }

    #[test]
    fn test_concat_files_json_output_empty_files() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;
        let files = vec![];
        let output = yek::concat_files(&files, &config).unwrap();
        assert_eq!(output, "[]");
    }

    #[test]
    fn test_concat_files_various_inputs() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);

        let files = vec![
            ProcessedFile {
                priority: 100,
                file_index: 0,
                rel_path: "src/main.rs".to_string(),
                content: "fn main() {}".to_string(),
            },
            ProcessedFile {
                priority: 50,
                file_index: 1,
                rel_path: "README.md".to_string(),
                content: "# Yek".to_string(),
            },
        ];

        // Test default template
        let output_default = yek::concat_files(&files, &config).unwrap();
        assert!(output_default.contains(">>>> src/main.rs\nfn main() {}"));
        assert!(output_default.contains(">>>> README.md\n# Yek"));

        // Test JSON output
        config.json = true;
        let output_json = yek::concat_files(&files, &config).unwrap();
        assert!(output_json.contains(r#""filename": "src/main.rs""#));
        assert!(output_json.contains(r#""content": "fn main() {}""#));
        assert!(output_json.contains(r#""filename": "README.md""#));
        assert!(output_json.contains(r##""content": "# Yek"##));

        // Test custom template
        config.json = false;
        config.output_template = Some("==FILE_PATH==\n---\nFILE_CONTENT\n====".to_string());
        let output_custom = yek::concat_files(&files, &config).unwrap();
        assert!(output_custom.contains("==src/main.rs==\n---\nfn main() {}\n===="));
        assert!(output_custom.contains("==README.md==\n---\n# Yek\n===="));
    }

    #[test]
    fn test_concat_files_json_output_special_chars_in_filename() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;

        let files = vec![ProcessedFile {
            priority: 100,
            file_index: 0,
            rel_path: "file with ünicöde.txt".to_string(),
            content: "content".to_string(),
        }];
        let output_json = yek::concat_files(&files, &config).unwrap();
        assert!(output_json.contains(r#""filename": "file with ünicöde.txt""#));
    }

    #[test]
    fn test_concat_files_template_output_empty_content() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = false;

        let files = vec![ProcessedFile {
            priority: 100,
            file_index: 0,
            rel_path: "file.txt".to_string(),
            content: "".to_string(), // Empty content
        }];
        let output_template = yek::concat_files(&files, &config).unwrap();
        assert!(output_template.contains(">>>> file.txt\n")); // Should handle empty content
    }

    #[test]
    fn test_concat_files_json_output_empty_content() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;

        let files = vec![ProcessedFile {
            priority: 100,
            file_index: 0,
            rel_path: "file.txt".to_string(),
            content: "".to_string(), // Empty content
        }];
        let output_json = yek::concat_files(&files, &config).unwrap();
        assert!(output_json.contains(r#""content": """#)); // Should handle empty content in JSON
    }

    #[test]
    fn test_token_counting_basic() {
        let text = "Hello, world! This is a test.";
        let tokens = count_tokens(text);
        // GPT tokenizer has its own tokenization rules that may not match our assumptions
        assert_eq!(tokens, 9);
    }

    #[test]
    fn test_token_counting_with_template() {
        let config = YekConfig {
            output_template: Some("File: FILE_PATH\nContent:\nFILE_CONTENT".to_string()),
            ..Default::default()
        };
        let files = vec![ProcessedFile {
            rel_path: "test.txt".to_string(),
            content: "Hello world".to_string(),
            priority: 0,
            file_index: 0,
        }];
        let output = concat_files(&files, &config).unwrap();
        let tokens = count_tokens(&output);
        // Verify token count includes template overhead
        assert!(tokens > count_tokens("Hello world"));
    }

    #[test]
    fn test_token_counting_with_json() {
        let config = YekConfig {
            json: true,
            ..Default::default()
        };
        let files = vec![ProcessedFile {
            rel_path: "test.txt".to_string(),
            content: "Hello world".to_string(),
            priority: 0,
            file_index: 0,
        }];
        let output = concat_files(&files, &config).unwrap();
        let tokens = count_tokens(&output);
        // Verify token count includes JSON structure overhead
        assert!(tokens > count_tokens("Hello world"));
    }

    #[test]
    fn test_token_limit_enforcement() {
        let config = YekConfig {
            token_mode: true,
            tokens: "10".to_string(), // Set a very low token limit
            // Include filename in template so we can verify which files are included
            output_template: Some(">>>> FILE_PATH\nFILE_CONTENT".to_string()),
            ..Default::default()
        };
        let files = vec![
            ProcessedFile {
                rel_path: "test1.txt".to_string(),
                content: "This is a short test".to_string(),
                priority: 0,
                file_index: 0,
            },
            ProcessedFile {
                rel_path: "test2.txt".to_string(),
                content: "This is another test that should be excluded".to_string(),
                priority: 0,
                file_index: 1,
            },
        ];
        let output = concat_files(&files, &config).unwrap();
        // Check that only the first file is included in the output
        assert!(
            output.contains("test1.txt"),
            "Expected file test1.txt to be present"
        );
        assert!(
            !output.contains("test2.txt"),
            "Expected file test2.txt to be excluded"
        );
    }

    #[test]
    fn test_parse_token_limit() {
        assert_eq!(parse_token_limit("1000").unwrap(), 1000);
        assert_eq!(parse_token_limit("1k").unwrap(), 1000);
        assert_eq!(parse_token_limit("1K").unwrap(), 1000);
        assert!(parse_token_limit("-1").is_err());
        assert!(parse_token_limit("invalid").is_err());
    }

    // Bug validation tests
    #[test]
    fn test_bug_119_cannot_handle_emojis() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let file_name = "file_with_emoji_😀.txt";
        std::fs::write(temp_dir.path().join(file_name), "content with emoji 😀").unwrap();

        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config);
        // If bug is present, this might fail
        assert!(
            result.is_ok(),
            "serialize_repo should handle files with emojis in names"
        );
        let (output, files) = result.unwrap();
        assert_eq!(files.len(), 1);
        // The rel_path should be relative to the current directory
        assert_eq!(files[0].rel_path, file_name);
        assert!(output.contains(&format!(">>>> {}\ncontent with emoji 😀", file_name)));
    }

    #[test]
    fn test_bug_125_file_paths_relativity_unreliable_with_globs() {
        init_tracing();
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        std::fs::write(dir1.path().join("file.txt"), "content1").unwrap();
        std::fs::write(dir2.path().join("file.txt"), "content2").unwrap();

        // Use globs for multiple sources
        let config = create_test_config(vec![
            format!("{}/*.txt", dir1.path().to_string_lossy()),
            format!("{}/*.txt", dir2.path().to_string_lossy()),
        ]);

        let result = serialize_repo(&config);
        assert!(result.is_ok());
        let (output, files) = result.unwrap();
        // This test verifies that each file's rel_path is unique, which is the expected correct behavior.
        // If the bug is present (rel_path is unreliable), this assertion will fail.
        // The output should contain both file contents, and rel_paths should be unique.
        assert!(output.contains("content1"));
        assert!(output.contains("content2"));
        // Assert that rel_path is unique for each file.
        let rel_paths: std::collections::HashSet<_> = files.iter().map(|f| &f.rel_path).collect();
        assert_eq!(
            rel_paths.len(),
            files.len(),
            "rel_path should be unique for each file"
        );
    }

    #[test]
    fn test_bug_144_missing_file_paths_in_output() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

        let config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        let result = serialize_repo(&config).unwrap();
        let output = result.0;
        // Check that FILE_PATH is not empty
        assert!(
            output.contains(">>>> test.txt\ncontent"),
            "File path should not be missing in output"
        );
    }

    #[test]
    fn test_line_numbers_feature() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let content = "line 1\nline 2\nline 3";
        std::fs::write(temp_dir.path().join("test.txt"), content).unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.line_numbers = true;
        let result = serialize_repo(&config).unwrap();
        let output = result.0;

        // Check that line numbers are included
        assert!(output.contains("  1 | line 1"));
        assert!(output.contains("  2 | line 2"));
        assert!(output.contains("  3 | line 3"));
    }

    #[test]
    fn test_line_numbers_feature_json_output() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let content = "line 1\nline 2";
        std::fs::write(temp_dir.path().join("test.txt"), content).unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.line_numbers = true;
        config.json = true;
        let result = serialize_repo(&config).unwrap();
        let output = result.0;

        // Check that line numbers are included in JSON content
        assert!(output.contains(r#""content": "  1 | line 1\n  2 | line 2""#));
    }

    #[test]
    fn test_line_numbers_feature_empty_file() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("empty.txt"), "").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.line_numbers = true;
        let result = serialize_repo(&config).unwrap();
        let output = result.0;

        // Empty file should still have the file header
        assert!(output.contains(">>>> empty.txt\n"));
    }

    #[test]
    fn test_line_numbers_feature_single_line() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("single.txt"), "single line").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.line_numbers = true;
        let result = serialize_repo(&config).unwrap();
        let output = result.0;

        // Single line should have line number 1
        assert!(output.contains("  1 | single line"));
    }
    #[test]
    fn test_serialize_repo_with_nonexistent_paths() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let config = create_test_config(vec![
            temp_dir
                .path()
                .join("nonexistent1")
                .to_string_lossy()
                .to_string(),
            temp_dir
                .path()
                .join("nonexistent2")
                .to_string_lossy()
                .to_string(),
        ]);

        let result = serialize_repo(&config);
        // Should succeed but with warnings
        assert!(result.is_ok());
        let (output, files) = result.unwrap();
        assert!(files.is_empty()); // No files processed
        assert_eq!(output, ""); // Empty output
    }

    #[test]
    fn test_serialize_repo_with_mixed_existent_nonexistent() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("existent.txt"), "content").unwrap();

        let config = create_test_config(vec![
            temp_dir
                .path()
                .join("existent.txt")
                .to_string_lossy()
                .to_string(),
            temp_dir
                .path()
                .join("nonexistent.txt")
                .to_string_lossy()
                .to_string(),
        ]);

        let result = serialize_repo(&config);
        assert!(result.is_ok());
        let (output, files) = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(output.contains("content"));
    }

    #[test]
    fn test_parse_token_limit_edge_cases() {
        // Test with very large numbers
        assert_eq!(parse_token_limit("999999k").unwrap(), 999999000);

        // Test with zero (parse_token_limit allows 0, validation happens elsewhere)
        assert_eq!(parse_token_limit("0").unwrap(), 0);
        assert_eq!(parse_token_limit("0k").unwrap(), 0); // 0k = 0 * 1000 = 0

        // Test with invalid format
        assert!(parse_token_limit("k").is_err());
        assert!(parse_token_limit("123k456").is_err());
    }

    #[test]
    fn test_concat_files_with_token_limit_exceeded() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.token_mode = true;
        config.tokens = "5".to_string(); // Very low token limit

        let files = vec![
            ProcessedFile {
                priority: 100,
                file_index: 0,
                rel_path: "long.txt".to_string(),
                content: "This is a very long piece of content that should exceed the token limit."
                    .to_string(),
            },
            ProcessedFile {
                priority: 50,
                file_index: 1,
                rel_path: "short.txt".to_string(),
                content: "Short".to_string(),
            },
        ];

        let result = concat_files(&files, &config);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Should include only the short file or part of the long one
        assert!(output.contains("Short") || output.len() < 100);
    }

    #[test]
    fn test_serialize_repo_with_debug_logging() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.debug = true; // Enable debug logging

        let result = serialize_repo(&config);
        assert!(result.is_ok());
        // The function should work with debug enabled
    }

    #[test]
    fn test_concat_files_with_tree_header_and_token_mode() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.tree_header = true;
        config.token_mode = true;
        config.tokens = "1000".to_string();

        let files = vec![ProcessedFile {
            priority: 100,
            file_index: 0,
            rel_path: "test.txt".to_string(),
            content: "content".to_string(),
        }];

        let result = concat_files(&files, &config);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Should include tree header and content
        assert!(output.contains("test.txt"));
        assert!(output.contains("content"));
    }

    // Priority 3: Output generation tests
    #[test]
    fn test_template_processing_with_special_escaping() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        
        // Create file with content that needs escaping
        let content = "Line with \"quotes\" and \\backslash\\ and $special {chars}";
        fs::write(temp_dir.path().join("special.txt"), content).unwrap();
        
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.output_template = Some("=== FILE_PATH ===\n{{FILE_CONTENT}}".to_string());
        
        let result = serialize_repo(&config);
        assert!(result.is_ok());
        let (output, _) = result.unwrap();
        
        // Template should handle special characters
        assert!(output.contains("=== special.txt ==="));
        assert!(output.contains("{{Line with \"quotes\""));
    }

    #[test]
    fn test_json_output_with_special_characters() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        
        // Create file with JSON special characters
        let content = r#"{"key": "value with \"quotes\" and \n newline"}"#;
        fs::write(temp_dir.path().join("data.json"), content).unwrap();
        
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.json = true;
        
        let result = serialize_repo(&config);
        assert!(result.is_ok());
        let (output, _) = result.unwrap();
        
        // JSON output should properly escape nested JSON
        assert!(output.contains(r#""filename": "data.json""#));
        // Content should be properly escaped
        assert!(output.contains(r#"\"quotes\""#) || output.contains(r#"\\\"quotes\\\""#));
    }

    #[test]
    fn test_token_limit_overflow_handling() {
        init_tracing();
        let temp_dir = tempdir().unwrap();
        
        // Create multiple files that exceed token limit
        for i in 0..10 {
            let content = format!("This is file {} with some content that will contribute to token count", i);
            fs::write(temp_dir.path().join(format!("file{}.txt", i)), content).unwrap();
        }
        
        let mut config = create_test_config(vec![temp_dir.path().to_string_lossy().to_string()]);
        config.token_mode = true;
        config.tokens = "50".to_string(); // Very low limit
        
        let result = concat_files(
            &vec![
                ProcessedFile {
                    priority: 0,
                    file_index: 0,
                    rel_path: "file0.txt".to_string(),
                    content: "This is file 0 with some content that will contribute to token count".to_string(),
                },
                ProcessedFile {
                    priority: 0,
                    file_index: 1,
                    rel_path: "file1.txt".to_string(),
                    content: "This is file 1 with some content that will contribute to token count".to_string(),
                },
            ],
            &config,
        );
        
        assert!(result.is_ok());
        let output = result.unwrap();
        // Should include at least one file but not all due to token limit
        assert!(output.contains("file0.txt") || output.contains("file1.txt"));
        assert!(output.len() < 500); // Should be truncated
    }

    #[test]
    fn test_tree_rendering_with_unicode_paths() {
        use yek::tree::generate_tree;
        use std::path::PathBuf;
        
        let paths = vec![
            PathBuf::from("文档/说明.txt"),
            PathBuf::from("código/main.rs"),
            PathBuf::from("файлы/данные.json"),
        ];
        
        let result = generate_tree(&paths);
        
        // Should handle Unicode paths correctly
        assert!(result.contains("文档/"));
        assert!(result.contains("说明.txt"));
        assert!(result.contains("código/"));
        assert!(result.contains("main.rs"));
        assert!(result.contains("файлы/"));
        assert!(result.contains("данные.json"));
    }

    #[test]
    fn test_tree_rendering_empty_prefix() {
        use yek::tree::generate_tree;
        use std::path::PathBuf;
        
        // Test with single root file (empty prefix case)
        let paths = vec![PathBuf::from("single.txt")];
        let result = generate_tree(&paths);
        
        assert!(result.contains("Directory structure:"));
        assert!(result.contains("└── single.txt"));
        // Should not have any prefix before the root item
        let lines: Vec<&str> = result.lines().collect();
        for line in lines {
            if line.contains("single.txt") {
                assert!(line.starts_with("└──"));
                break;
            }
        }
    }

    #[test]
    fn test_tree_rendering_root_only_paths() {
        use yek::tree::generate_tree;
        use std::path::PathBuf;
        
        // Test with only root-level files (no nested directories)
        let paths = vec![
            PathBuf::from("a.txt"),
            PathBuf::from("b.txt"),
            PathBuf::from("c.txt"),
        ];
        
        let result = generate_tree(&paths);
        
        assert!(result.contains("├── a.txt"));
        assert!(result.contains("├── b.txt"));
        assert!(result.contains("└── c.txt")); // Last item uses └──
    }
}
