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

    // Run serialization
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
        output_file_full_path: output_dir.join("chunk-0.txt").to_string_lossy().to_string(),
    };
    serialize_repo(&config).unwrap();

    // Verify output
    println!("Output directory exists: {}", output_dir.exists());
    println!("Output directory contents:");
    for entry in fs::read_dir(&output_dir).unwrap() {
        let entry = entry.unwrap();
        println!("  {}", entry.path().display());
        let content = fs::read_to_string(entry.path()).unwrap();
        println!("File contents:\n{}", content);
    }

    // Check that the first chunk exists and contains our test file
    let chunk_0 = output_dir.join("chunk-0.txt");
    assert!(chunk_0.exists(), "Should write first chunk");
    let content = fs::read_to_string(chunk_0).unwrap();
    assert!(
        content.contains("test content"),
        "Should contain file content"
    );
}

#[test]
fn basic_pipe_test() {
    let temp = TempDir::new().unwrap();

    // Create a test file
    let test_file = temp.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Run serialization in stream mode
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
    serialize_repo(&config).unwrap();

    // The output should be written to stdout, which we can't easily capture in a test
    // So we just verify that the function runs without error
}
