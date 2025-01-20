mod integration_common;

use integration_common::{create_file, setup_temp_repo};
use std::fs;
use tempfile::TempDir;
use walkdir::WalkDir;
use yek::{serialize_repo, PriorityRule, YekConfig};

#[test]
fn test_git_priority_basic() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    let repo_path = repo.path();
    let output_dir = repo_path.join("test_output");
    fs::create_dir_all(&output_dir)?;

    // Create test files and commit them
    create_file(repo_path, "src/main.rs", b"fn main() {}");
    create_file(repo_path, "docs/README.md", b"# Documentation");

    // Run serialization
    let mut config = YekConfig::default();
    config.output_dir = Some(output_dir.clone());
    serialize_repo(repo_path, Some(&config))?;

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
    let mut config = YekConfig::default();
    config.stream = true;
    serialize_repo(repo_path, Some(&config))?;

    Ok(())
}

#[test]
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
    let config = YekConfig {
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
        output_dir: Some(output_dir.clone()),
        ..Default::default()
    };
    serialize_repo(repo_path, Some(&config))?;

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

    let mut config = YekConfig::default();
    config.output_dir = Some(output_dir);
    serialize_repo(repo_path, Some(&config))?;
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

    let mut config = YekConfig::default();
    config.output_dir = Some(output_dir);
    serialize_repo(temp.path(), Some(&config))?;
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

    let mut config = YekConfig::default();
    config.output_dir = Some(output_dir);
    serialize_repo(repo_path, Some(&config))?;
    Ok(())
}
