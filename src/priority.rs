use std::{collections::HashMap, path::Path, process::Stdio};

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityRule {
    pub pattern: String,
    pub score: i32,
}

impl PriorityRule {
    #[allow(dead_code)]
    fn matches(&self, path: &str) -> bool {
        if let Ok(re) = Regex::new(&self.pattern) {
            re.is_match(path)
        } else {
            false
        }
    }
}

/// Determine final priority of a file by scanning the priority list
/// in descending order of score.
pub fn get_file_priority(path: &str, rules: &[PriorityRule]) -> i32 {
    rules
        .iter()
        .filter_map(|rule| {
            let re = match Regex::new(&rule.pattern) {
                Ok(re) => re,
                Err(_) => return None,
            };
            if re.is_match(path) {
                Some(rule.score)
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0)
}

/// Get the commit time of the most recent change to each file.
/// Returns a map from file path (relative to the repo root) → last commit Unix time.
/// If Git or .git folder is missing, returns None instead of erroring.
pub fn get_recent_commit_times(repo_path: &Path) -> Option<HashMap<String, u64>> {
    // Confirm there's a .git folder
    if !repo_path.join(".git").exists() {
        debug!("No .git directory found, skipping Git-based prioritization");
        return None;
    }
    // Get all files and their timestamps using bash with proper UTF-8 handling
    let output = std::process::Command::new("bash")
        .args([
            "-c",
            "export LC_ALL=en_US.UTF-8; export LANG=en_US.UTF-8; \
             git -c core.quotepath=false log \
             --format=%ct \
             --name-only \
             --no-merges \
             --no-renames \
             -- . | tr -cd '[:print:]\n' | iconv -f utf-8 -t utf-8 -c",
        ])
        .current_dir(repo_path)
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        debug!("Git log command failed, skipping Git-based prioritization");
        return None;
    }

    let mut git_times = HashMap::new();
    let mut current_timestamp = 0_u64;

    // Process output line by line with UTF-8 conversion
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        if let Ok(ts) = line.parse::<u64>() {
            current_timestamp = ts;
            debug!("Found timestamp: {}", ts);
        } else {
            debug!("Found file: {} with timestamp {}", line, current_timestamp);
            git_times.insert(line.to_string(), current_timestamp);
        }
    }

    if git_times.is_empty() {
        debug!("No valid timestamps found, skipping Git-based prioritization");
        None
    } else {
        Some(git_times)
    }
}
