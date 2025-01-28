mod integration_common;
use std::fs;
use tempfile::TempDir;
use yek::{config::FullYekConfig, serialize_repo};

#[test]
fn basic_file_output_test() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    // Create a test file
    let test_file = temp.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    let config = FullYekConfig {
        input_dirs: vec![temp.path().to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: output_dir.to_string_lossy().to_string(),
        ignore_patterns: vec![],
        priority_rules: vec![],
        binary_extensions: vec![],
        stream: false,
        token_mode: false,
        // This is the single file name from src/lib.rs -> write_output
        output_file_full_path: output_dir
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string(),
    };

    serialize_repo(&config).unwrap();

    // Verify single output file
    let out_file = output_dir.with_extension("txt");
    assert!(out_file.exists(), "Expected single output file");
    let content = fs::read_to_string(&out_file).unwrap();
    assert!(
        content.contains("test content"),
        "Output file should contain the test file content"
    );
}

#[test]
fn basic_pipe_test() {
    let temp = TempDir::new().unwrap();

    // Create a test file
    let test_file = temp.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Run in stream mode
    let config = FullYekConfig {
        input_dirs: vec![temp.path().to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: String::new(),
        ignore_patterns: vec![],
        priority_rules: vec![],
        binary_extensions: vec![],
        stream: true,
        token_mode: false,
        output_file_full_path: String::new(),
    };
    // We can't directly capture the stream here in a simple test,
    // so just ensure it doesn't error.
    serialize_repo(&config).unwrap();
}
