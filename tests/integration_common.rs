use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Creates a temporary directory and initializes a `.git` repo inside it.
/// Returns a `TempDir` whose path is a fresh Git repository directory.
#[allow(dead_code)]
pub fn setup_temp_repo() -> TempDir {
    let tempdir = TempDir::new().unwrap();
    let repo_path = tempdir.path();

    // Initialize a new git repository
    Command::new("git")
        .arg("init")
        .arg("--quiet")
        .current_dir(repo_path)
        .status()
        .unwrap();

    // Configure user name and email
    Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test User")
        .current_dir(repo_path)
        .status()
        .unwrap();

    Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(repo_path)
        .status()
        .unwrap();

    tempdir
}

/// Creates (or overwrites) a file at `[repo_dir]/[file_path]` with `content`.
/// If `repo_dir` contains `.git`, automatically `git add` and `git commit`.
/// This function handles large or binary data (including `\0`) without shell expansions.
#[allow(dead_code)]
pub fn create_file(repo_path: &Path, file_path: &str, content: &[u8]) {
    let full_path = repo_path.join(file_path);
    fs::create_dir_all(full_path.parent().unwrap()).unwrap();
    fs::write(full_path, content).unwrap();

    // Stage the new file
    Command::new("git")
        .arg("add")
        .arg(file_path)
        .current_dir(repo_path)
        .status()
        .unwrap();

    // Commit the file
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(format!("Add {}", file_path))
        .current_dir(repo_path)
        .status()
        .unwrap();
}

/// Ensures an output directory exists and is empty.
/// Creates it if it doesn't exist, cleans it if it does.
#[allow(dead_code)]
pub fn ensure_empty_output_dir(path: &Path) {
    if path.exists() {
        if path.is_dir() {
            fs::remove_dir_all(path).expect("Failed to clean output directory");
        } else {
            fs::remove_file(path).expect("Failed to remove file at output path");
        }
    }
    fs::create_dir_all(path).expect("Failed to create output directory");
}

#[allow(dead_code)]
pub fn assert_output_file_contains(dir: &Path, patterns: &[&str]) {
    let output_file_path = dir.join("output.txt");
    assert!(
        output_file_path.exists(),
        "Output file should exist: {}",
        output_file_path.display()
    );

    let content = fs::read_to_string(output_file_path).expect("Failed to read output file");
    for pattern in patterns {
        assert!(
            content.contains(pattern),
            "Output file should contain '{}'",
            pattern
        );
    }
}
