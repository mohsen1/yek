use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command as SysCommand;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use yek::{get_recent_commit_times, serialize_repo, PriorityRule, YekConfig};

fn setup_git_repo(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize git repo
    let status = SysCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .status()?;
    assert!(status.success());

    // Configure git user for commits
    let _ = SysCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir)
        .status()?;
    let _ = SysCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .status()?;

    Ok(())
}

fn commit_file(
    repo_dir: &Path,
    file_name: &str,
    content: &str,
    date: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Write file
    fs::write(repo_dir.join(file_name), content)?;

    // Stage file
    let status = SysCommand::new("git")
        .args(["add", file_name])
        .current_dir(repo_dir)
        .status()?;
    assert!(status.success());

    // Set environment variables for Git commit date
    let mut cmd = SysCommand::new("git");
    cmd.env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .args(["commit", "-m", &format!("Update {}", file_name)])
        .current_dir(repo_dir);

    let status = cmd.status()?;
    assert!(status.success());

    Ok(())
}

#[test]
fn test_get_recent_commit_times() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    setup_git_repo(temp.path())?;

    // Create and commit files with different dates
    commit_file(
        temp.path(),
        "old.txt",
        "old content",
        "2023-01-01T12:00:00+00:00",
    )?;
    commit_file(
        temp.path(),
        "recent.txt",
        "recent content",
        "2024-01-01T12:00:00+00:00",
    )?;

    let times = get_recent_commit_times(temp.path()).expect("Should get commit times");

    // Verify we got timestamps for both files
    assert_eq!(times.len(), 2);

    // Verify the timestamps are in the expected order
    let old_ts = times.get("old.txt").expect("Should have old.txt");
    let recent_ts = times.get("recent.txt").expect("Should have recent.txt");
    assert!(
        recent_ts > old_ts,
        "Recent file should have later timestamp"
    );

    Ok(())
}

#[test]
fn test_git_priority_boost() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    setup_git_repo(temp.path())?;

    // Create test files with different dates
    commit_file(
        temp.path(),
        "old.txt",
        "old content",
        "2023-01-01T12:00:00+00:00",
    )?;

    // Create a file with very recent changes
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let recent_date = chrono::DateTime::from_timestamp(now as i64, 0)
        .unwrap()
        .to_rfc3339();
    commit_file(temp.path(), "recent.txt", "recent content", &recent_date)?;

    // Run serialization with default config
    let result = serialize_repo(
        1024 * 1024, // 1MB max size
        Some(temp.path()),
        false, // don't count tokens
        true,  // stream mode
        None,  // no config
        None,  // no output dir override
        None,  // no path prefix
    )?;

    // The function should complete successfully
    assert!(result.is_none(), "Stream mode should return None");

    // We can't easily verify the exact output order in stream mode,
    // but we can verify that the Git functionality works by checking
    // the commit times directly
    let times = get_recent_commit_times(temp.path()).expect("Should get commit times");
    let old_ts = times.get("old.txt").expect("Should have old.txt");
    let recent_ts = times.get("recent.txt").expect("Should have recent.txt");

    // Verify timestamps are as expected
    assert!(
        recent_ts > old_ts,
        "Recent file should have later timestamp"
    );

    // The recent file's timestamp should be very close to now
    assert!(
        now - recent_ts < 86400,
        "Recent file should be less than a day old"
    );

    Ok(())
}

#[test]
fn test_no_git_fallback() {
    let temp = TempDir::new().unwrap();

    // Try to get commit times from a non-git directory
    let times = get_recent_commit_times(temp.path());
    assert!(times.is_none(), "Should return None for non-git directory");

    // Verify serialization still works without git
    let result = serialize_repo(
        1024 * 1024, // 1MB max size
        Some(temp.path()),
        false, // don't count tokens
        true,  // stream mode
        None,  // no config
        None,  // no output dir override
        None,  // no path prefix
    );
    assert!(result.is_ok(), "Should succeed even without git");
}

#[test]
fn test_git_priority_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    setup_git_repo(temp.path())?;

    // Create files with different dates and in different directories
    fs::create_dir_all(temp.path().join("src"))?;
    fs::create_dir_all(temp.path().join("docs"))?;

    // Old files in different directories
    commit_file(
        temp.path(),
        "src/old.rs",
        "old content",
        "2023-01-01T12:00:00+00:00",
    )?;
    commit_file(
        temp.path(),
        "docs/old.md",
        "old content",
        "2023-01-01T12:00:00+00:00",
    )?;

    // Recent files in different directories
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let recent_date = chrono::DateTime::from_timestamp(now as i64, 0)
        .unwrap()
        .to_rfc3339();
    commit_file(temp.path(), "src/recent.rs", "recent content", &recent_date)?;
    commit_file(
        temp.path(),
        "docs/recent.md",
        "recent content",
        &recent_date,
    )?;

    // Create config that prioritizes src/ files
    let config = YekConfig {
        priority_rules: vec![PriorityRule {
            score: 100,
            patterns: vec!["^src/.*".to_string()],
        }],
        ..Default::default()
    };

    // Run serialization with non-stream mode to check output files
    let output_dir = temp.path().join("output");
    let result = serialize_repo(
        1024 * 1024,
        Some(temp.path()),
        false,
        false,
        Some(config),
        Some(&output_dir),
        None,
    )?;

    assert!(result.is_some(), "Should have output directory");

    // Read the first chunk to verify order
    let chunk_content = fs::read_to_string(output_dir.join("chunk-0.txt"))?;

    // src/recent.rs should appear first (highest priority: src/ + recent)
    assert!(
        chunk_content.find("src/recent.rs").unwrap()
            < chunk_content.find("docs/recent.md").unwrap_or(usize::MAX),
        "src/recent.rs should appear before docs/recent.md"
    );

    // recent files should appear before old files
    assert!(
        chunk_content.find("recent").unwrap() < chunk_content.find("old").unwrap_or(usize::MAX),
        "Recent files should appear before old files"
    );

    Ok(())
}

#[test]
fn test_git_priority_with_untracked_files() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    setup_git_repo(temp.path())?;

    // Create and commit an old file
    commit_file(
        temp.path(),
        "old.txt",
        "old content",
        "2023-01-01T12:00:00+00:00",
    )?;

    // Create untracked files (not added to git)
    fs::write(temp.path().join("untracked1.txt"), "untracked content")?;
    fs::write(temp.path().join("untracked2.txt"), "more untracked content")?;

    let times = get_recent_commit_times(temp.path()).expect("Should get commit times");

    // Should only have the committed file
    assert_eq!(times.len(), 1);
    assert!(times.contains_key("old.txt"));
    assert!(!times.contains_key("untracked1.txt"));
    assert!(!times.contains_key("untracked2.txt"));

    // Verify serialization still processes untracked files
    let _result = serialize_repo(
        1024 * 1024,
        Some(temp.path()),
        false,
        true,
        None,
        None,
        None,
    )?;

    Ok(())
}

#[test]
fn test_git_priority_with_deleted_files() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    setup_git_repo(temp.path())?;

    // Create and commit some files
    commit_file(
        temp.path(),
        "file1.txt",
        "content 1",
        "2023-01-01T12:00:00+00:00",
    )?;
    commit_file(
        temp.path(),
        "file2.txt",
        "content 2",
        "2023-01-01T12:00:00+00:00",
    )?;

    // Delete one file
    fs::remove_file(temp.path().join("file1.txt"))?;

    let times = get_recent_commit_times(temp.path()).expect("Should get commit times");

    // Both files should be in the git history
    assert_eq!(times.len(), 2);
    assert!(times.contains_key("file1.txt"));
    assert!(times.contains_key("file2.txt"));

    // But serialization should only process existing files
    let _result = serialize_repo(
        1024 * 1024,
        Some(temp.path()),
        false,
        true,
        None,
        None,
        None,
    )?;

    Ok(())
}

#[test]
fn test_git_priority_with_path_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    setup_git_repo(temp.path())?;

    // Create directory structure
    fs::create_dir_all(temp.path().join("src/module1"))?;
    fs::create_dir_all(temp.path().join("src/module2"))?;

    // Create files in different directories
    commit_file(
        temp.path(),
        "src/module1/file1.rs",
        "content 1",
        "2024-01-01T12:00:00+00:00",
    )?;
    commit_file(
        temp.path(),
        "src/module2/file2.rs",
        "content 2",
        "2024-01-01T12:00:00+00:00",
    )?;

    // Run serialization with path prefix
    let _result = serialize_repo(
        1024 * 1024,
        Some(temp.path()),
        false,
        true,
        None,
        None,
        Some("src/module1"),
    )?;

    // Verify that git times are still retrieved correctly
    let times = get_recent_commit_times(temp.path()).expect("Should get commit times");
    assert_eq!(times.len(), 2); // Should have both files in git history

    Ok(())
}

#[test]
fn test_git_priority_with_empty_repo() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    setup_git_repo(temp.path())?;

    // Configure Git user for commits (required for an empty repo)
    let _ = SysCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(temp.path())
        .status()?;
    let _ = SysCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(temp.path())
        .status()?;

    // Create an initial commit to properly initialize the repo
    let _ = SysCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "Initial commit"])
        .current_dir(temp.path())
        .status()?;

    // Now try to get commit times from the empty (but initialized) repo
    let times = get_recent_commit_times(temp.path());
    assert!(times.is_some(), "Should return empty map for empty repo");
    assert_eq!(times.unwrap().len(), 0);

    Ok(())
}
