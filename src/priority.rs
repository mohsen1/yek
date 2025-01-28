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

/// Rank-based approach to compute how "recent" each file is (0=oldest, 1=newest).
/// Then scale it to a user-defined or default max boost.
pub fn compute_recentness_boost(
    commit_times: &HashMap<String, u64>,
    max_boost: i32,
) -> HashMap<String, i32> {
    if commit_times.is_empty() {
        return HashMap::new();
    }

    // Sort by ascending commit time => first is oldest
    let mut sorted: Vec<(&String, &u64)> = commit_times.iter().collect();
    sorted.sort_by_key(|(_, t)| **t);

    // oldest file => rank=0, newest => rank=1
    let last_index = sorted.len().saturating_sub(1) as f64;
    if last_index < 1.0 {
        // If there's only one file, or zero, no boosts make sense
        let mut single = HashMap::new();
        for file in commit_times.keys() {
            single.insert(file.clone(), 0);
        }
        return single;
    }

    let mut result = HashMap::new();
    for (i, (path, _time)) in sorted.iter().enumerate() {
        let rank = i as f64 / last_index; // 0.0..1.0 (older files get lower rank)
        let boost = (rank * max_boost as f64).round() as i32; // Newer files get higher boost
        result.insert((*path).clone(), boost);
    }
    result
}

#[cfg(target_family = "windows")]
#[allow(dead_code)]
fn is_effectively_absolute(path: &std::path::Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    // Also treat a leading slash/backslash as absolute
    match path.to_str() {
        Some(s) => s.starts_with('/') || s.starts_with('\\'),
        None => false,
    }
}

#[cfg(not(target_family = "windows"))]
#[allow(dead_code)]
fn is_effectively_absolute(path: &std::path::Path) -> bool {
    path.is_absolute()
}

/// Get the commit time of the most recent change to each file.
/// Returns a map from file path (relative to the repo root) → last commit Unix time.
/// If Git or .git folder is missing, returns None instead of erroring.
#[allow(dead_code)]
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
             --pretty=format:%ct \
             -- . | tr -cd '[:print:]\n' | iconv -f utf-8",
        ])
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        debug!("Git command failed, skipping Git-based prioritization");
        return None;
    }

    // Parse the output into a map of file → timestamp
    let output = String::from_utf8_lossy(&output.stdout);
    let mut lines = output.lines();

    // Check if there are any commits
    if lines.clone().next().is_none() {
        debug!("No commits found, skipping Git-based prioritization");
        return None;
    }

    let mut result = HashMap::new();

    while let Some(timestamp_str) = lines.next() {
        // Skip empty lines
        if timestamp_str.is_empty() {
            continue;
        }

        // Parse the timestamp
        let timestamp = match timestamp_str.parse::<u64>() {
            Ok(ts) => ts,
            Err(_) => continue,
        };

        // Get all files until next timestamp
        while let Some(file) = lines.next() {
            if file.is_empty() {
                break;
            }
            // Only store if we can convert path to relative
            let path = Path::new(file);
            if !is_effectively_absolute(path) {
                result.insert(file.to_string(), timestamp);
            }
        }
    }

    Some(result)
}
