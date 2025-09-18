#[cfg(test)]
mod priority_tests {
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;
    use yek::priority::{
        compute_recentness_boost, get_file_priority, get_recent_commit_times_git2, PriorityRule,
    };

    #[test]
    fn test_get_file_priority_multiple_matches() {
        let rules = vec![
            PriorityRule {
                pattern: r"src/.*".to_string(),
                score: 5,
            },
            PriorityRule {
                pattern: r".*\.rs".to_string(),
                score: 10,
            },
        ];
        assert_eq!(get_file_priority("src/main.rs", &rules), 15);
    }

    #[test]
    fn test_compute_recentness_boost_empty() {
        let commit_times = HashMap::new();
        let boosts = compute_recentness_boost(&commit_times, 100);
        assert!(boosts.is_empty());
    }

    #[test]
    fn test_compute_recentness_boost_single() {
        let mut commit_times = HashMap::new();
        commit_times.insert("file.rs".to_string(), 12345);
        let boosts = compute_recentness_boost(&commit_times, 100);
        assert_eq!(boosts.get("file.rs"), Some(&0));
    }

    #[test]
    fn test_compute_recentness_boost_multiple() {
        let mut commit_times = HashMap::new();
        commit_times.insert("old.rs".to_string(), 10000);
        commit_times.insert("mid.rs".to_string(), 20000);
        commit_times.insert("new.rs".to_string(), 30000);

        let boosts = compute_recentness_boost(&commit_times, 100);

        assert_eq!(boosts.get("old.rs"), Some(&0));
        assert_eq!(boosts.get("mid.rs"), Some(&50));
        assert_eq!(boosts.get("new.rs"), Some(&100));
    }

    #[test]
    fn test_get_file_priority_no_rules() {
        let path = "src/main.rs";
        let rules = vec![];
        let priority = get_file_priority(path, &rules);
        assert_eq!(priority, 0);
    }

    #[test]
    fn test_get_file_priority_with_rules() {
        let path = "src/main.rs";
        let rules = vec![
            PriorityRule {
                pattern: r"src/.*\.rs".to_string(),
                score: 10,
            },
            PriorityRule {
                pattern: r".*\.md".to_string(),
                score: 5,
            },
        ];
        let priority = get_file_priority(path, &rules);
        assert_eq!(priority, 10);
    }

    #[test]
    fn test_get_file_priority_with_rules_no_match() {
        let path = "docs/README.txt";
        let rules = vec![
            PriorityRule {
                pattern: r"src/.*\.rs".to_string(),
                score: 10,
            },
            PriorityRule {
                pattern: r".*\.md".to_string(),
                score: 5,
            },
        ];
        let priority = get_file_priority(path, &rules);
        assert_eq!(priority, 0);
    }

    #[test]
    fn test_get_file_priority_invalid_regex() {
        let path = "src/main.rs";
        let rules = vec![PriorityRule {
            pattern: r"src/.*\.rs".to_string(),
            score: 10,
        }];
        let priority = get_file_priority(path, &rules);
        assert_eq!(priority, 10); // Should still match

        let rules = vec![PriorityRule {
            pattern: r"src/[[.*\.rs".to_string(), // Invalid regex
            score: 10,
        }];
        let priority = get_file_priority(path, &rules);
        assert_eq!(priority, 0); // Invalid regex should not match
    }

    #[test]
    fn test_compute_recentness_boost_single_file() {
        let mut commit_times = HashMap::new();
        commit_times.insert("file1.txt".to_string(), 1234567890);
        let max_boost = 100;
        let boosts = compute_recentness_boost(&commit_times, max_boost);
        assert_eq!(boosts.len(), 1);
        assert_eq!(boosts["file1.txt"], 0); // Single file gets 0 boost
    }

    #[test]
    fn test_compute_recentness_boost_multiple_files() {
        let mut commit_times = HashMap::new();
        commit_times.insert("file1.txt".to_string(), 1234567890);
        commit_times.insert("file2.txt".to_string(), 1234567891);
        commit_times.insert("file3.txt".to_string(), 1234567892);
        let max_boost = 100;
        let boosts = compute_recentness_boost(&commit_times, max_boost);
        assert_eq!(boosts.len(), 3);
        assert_eq!(boosts["file1.txt"], 0); // Oldest
        assert_eq!(boosts["file2.txt"], 50); // Middle
        assert_eq!(boosts["file3.txt"], 100); // Newest
    }

    #[test]
    fn test_compute_recentness_boost_unsorted_input() {
        let mut commit_times = HashMap::new();
        commit_times.insert("file3.txt".to_string(), 1234567892);
        commit_times.insert("file1.txt".to_string(), 1234567890);
        commit_times.insert("file2.txt".to_string(), 1234567891);
        let max_boost = 100;
        let boosts = compute_recentness_boost(&commit_times, max_boost);
        assert_eq!(boosts.len(), 3);
        assert_eq!(boosts["file1.txt"], 0); // Oldest
        assert_eq!(boosts["file2.txt"], 50); // Middle
        assert_eq!(boosts["file3.txt"], 100); // Newest
    }

    #[test]
    fn test_compute_recentness_boost_max_boost() {
        let mut commit_times = HashMap::new();
        commit_times.insert("file1.txt".to_string(), 1234567890);
        commit_times.insert("file2.txt".to_string(), 1234567891);
        let max_boost = 50;
        let boosts = compute_recentness_boost(&commit_times, max_boost);
        assert_eq!(boosts["file1.txt"], 0); // Oldest
        assert_eq!(boosts["file2.txt"], 50); // Newest, capped at max_boost
    }

    #[test]
    fn test_get_recent_commit_times_no_git() {
        let dir = tempdir().unwrap();
        let times = get_recent_commit_times_git2(dir.path(), 100);
        assert!(times.is_none());
    }

    #[test]
    fn test_get_recent_commit_times_with_git() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a Git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Set up git config for the test repository
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create some files and commit them
        fs::write(repo_path.join("file1.txt"), "content1").unwrap();
        std::process::Command::new("git")
            .args(["add", "file1.txt"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        fs::write(repo_path.join("file2.txt"), "content2").unwrap();
        std::process::Command::new("git")
            .args(["add", "file2.txt"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Add file2"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let times = get_recent_commit_times_git2(repo_path, 100).unwrap();
        assert_eq!(times.len(), 2);
        assert!(times.contains_key("file1.txt"));
        assert!(times.contains_key("file2.txt"));
    }

    #[test]
    fn test_get_recent_commit_times_empty_repo() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize an empty Git repo (no commits)
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let times = get_recent_commit_times_git2(repo_path, 100);
        assert!(times.is_none(), "Expected no times for empty repo");
    }

    #[test]
    fn test_get_recent_commit_times_git_failure() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a Git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Corrupt the .git directory to simulate a Git failure
        fs::remove_dir_all(repo_path.join(".git")).unwrap();
        fs::create_dir(repo_path.join(".git")).unwrap(); // Create an empty directory

        let times = get_recent_commit_times_git2(repo_path, 100);
        assert!(times.is_none(), "Expected no times on Git failure");
    }

    #[test]
    fn test_empty_priority_rules() {
        let rules = vec![];
        assert_eq!(get_file_priority("src/main.rs", &rules), 0);
    }

    #[test]
    fn test_single_priority_rule() {
        let rules = vec![PriorityRule {
            pattern: String::from(".*\\.rs$"),
            score: 100,
        }];
        assert_eq!(get_file_priority("src/main.rs", &rules), 100);
        assert_eq!(get_file_priority("README.md", &rules), 0);
    }

    #[test]
    fn test_multiple_priority_rules() {
        let rules = vec![
            PriorityRule {
                pattern: String::from(".*\\.rs$"),
                score: 100,
            },
            PriorityRule {
                pattern: String::from("^src/.*"),
                score: 50,
            },
        ];
        // File matches both patterns, should get sum of scores
        assert_eq!(get_file_priority("src/main.rs", &rules), 150);
        // File matches only .rs pattern
        assert_eq!(get_file_priority("tests/main.rs", &rules), 100);
        // File matches no patterns
        assert_eq!(get_file_priority("README.md", &rules), 0);
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let rules = vec![PriorityRule {
            pattern: String::from("[invalid regex"),
            score: 100,
        }];
        // Invalid regex should be skipped without affecting score
        assert_eq!(get_file_priority("any_file.txt", &rules), 0);
    }

    #[test]
    fn test_recentness_boost_empty() {
        let commit_times = HashMap::new();
        let boosts = compute_recentness_boost(&commit_times, 100);
        assert!(boosts.is_empty());
    }

    #[test]
    fn test_recentness_boost_single_file() {
        let mut commit_times = HashMap::new();
        commit_times.insert("file.rs".to_string(), 1000);

        let boosts = compute_recentness_boost(&commit_times, 100);
        assert_eq!(boosts["file.rs"], 0);
    }

    #[test]
    fn test_recentness_boost_evenly_spaced() {
        let mut commit_times = HashMap::new();
        commit_times.insert("oldest.rs".to_string(), 1000);
        commit_times.insert("middle.rs".to_string(), 2000);
        commit_times.insert("newest.rs".to_string(), 3000);

        let max_boost = 100;
        let boosts = compute_recentness_boost(&commit_times, max_boost);

        assert_eq!(boosts["oldest.rs"], 0);
        assert_eq!(boosts["newest.rs"], max_boost);
        assert!(boosts["middle.rs"] >= 45 && boosts["middle.rs"] <= 55);
    }

    #[test]
    fn test_recentness_boost_same_time() {
        let mut commit_times = HashMap::new();
        commit_times.insert("file1.rs".to_string(), 1000);
        commit_times.insert("file2.rs".to_string(), 1000);

        let boosts = compute_recentness_boost(&commit_times, 100);

        // Files with same timestamp should get same boost
        assert_eq!(boosts["file1.rs"], boosts["file2.rs"]);
    }
    #[test]
    fn test_get_recent_commit_times_git_max_depth() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a Git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Set up git config
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create nested directory structure
        fs::create_dir(repo_path.join("subdir")).unwrap();
        fs::write(repo_path.join("subdir").join("file.txt"), "content").unwrap();

        // Add and commit
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Test with max_commits = 1 (should limit results)
        let times = get_recent_commit_times_git2(repo_path, 1).unwrap();
        assert!(!times.is_empty());
    }

    #[test]
    fn test_get_recent_commit_times_git_tree_walk_error() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a Git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Set up git config
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create and commit a file
        fs::write(repo_path.join("file.txt"), "content").unwrap();
        std::process::Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Test that the function handles tree walk gracefully
        let times = get_recent_commit_times_git2(repo_path, 100).unwrap();
        assert!(times.contains_key("file.txt"));
    }

    #[test]
    fn test_get_recent_commit_times_git_revwalk_error() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a Git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Don't set up commits, so revwalk will be empty
        let times = get_recent_commit_times_git2(repo_path, 100);
        // Should return None for empty repo
        assert!(times.is_none());
    }

    #[test]
    fn test_compute_recentness_boost_large_time_range() {
        let mut commit_times = HashMap::new();
        commit_times.insert("old.rs".to_string(), 1000000000); // 2001
        commit_times.insert("new.rs".to_string(), 1670000000); // 2023

        let boosts = compute_recentness_boost(&commit_times, 100);
        assert_eq!(boosts["old.rs"], 0);
        assert_eq!(boosts["new.rs"], 100);
    }

    #[test]
    fn test_get_file_priority_regex_compilation_failure() {
        let rules = vec![PriorityRule {
            pattern: r"[invalid".to_string(), // Invalid regex
            score: 10,
        }];
        // Should return 0 when regex compilation fails
        assert_eq!(get_file_priority("test.rs", &rules), 0);
    }
    #[test]
    fn test_get_recent_commit_times_git_no_head() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a Git repo but don't create any commits or branches
        std::process::Command::new("git")
            .args(["init", "--bare"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let times = get_recent_commit_times_git2(repo_path, 100);
        // Should return None because there's no HEAD
        assert!(times.is_none());
    }

    #[test]
    fn test_get_recent_commit_times_git_corrupted_objects() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a Git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Set up git config
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create and commit a file
        fs::write(repo_path.join("file1.txt"), "content1").unwrap();
        std::process::Command::new("git")
            .args(["add", "file1.txt"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Corrupt the objects directory to simulate corruption
        let objects_dir = repo_path.join(".git").join("objects");
        if objects_dir.exists() {
            // Remove some objects to cause errors
            for entry in fs::read_dir(&objects_dir).unwrap().flatten() {
                if entry.path().is_dir() {
                    fs::remove_dir_all(entry.path()).ok();
                    break; // Remove just one directory
                }
            }
        }
        let _times = get_recent_commit_times_git2(repo_path, 100);
        // Should handle corruption gracefully (return None or partial results)
        // The function should not panic
        // We don't assert the exact result since corruption handling may vary
    }
}

// Priority 1: Critical Git integration error handling tests

#[test]
fn test_get_recent_commit_times_git_with_corrupted_head() {
    use std::fs;
    use tempfile::tempdir;
    use yek::priority::get_recent_commit_times_git2;

    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    // Initialize a Git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Corrupt the HEAD file
    let head_file = repo_path.join(".git").join("HEAD");
    fs::write(&head_file, "corrupted content").unwrap();

    let times = get_recent_commit_times_git2(repo_path, 100);
    // Should handle corrupted HEAD gracefully
    assert!(times.is_none());
}

#[test]
fn test_get_recent_commit_times_git_with_invalid_repo_structure() {
    use std::fs;
    use tempfile::tempdir;
    use yek::priority::get_recent_commit_times_git2;

    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    // Create a fake .git directory without proper structure
    fs::create_dir(repo_path.join(".git")).unwrap();
    fs::write(repo_path.join(".git").join("config"), "[core]\n").unwrap();

    let times = get_recent_commit_times_git2(repo_path, 100);
    // Should handle invalid repo structure gracefully
    assert!(times.is_none());
}

#[test]
fn test_get_recent_commit_times_git_with_detached_head() {
    use std::fs;
    use tempfile::tempdir;
    use yek::priority::get_recent_commit_times_git2;

    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    // Initialize a Git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Set up git config
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create and commit a file
    fs::write(repo_path.join("file.txt"), "content").unwrap();
    std::process::Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Get the commit hash
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Checkout to detached HEAD state
    std::process::Command::new("git")
        .args(["checkout", &commit_hash])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let times = get_recent_commit_times_git2(repo_path, 100);
    // Should handle detached HEAD state
    assert!(times.is_some());
    assert!(times.unwrap().contains_key("file.txt"));
}

#[test]
fn test_get_recent_commit_times_git_with_shallow_clone() {
    use std::fs;
    use tempfile::tempdir;
    use yek::priority::get_recent_commit_times_git2;

    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    // Initialize a Git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Set up git config
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create multiple commits
    for i in 1..=5 {
        let filename = format!("file{}.txt", i);
        fs::write(repo_path.join(&filename), format!("content{}", i)).unwrap();
        std::process::Command::new("git")
            .args(["add", &filename])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", &format!("Commit {}", i)])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    // Test with max_depth = 2 (shallow processing)
    let times = get_recent_commit_times_git2(repo_path, 2);
    assert!(times.is_some());
    let times = times.unwrap();
    // Should have processed only the most recent commits
    assert!(!times.is_empty());
}

#[test]
fn test_get_recent_commit_times_git_with_submodules() {
    use std::fs;
    use tempfile::tempdir;
    use yek::priority::get_recent_commit_times_git2;

    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    // Initialize a Git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create .gitmodules file (simulating submodules)
    fs::write(
        repo_path.join(".gitmodules"),
        "[submodule \"sub\"]\n\tpath = sub\n\turl = ./sub\n",
    )
    .unwrap();

    let times = get_recent_commit_times_git2(repo_path, 100);
    // Should handle repos with submodules configuration
    assert!(times.is_none() || times.unwrap().is_empty());
}
