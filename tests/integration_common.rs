use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Utility to create a temporary repo directory with a `.git` folder
/// plus an optional `.gitignore` and any additional files needed.
pub fn setup_temp_repo() -> TempDir {
    let repo_dir = TempDir::new().expect("Failed to create temp repo dir");
    let git_dir = repo_dir.path().join(".git");
    fs::create_dir(&git_dir).expect("Failed to create .git dir");

    repo_dir
}

/// Create a file within the repo with given relative path and content.
pub fn create_file(repo_dir: &Path, file_path: &str, content: &str) {
    let full_path = repo_dir.join(file_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create parent dirs");
    }
    let mut file = File::create(&full_path).expect("Failed to create file");
    file.write_all(content.as_bytes())
        .expect("Failed to write file content");
}
