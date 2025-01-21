mod integration_common;

use integration_common::{create_file, setup_temp_repo};
use std::fs;
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;
use yek::{get_recent_commit_times, serialize_repo, PriorityRule, YekConfig};

#[test]
fn test_git_priority_basic() -> Result<(), Box<dyn std::error::Error>> {
    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().to_path_buf();
    let config = YekConfig {
        output_dir: Some(output_dir.clone()),
        ..Default::default()
    };
    let repo = setup_temp_repo();
    let repo_path = repo.path();
    fs::create_dir_all(&output_dir)?;

    // Create test files and commit them
    create_file(repo_path, "src/main.rs", b"fn main() {}");
    create_file(repo_path, "docs/README.md", b"# Documentation");

    // Verify Git commit times
    let git_times = get_recent_commit_times(repo_path).expect("Failed to get Git commit times");
    assert!(
        git_times.contains_key("src/main.rs"),
        "src/main.rs should have Git commit time"
    );
    assert!(
        git_times.contains_key("docs/README.md"),
        "docs/README.md should have Git commit time"
    );

    // Run serialization
    serialize_repo(repo_path, Some(&config))?;

    // Verify output
    assert!(output_dir.exists(), "Output directory should exist");
    let mut found_files = 0;
    let mut found_main = false;
    let mut found_readme = false;
    for entry in WalkDir::new(&output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            found_files += 1;
            let content = fs::read_to_string(entry.path())?;
            if content.contains("src/main.rs") {
                found_main = true;
            }
            if content.contains("docs/README.md") {
                found_readme = true;
            }
        }
    }
    assert!(
        found_files > 0,
        "Should have created at least one output file"
    );
    assert!(found_main, "Should have included src/main.rs");
    assert!(found_readme, "Should have included docs/README.md");

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

    // Verify Git commit times
    let git_times = get_recent_commit_times(repo_path).expect("Failed to get Git commit times");
    assert!(
        git_times.contains_key("src/main.rs"),
        "src/main.rs should have Git commit time"
    );
    assert!(
        git_times.contains_key("docs/README.md"),
        "docs/README.md should have Git commit time"
    );

    // Run serialization in stream mode
    let config = YekConfig {
        stream: true,
        ..Default::default()
    };
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

    // Verify Git commit times
    let git_times = get_recent_commit_times(repo_path).expect("Failed to get Git commit times");
    assert!(
        git_times.contains_key("src/main.rs"),
        "src/main.rs should have Git commit time"
    );
    assert!(
        git_times.contains_key("docs/README.md"),
        "docs/README.md should have Git commit time"
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
    let mut found_main = false;
    let mut found_readme = false;
    for entry in WalkDir::new(&output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            found_files += 1;
            let content = fs::read_to_string(entry.path())?;
            if content.contains("src/main.rs") {
                found_main = true;
            }
            if content.contains("docs/README.md") {
                found_readme = true;
            }
        }
    }
    assert!(
        found_files > 0,
        "Should have created at least one output file"
    );
    assert!(found_main, "Should have included src/main.rs");
    assert!(found_readme, "Should have included docs/README.md");

    Ok(())
}

#[test]
fn test_git_priority_empty_repo() -> Result<(), Box<dyn std::error::Error>> {
    let repo = TempDir::new()?;
    let repo_path = repo.path();
    let output_dir = repo_path.join("test_output");
    fs::create_dir_all(&output_dir)?;

    // Initialize empty git repo without any commits
    Command::new("git")
        .args(["init", "--quiet", repo_path.to_str().unwrap()])
        .status()
        .expect("Failed to run git init");

    // Configure git user info
    Command::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap(),
            "config",
            "user.name",
            "test-user",
        ])
        .status()
        .expect("Failed to set git user.name");

    Command::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap(),
            "config",
            "user.email",
            "test@example.com",
        ])
        .status()
        .expect("Failed to set git user.email");

    // Verify Git commit times
    let git_times = get_recent_commit_times(repo_path);
    assert!(
        git_times.is_none(),
        "Empty repo should return None for Git commit times"
    );

    let config = YekConfig {
        output_dir: Some(output_dir),
        ..Default::default()
    };
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

    // Verify Git commit times
    let git_times = get_recent_commit_times(temp.path());
    assert!(git_times.is_none(), "No Git repo should return None");

    let config = YekConfig {
        output_dir: Some(output_dir),
        ..Default::default()
    };
    serialize_repo(temp.path(), Some(&config))?;
    Ok(())
}

#[test]
fn test_git_priority_binary_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo_path = TempDir::new()?.into_path();
    let output_dir = TempDir::new()?.into_path();

    // Create test files
    create_file(
        &repo_path,
        "src/main.rs",
        "fn main() { println!(\"Hello\"); }".as_bytes(),
    );
    create_file(
        &repo_path,
        "README.md",
        "# Test Repository\n\nInitial commit.".as_bytes(),
    );
    create_file(
        &repo_path,
        "docs/README.md",
        "# Documentation\nThis is a test.".as_bytes(),
    );
    create_file(&repo_path, "binary.bin", b"fake binary\x00\x7f");

    let config = YekConfig {
        output_dir: Some(output_dir.clone()),
        binary_extensions: vec!["bin".to_string()],
        ..Default::default()
    };
    serialize_repo(&repo_path, Some(&config))?;

    // Check output
    let chunk_path = output_dir.join("chunk-0.txt");
    let content = fs::read_to_string(chunk_path)?;
    assert!(
        !content.contains("binary.bin"),
        "Should not have included binary.bin"
    );

    Ok(())
}
