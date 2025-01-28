mod integration_common;

use integration_common::{create_file, setup_temp_repo};
use std::fs;
use tempfile::TempDir;
use walkdir::WalkDir;
use yek::{config::FullYekConfig, priority::PriorityRule, serialize_repo};

#[test]
#[ignore]
fn test_git_priority_basic() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    let repo_path = repo.path();
    let output_dir = repo_path.join("test_output");
    fs::create_dir_all(&output_dir)?;

    // Create test files and commit them
    create_file(repo_path, "src/main.rs", b"fn main() {}");
    create_file(repo_path, "docs/README.md", b"# Documentation");
    // Run serialization
    let config = FullYekConfig {
        input_dirs: vec![repo_path.to_string_lossy().to_string()],
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
    serialize_repo(&config)?;

    // Verify output
    assert!(output_dir.exists(), "Output directory should exist");
    let mut found_files = 0;
    for entry in WalkDir::new(&output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            found_files += 1;
        }
    }
    assert!(
        found_files > 0,
        "Should have created at least one output file"
    );

    Ok(())
}

#[test]
fn test_git_priority_stream() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    let repo_path = repo.path();

    // Create test files and commit them
    create_file(
        repo_path,
        "src/main.rs",
        b"fn main() { println!(\"Hello\"); }",
    );
    create_file(
        repo_path,
        "docs/README.md",
        b"# Documentation\nThis is a test.",
    );

    // Run serialization in stream mode
    let config = FullYekConfig {
        input_dirs: vec![repo_path.to_string_lossy().to_string()],
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
    serialize_repo(&config)?;

    Ok(())
}

#[test]
#[ignore]
fn test_git_priority_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    let repo_path = repo.path();
    let output_dir = repo_path.join("test_output");
    fs::create_dir_all(&output_dir)?;

    // Create test files and commit them
    create_file(
        repo_path,
        "src/main.rs",
        b"fn main() { println!(\"Hello\"); }",
    );
    create_file(
        repo_path,
        "docs/README.md",
        b"# Documentation\nThis is a test.",
    );

    // Run serialization with custom config
    let config = FullYekConfig {
        input_dirs: vec![repo_path.to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: output_dir.to_string_lossy().to_string(),
        ignore_patterns: vec![],
        priority_rules: vec![
            PriorityRule {
                pattern: "^src/".to_string(),
                score: 100,
            },
            PriorityRule {
                pattern: "^docs/".to_string(),
                score: 50,
            },
        ],
        binary_extensions: vec![],
        stream: false,
        token_mode: false,
        output_file_full_path: output_dir.join("chunk-0.txt").to_string_lossy().to_string(),
    };
    serialize_repo(&config)?;

    // Verify output
    assert!(output_dir.exists(), "Output directory should exist");
    let mut found_files = 0;
    for entry in WalkDir::new(&output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            found_files += 1;
        }
    }
    assert!(
        found_files > 0,
        "Should have created at least one output file"
    );

    Ok(())
}

#[test]
fn test_git_priority_empty_repo() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    let repo_path = repo.path();
    let output_dir = repo_path.join("test_output");
    fs::create_dir_all(&output_dir)?;

    let config = FullYekConfig {
        input_dirs: vec![repo_path.to_string_lossy().to_string()],
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
    serialize_repo(&config)?;
    Ok(())
}

#[test]
fn test_git_priority_no_git() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let output_dir = temp.path().join("test_output");
    fs::create_dir_all(&output_dir)?;

    create_file(
        temp.path(),
        "file1.txt",
        b"This is a test file without git.",
    );

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
    serialize_repo(&config)?;
    Ok(())
}

#[test]
fn test_git_priority_binary_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    let repo_path = repo.path();
    let output_dir = repo_path.join("test_output");
    fs::create_dir_all(&output_dir)?;

    create_file(repo_path, "binary.bin", b"\x00\x01\x02\x03");
    create_file(repo_path, "text.txt", b"This is a text file.");

    let config = FullYekConfig {
        input_dirs: vec![repo_path.to_string_lossy().to_string()],
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
    serialize_repo(&config)?;
    Ok(())
}
