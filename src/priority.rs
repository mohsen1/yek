use git2::Repository;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityRule {
    pub pattern: String,
    pub score: i32,
}

/// Determine final priority of a file by scanning the priority list
/// in descending order of score.
pub fn get_file_priority(path: &str, rules: &[PriorityRule]) -> i32 {
    rules
        .iter()
        .filter_map(|rule| {
            if glob::Pattern::new(&rule.pattern).unwrap().matches(path) {
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

/// Get the commit time of the most recent change to each file using git2.
/// Returns a map from file path (relative to the repo root) → last commit Unix time.
/// If Git or .git folder is missing, returns None instead of erroring.
pub fn get_recent_commit_times_git2(repo_path: &Path) -> Option<HashMap<String, u64>> {
    // Walk up until you find a .git folder but not higher than the base of the given repo_path
    let mut current_path = repo_path.to_path_buf();
    while current_path.components().count() > 1 {
        if current_path.join(".git").exists() {
            break;
        }
        current_path = current_path.parent()?.to_path_buf();
    }

    let repo = match Repository::open(&current_path) {
        Ok(repo) => repo,
        Err(_) => {
            debug!("Not a Git repository or unable to open: {:?}", current_path);
            return None;
        }
    };

    let mut revwalk = match repo.revwalk() {
        Ok(revwalk) => revwalk,
        Err(_) => {
            debug!("Unable to get revwalk for: {:?}", current_path);
            return None;
        }
    };

    if let Err(e) = revwalk.push_head() {
        debug!(
            "Unable to push HEAD to revwalk: {:?} in {:?}",
            e, current_path
        );
        return None;
    }
    revwalk.set_sorting(git2::Sort::TIME).ok()?;

    let mut commit_times = HashMap::new();
    for oid in revwalk {
        let oid = match oid {
            Ok(oid) => oid,
            Err(e) => {
                debug!("Error during revwalk iteration: {:?}", e);
                continue;
            }
        };
        let commit = match repo.find_commit(oid) {
            Ok(commit) => commit,
            Err(e) => {
                debug!("Failed to find commit for OID {:?}: {:?}", oid, e);
                continue;
            }
        };
        let tree = match commit.tree() {
            Ok(tree) => tree,
            Err(e) => {
                debug!("Failed to get tree for commit {:?}: {:?}", oid, e);
                continue;
            }
        };
        let time = commit.time().seconds() as u64;

        tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            if let Some(name) = entry.name() {
                if entry.kind() == Some(git2::ObjectType::Blob) {
                    let full_path = format!("{}{}", root, name);
                    commit_times.entry(full_path).or_insert(time);
                }
            }
            git2::TreeWalkResult::Ok
        })
        .ok()?;
    }

    Some(commit_times)
}
