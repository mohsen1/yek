use regex::Regex;
#[allow(dead_code)]
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Creates a temporary directory and initializes a `.git` repo inside it.
/// Returns a `TempDir` whose path is a fresh Git repository directory.
#[allow(dead_code)]
pub fn setup_temp_repo() -> TempDir {
    let repo_dir = TempDir::new().expect("Failed to create temp dir for repo");
    init_git_repo(repo_dir.path());
    repo_dir
}

/// Initializes a new Git repository in the given directory.
/// Configures user.name and user.email so commits will succeed without prompts.
#[allow(dead_code)]
fn init_git_repo(path: &Path) {
    let repo_str = path.to_str().expect("Non-UTF8 path to temp dir?");
    // 1. git init
    let status_init = Command::new("git")
        .args(["init", "--quiet", repo_str])
        .status()
        .expect("Failed to run git init");
    assert!(status_init.success(), "git init failed");

    // 2. Set a dummy user name and email so commits work
    let status_config_name = Command::new("git")
        .args(["-C", repo_str, "config", "user.name", "test-user"])
        .status()
        .expect("Failed to set git user.name");
    assert!(status_config_name.success(), "git config user.name failed");

    let status_config_email = Command::new("git")
        .args(["-C", repo_str, "config", "user.email", "test@example.com"])
        .status()
        .expect("Failed to set git user.email");
    assert!(
        status_config_email.success(),
        "git config user.email failed"
    );
}

/// Creates (or overwrites) a file at `[repo_dir]/[file_path]` with `content`.
/// If `repo_dir` contains `.git`, automatically `git add` and `git commit`.
/// This function handles large or binary data (including `\0`) without shell expansions.
#[allow(dead_code)]
pub fn create_file(repo_dir: &Path, file_path: &str, content: &[u8]) {
    // Ensure parent directories exist
    let full_path = repo_dir.join(file_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|_| panic!("Failed to create parent directory for {}", file_path));
    }

    // Write file content in Rust, no shell expansion
    fs::write(&full_path, content)
        .unwrap_or_else(|_| panic!("Failed to write file content for {}", file_path));

    // If there's a .git folder, stage & commit the file
    if repo_dir.join(".git").exists() {
        let repo_str = repo_dir.to_str().unwrap();

        // First check if .gitignore exists and if this file should be ignored
        let gitignore_path = repo_dir.join(".gitignore");
        if gitignore_path.exists() {
            let gitignore_content = fs::read_to_string(&gitignore_path).unwrap();
            let should_ignore = gitignore_content.lines().any(|pattern| {
                let pattern = pattern.trim();
                if pattern.is_empty() || pattern.starts_with('#') {
                    return false;
                }
                // Very basic glob matching - just checks if pattern is a prefix or suffix
                if pattern.ends_with('/') {
                    file_path.starts_with(&pattern[..pattern.len() - 1])
                } else if pattern.starts_with('*') {
                    file_path.ends_with(&pattern[1..])
                } else if pattern.ends_with('*') {
                    file_path.starts_with(&pattern[..pattern.len() - 1])
                } else {
                    file_path == pattern || file_path.starts_with(pattern)
                }
            });
            if should_ignore {
                return; // Don't commit ignored files
            }
        }

        // Also check if yek.toml exists and if this file should be ignored
        let yek_toml_path = repo_dir.join("yek.toml");
        if yek_toml_path.exists() {
            let yek_toml_content = fs::read_to_string(&yek_toml_path).unwrap();
            let should_ignore = yek_toml_content
                .lines()
                .filter(|line| line.contains("^")) // Only look at lines with regex patterns
                .map(|line| {
                    line.trim()
                        .trim_matches(|c| c == '"' || c == '[' || c == ']')
                })
                .filter(|line| !line.is_empty())
                .any(|pattern| {
                    if let Ok(re) = Regex::new(pattern) {
                        re.is_match(file_path)
                    } else {
                        false
                    }
                });
            if should_ignore {
                return; // Don't commit ignored files
            }
        }

        // Stage the file
        let status_add = Command::new("git")
            .args(["add", "-f", file_path])
            .current_dir(repo_dir)
            .status()
            .expect("git add failed");
        assert!(status_add.success(), "git add failed for {}", file_path);

        // Commit with a descriptive message
        let status_commit = Command::new("git")
            .args([
                "-C",
                repo_str,
                "commit",
                "--quiet",
                "--allow-empty", // allow empty trees
                "-m",
                &format!("Add {}", file_path),
            ])
            .status()
            .expect("Failed to git commit file");
        assert!(status_commit.success(), "git commit failed for {file_path}");
    }
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
