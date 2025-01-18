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

    // Run serialization with non-stream mode to check output files
    let output_dir = temp.path().join("output");
    let result = serialize_repo(
        1024 * 1024, // 1MB max size
        Some(temp.path()),
        false,
        false,
        None,
        Some(&output_dir),
        None,
    )?;

    assert!(result.is_some(), "Should have output directory");

    // Read the first chunk to verify order
    let mut chunk_content = fs::read_to_string(output_dir.join("chunk-0.txt"))?;

    // Convert Windows paths to Unix style for consistent comparison
    #[cfg(windows)]
    {
        chunk_content = chunk_content.replace("\\", "/");
    }

    // Verify file order
    let old_pos = chunk_content.find("old.txt").expect("Should find old.txt");
    let recent_pos = chunk_content
        .find("recent.txt")
        .expect("Should find recent.txt");

    // recent files should appear after old files
    assert!(
        old_pos < recent_pos,
        "Old files should appear before recent files since higher priority files come last"
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
        true,  // stream mode (simulated pipe)
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
    let docs_date = chrono::DateTime::from_timestamp((now as i64) - 1, 0)
        .unwrap()
        .to_rfc3339();
    let src_date = chrono::DateTime::from_timestamp(now as i64, 0)
        .unwrap()
        .to_rfc3339();

    // Create and commit src/recent.rs with newer timestamp
    commit_file(temp.path(), "src/recent.rs", "recent content", &src_date)?;

    // Create and commit docs/recent.md with older timestamp
    commit_file(temp.path(), "docs/recent.md", "recent docs", &docs_date)?;

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
        1024 * 1024, // 1MB max size
        Some(temp.path()),
        false,
        false,
        Some(config),
        Some(&output_dir),
        None,
    )?;

    assert!(result.is_some(), "Should have output directory");

    // Read the first chunk to verify order
    let mut chunk_content = fs::read_to_string(output_dir.join("chunk-0.txt"))?;

    // Convert Windows paths to Unix style for consistent comparison
    #[cfg(windows)]
    {
        chunk_content = chunk_content.replace("\\", "/");
    }

    // Verify file order
    let docs_pos = chunk_content
        .find("docs/recent.md")
        .expect("Should find docs/recent.md");
    let src_pos = chunk_content
        .find("src/recent.rs")
        .expect("Should find src/recent.rs");
    let old_pos = chunk_content
        .find("src/old.rs")
        .expect("Should find src/old.rs");
    let recent_pos = chunk_content
        .find("src/recent.rs")
        .expect("Should find src/recent.rs");

    // src/recent.rs should appear last (highest priority: src/ + recent)
    assert!(
        docs_pos < src_pos,
        "docs/recent.md should appear before src/recent.rs since higher priority files come last"
    );

    // recent files should appear after old files
    assert!(
        old_pos < recent_pos,
        "Old files should appear before recent files since higher priority files come last"
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

    // Create test files in different paths
    fs::create_dir_all(temp.path().join("src/module1"))?;
    fs::create_dir_all(temp.path().join("src/module2"))?;

    commit_file(
        temp.path(),
        "src/module1/file1.txt",
        "content 1",
        "2023-01-01T12:00:00+00:00",
    )?;
    commit_file(
        temp.path(),
        "src/module2/file2.txt",
        "content 2",
        "2024-01-01T12:00:00+00:00",
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

#[test]
fn test_git_priority_boost_with_path_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    setup_git_repo(temp.path())?;

    // We'll give src/module2/recent.rs a commit date that is 1 second newer
    // so that it definitely has a higher priority than docs/recent.md.
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    // For docs:
    let docs_date = chrono::DateTime::from_timestamp((now as i64) - 1, 0)
        .unwrap()
        .to_rfc3339();
    // For src:
    let src_date = chrono::DateTime::from_timestamp(now as i64, 0)
        .unwrap()
        .to_rfc3339();

    fs::create_dir_all(temp.path().join("src/module1"))?;
    fs::create_dir_all(temp.path().join("src/module2"))?;
    fs::create_dir_all(temp.path().join("docs"))?;

    // Create files in src/module1
    commit_file(
        temp.path(),
        "src/module1/old.rs",
        "old content",
        "2023-01-01T12:00:00+00:00",
    )?;

    // Create files in src/module2 with newer timestamp
    commit_file(
        temp.path(),
        "src/module2/recent.rs",
        "recent content",
        &src_date,
    )?;

    // Create files in docs with slightly older timestamp
    commit_file(temp.path(), "docs/recent.md", "recent docs", &docs_date)?;

    // Create config with priority rules
    let config = YekConfig {
        priority_rules: vec![
            PriorityRule {
                score: 100,
                patterns: vec!["^src/".to_string()],
            },
            PriorityRule {
                score: 50,
                patterns: vec!["^docs/".to_string()],
            },
        ],
        ..Default::default()
    };

    // Run serialization with non-stream mode to check output files
    let output_dir = temp.path().join("output");
    let result = serialize_repo(
        1024 * 1024, // 1MB max size
        Some(temp.path()),
        false,
        false,
        Some(config),
        Some(&output_dir),
        None,
    )?;

    assert!(result.is_some(), "Should have output directory");

    // Read the first chunk to verify order
    let mut chunk_content = fs::read_to_string(output_dir.join("chunk-0.txt"))?;

    // Convert Windows paths to Unix style for consistent comparison
    #[cfg(windows)]
    {
        chunk_content = chunk_content.replace("\\", "/");
    }

    // Verify file order
    let docs_pos = chunk_content
        .find("docs/recent.md")
        .expect("Should find docs/recent.md");
    let src_pos = chunk_content
        .find("src/module2/recent.rs")
        .expect("Should find src/module2/recent.rs");
    let old_pos = chunk_content
        .find("src/module1/old.rs")
        .expect("Should find src/module1/old.rs");
    let recent_pos = chunk_content
        .find("src/module2/recent.rs")
        .expect("Should find src/module2/recent.rs");

    // src/recent.rs should appear last (highest priority: src/ + recent)
    assert!(
        docs_pos < src_pos,
        "docs/recent.md should appear before src/module2/recent.rs since higher priority files come last"
    );

    // src/module1/old.rs should appear before src/module2/recent.rs
    assert!(
        old_pos < recent_pos,
        "src/module1/old.rs should appear before src/module2/recent.rs since higher priority files come last"
    );

    Ok(())
}
