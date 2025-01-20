use std::fs;
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;
use yek::{serialize_repo, PriorityRule, YekConfig};

#[test]
fn test_git_priority_basic() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let output_dir = temp.path().join("test_output");
    fs::create_dir_all(&output_dir)?;

    // Setup git repo and show log using our shell script
    let output = Command::new("bash")
        .arg("tests/test_helpers.sh")
        .arg("setup_git_repo")
        .arg(temp.path().to_str().unwrap())
        .output()?;
    if !output.status.success() {
        eprintln!("Setup failed: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to setup git repo".into());
    }

    // Get git log for debugging
    let output = Command::new("bash")
        .arg("tests/test_helpers.sh")
        .arg("show_git_log")
        .arg(temp.path().to_str().unwrap())
        .output()?;
    if !output.status.success() {
        eprintln!(
            "Git log failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err("Failed to get git log".into());
    }

    // Clean any control characters from output except newlines
    let stdout_lossy = String::from_utf8_lossy(&output.stdout);
    let cleaned = stdout_lossy.replace(|c: char| c.is_control() && c != '\n', "");
    eprintln!("Git log output: {:?}", cleaned);

    // Run serialization
    let mut config = YekConfig::default();
    config.output_dir = Some(output_dir.clone());
    eprintln!("Output directory: {:?}", output_dir);
    eprintln!("Output directory exists: {}", output_dir.exists());
    serialize_repo(temp.path(), Some(&config))?;

    // Verify that the output directory exists and contains files
    assert!(output_dir.exists(), "Output directory should exist");
    let mut found_files = 0;
    for entry in WalkDir::new(&output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            found_files += 1;
            eprintln!("Found file: {:?}", entry.path());
        }
    }
    assert!(
        found_files > 0,
        "Should have created at least one output file"
    );

    // Clean up
    fs::remove_dir_all(&output_dir)?;
    Ok(())
}

#[test]
fn test_git_priority_stream() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    // Create a test repository
    fs::create_dir_all(temp.path().join("src"))?;
    fs::create_dir_all(temp.path().join("docs"))?;

    // Add some files
    fs::write(
        temp.path().join("src/main.rs"),
        "fn main() { println!(\"Hello\"); }",
    )?;
    fs::write(
        temp.path().join("docs/README.md"),
        "# Documentation\nThis is a test.",
    )?;

    // Run serialization in stream mode
    let mut config = YekConfig::default();
    config.stream = true;
    serialize_repo(temp.path(), Some(&config))?;

    Ok(())
}

#[test]
fn test_git_priority_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let output_dir = temp.path().join("test_output_config");
    fs::create_dir_all(&output_dir)?;

    // Setup git repo and show log using our shell script
    let output = Command::new("bash")
        .arg("tests/test_helpers.sh")
        .arg("setup_git_repo")
        .arg(temp.path().to_str().unwrap())
        .output()?;
    if !output.status.success() {
        eprintln!("Setup failed: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to setup git repo".into());
    }

    // Get git log for debugging
    let output = Command::new("bash")
        .arg("tests/test_helpers.sh")
        .arg("show_git_log")
        .arg(temp.path().to_str().unwrap())
        .output()?;
    if !output.status.success() {
        eprintln!(
            "Git log failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err("Failed to get git log".into());
    }

    // Clean any control characters from output except newlines
    let stdout_lossy = String::from_utf8_lossy(&output.stdout);
    let cleaned = stdout_lossy.replace(|c: char| c.is_control() && c != '\n', "");
    eprintln!("Git log output: {:?}", cleaned);

    // Run serialization with config
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
    serialize_repo(temp.path(), Some(&config))?;

    // Verify that the output directory exists and contains files
    assert!(output_dir.exists(), "Output directory should exist");
    let mut found_files = 0;
    for entry in WalkDir::new(&output_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            found_files += 1;
            eprintln!("Found file: {:?}", entry.path());
        }
    }
    assert!(
        found_files > 0,
        "Should have created at least one output file"
    );

    // Clean up
    fs::remove_dir_all(&output_dir)?;
    Ok(())
}

#[test]
fn test_git_priority_empty_repo() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    serialize_repo(temp.path(), None)?;
    Ok(())
}

#[test]
fn test_git_priority_no_git() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    fs::write(
        temp.path().join("file1.txt"),
        "This is a test file without git.",
    )?;
    serialize_repo(temp.path(), None)?;
    Ok(())
}

#[test]
fn test_git_priority_binary_files() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("binary.bin"), b"\x00\x01\x02\x03")?;
    fs::write(temp.path().join("text.txt"), "This is a text file.")?;
    serialize_repo(temp.path(), None)?;
    Ok(())
}
