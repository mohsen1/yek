mod integration_common;
use std::fs;
use tempfile::TempDir;
use yek::{config::FullYekConfig, serialize_repo};

#[test]
fn skips_known_binary_files() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    // Create a binary file
    fs::write(temp.path().join("test.jpg"), b"binary content").unwrap();
    // Create a text file
    fs::write(temp.path().join("test.txt"), "text content").unwrap();

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
        output_file_full_path: output_dir
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string(),
    };
    serialize_repo(&config).unwrap();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(out_file).unwrap();
    assert!(content.contains("text content"));
    assert!(!content.contains("binary content"));
}

#[test]
fn respects_custom_binary_extensions() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    // Create a file with custom binary extension
    fs::write(temp.path().join("test.dat"), b"custom binary content").unwrap();
    // Create a text file
    fs::write(temp.path().join("test.txt"), "text content").unwrap();

    let config = FullYekConfig {
        input_dirs: vec![temp.path().to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: output_dir.to_string_lossy().to_string(),
        ignore_patterns: vec![],
        priority_rules: vec![],
        binary_extensions: vec!["dat".to_string()],
        stream: false,
        token_mode: false,
        output_file_full_path: output_dir
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string(),
    };
    serialize_repo(&config).unwrap();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(out_file).unwrap();
    assert!(content.contains("text content"));
    assert!(!content.contains("custom binary content"));
}
