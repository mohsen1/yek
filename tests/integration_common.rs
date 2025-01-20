use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Utility to create a temporary repo directory with a `.git` folder
/// plus an optional `.gitignore` and any additional files needed.
pub fn setup_temp_repo() -> TempDir {
    let repo_dir = TempDir::new().expect("Failed to create temp repo dir");

    // Initialize git repo using bash script
    Command::new("bash")
        .args([
            "tests/test_helpers.sh",
            "setup_temp_repo",
            repo_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to setup temp repo");

    repo_dir
}

/// Create a file within the repo with given relative path and content.
pub fn create_file(repo_dir: &Path, file_path: &str, content: &str) {
    Command::new("bash")
        .args([
            "tests/test_helpers.sh",
            "create_repo_file",
            repo_dir.to_str().unwrap(),
            file_path,
            content,
        ])
        .output()
        .expect("Failed to create file");
}
