use git2;
use regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityRule {
    pub pattern: String,
    pub score: i32,
}

/// Determine final priority of a file by scanning the priority list
/// in descending order of score.
pub fn get_file_priority(path: &str, rules: &[PriorityRule]) -> i32 {
    let mut priority = 0;
    for rule in rules {
        if let Ok(re) = regex::Regex::new(&rule.pattern) {
            if re.is_match(path) {
                priority += rule.score;
            }
        }
    }
    priority
}

/// Calculate file priority including category-based offset
pub fn get_file_priority_with_category(
    path: &str,
    rules: &[PriorityRule],
    category_weights: &crate::category::CategoryWeights,
) -> (i32, crate::category::FileCategory) {
    let category = crate::category::categorize_file(path);
    let rule_priority = get_file_priority(path, rules);
    let category_offset = category_weights.get_offset(category);
    let total_priority = rule_priority + category_offset;

    debug!(
        "File: {} | Category: {} | Rule priority: {} | Category offset: {} | Total: {}",
        path,
        category.name(),
        rule_priority,
        category_offset,
        total_priority
    );

    (total_priority, category)
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

    // If there's only one file, or zero, no boosts make sense
    if sorted.len() <= 1 {
        let mut single = HashMap::new();
        for file in commit_times.keys() {
            single.insert(file.clone(), 0);
        }
        return single;
    }

    let mut result = HashMap::new();
    let oldest_time = *sorted.first().unwrap().1;
    let newest_time = *sorted.last().unwrap().1;
    let time_range = newest_time.saturating_sub(oldest_time) as f64;

    // If all files have the same timestamp, they should all get the same boost
    if time_range == 0.0 {
        for (path, _) in sorted {
            result.insert(path.clone(), 0);
        }
        return result;
    }

    // Calculate boost based on time difference from oldest file
    for (path, time) in sorted {
        let time_diff = (*time - oldest_time) as f64;
        let rank = time_diff / time_range; // 0.0..1.0 (older files get lower rank)
        let boost = (rank * max_boost as f64).round() as i32; // Newer files get higher boost
        result.insert(path.clone(), boost);
    }
    result
}

/// Get the commit time of the most recent change to each file using git2.
/// Returns a map from file path (relative to the repo root) → last commit Unix time.
/// If Git or .git folder is missing, returns None instead of erroring.
/// Only considers up to `max_commits` most recent commits.
pub fn get_recent_commit_times_git2(
    repo_path: &Path,
    max_commits: usize,
) -> Option<HashMap<String, u64>> {
    // Walk up until you find a .git folder but not higher than the base of the given repo_path
    let mut current_path = repo_path.to_path_buf();
    while current_path.components().count() > 1 {
        if current_path.join(".git").exists() {
            break;
        }
        current_path = current_path.parent()?.to_path_buf();
    }

    let repo = match git2::Repository::open(&current_path) {
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
    for oid_result in revwalk.take(max_commits) {
        let oid = match oid_result {
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

// ---------------------------------------------------------------------------
// Dependency Centrality Analysis & Ranking (Issue #255)
// ---------------------------------------------------------------------------

/// Normalizes path string by replacing backslashes with slashes and trimming leading `./`.
pub fn normalize_priority_path(p: &str) -> String {
    p.replace('\\', "/").trim_start_matches("./").to_string()
}

/// Extracts dependency / import / require / include specifiers from a file's content.
///
/// Supports:
/// - JavaScript / TypeScript: `import ... from '...'`, `require('...')`, `export ... from '...'`
/// - Rust: `mod foo;`, `use foo::...;`, `use crate::foo::...;`, `use super::foo;`
/// - Python: `from foo import bar`, `from .foo import bar`, `import foo`
/// - C / C++: `#include "foo.h"`
/// - Go: `import "pkg/foo"`, `import "./foo"`
pub fn extract_import_specifiers(content: &str) -> Vec<String> {
    use std::sync::OnceLock;
    static QUOTED_IMPORT_RE: OnceLock<regex::Regex> = OnceLock::new();
    static RUST_MOD_RE: OnceLock<regex::Regex> = OnceLock::new();
    static RUST_USE_RE: OnceLock<regex::Regex> = OnceLock::new();
    static PYTHON_IMPORT_RE: OnceLock<regex::Regex> = OnceLock::new();

    let quoted_re = QUOTED_IMPORT_RE.get_or_init(|| {
        regex::Regex::new(
            r#"(?m)(?:import\s+(?:\{[^}]*\}|\*|\w+)?\s*(?:from\s+)?|require\s*\(\s*|export\s+(?:\{[^}]*\}|\*)\s+from\s+|#include\s*)["'<]([^"'>]+)[">']"#
        ).expect("valid quoted import regex")
    });

    let rust_mod_re = RUST_MOD_RE.get_or_init(|| {
        regex::Regex::new(r#"(?m)^\s*(?:pub(?:\([^)]+\))?\s+)?mod\s+([a-zA-Z0-9_]+)\s*;"#)
            .expect("valid rust mod regex")
    });

    let rust_use_re = RUST_USE_RE.get_or_init(|| {
        regex::Regex::new(r#"(?m)^\s*(?:pub(?:\([^)]+\))?\s+)?use\s+(?:crate::|super::)?([a-zA-Z0-9_]+)"#)
            .expect("valid rust use regex")
    });

    let python_import_re = PYTHON_IMPORT_RE.get_or_init(|| {
        regex::Regex::new(r#"(?m)^\s*(?:from\s+([a-zA-Z0-9_.]+)\s+import|import\s+([a-zA-Z0-9_.]+))"#)
            .expect("valid python import regex")
    });

    let mut specifiers = Vec::new();

    // 1. Quoted imports (JS, TS, C, C++, Go, CSS, etc.)
    for cap in quoted_re.captures_iter(content) {
        if let Some(spec) = cap.get(1) {
            specifiers.push(spec.as_str().to_string());
        }
    }

    // 2. Rust mod statements
    for cap in rust_mod_re.captures_iter(content) {
        if let Some(spec) = cap.get(1) {
            specifiers.push(spec.as_str().to_string());
        }
    }

    // 3. Rust use statements
    for cap in rust_use_re.captures_iter(content) {
        if let Some(spec) = cap.get(1) {
            let spec_str = spec.as_str();
            if spec_str != "std" && spec_str != "core" && spec_str != "alloc" {
                specifiers.push(spec_str.to_string());
            }
        }
    }

    // 4. Python imports
    for cap in python_import_re.captures_iter(content) {
        if let Some(spec) = cap.get(1).or_else(|| cap.get(2)) {
            specifiers.push(spec.as_str().to_string());
        }
    }

    specifiers
}

/// Catalog of project files used to resolve import specifiers to project files.
pub struct ProjectFileCatalog {
    exact_paths: HashMap<String, usize>,
    no_ext_paths: HashMap<String, Vec<usize>>,
    stems: HashMap<String, Vec<usize>>,
    filenames: HashMap<String, Vec<usize>>,
}

impl ProjectFileCatalog {
    pub fn new(files: &[String]) -> Self {
        let mut exact_paths = HashMap::new();
        let mut no_ext_paths: HashMap<String, Vec<usize>> = HashMap::new();
        let mut stems: HashMap<String, Vec<usize>> = HashMap::new();
        let mut filenames: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, file_path) in files.iter().enumerate() {
            let normalized = normalize_priority_path(file_path);
            exact_paths.insert(normalized.clone(), idx);

            let path = Path::new(&normalized);

            if let Some(parent) = path.parent() {
                let parent_str = parent.to_string_lossy().replace('\\', "/");
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let no_ext = if parent_str.is_empty() || parent_str == "." {
                        stem.to_string()
                    } else {
                        format!("{}/{}", parent_str, stem)
                    };
                    no_ext_paths.entry(no_ext).or_default().push(idx);
                }
            }

            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                stems.entry(stem.to_string()).or_default().push(idx);
            }

            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                filenames.entry(name.to_string()).or_default().push(idx);
            }
        }

        Self {
            exact_paths,
            no_ext_paths,
            stems,
            filenames,
        }
    }

    pub fn resolve(&self, from_file: &str, specifier: &str) -> Option<usize> {
        let norm_from = normalize_priority_path(from_file);
        let from_path = Path::new(&norm_from);
        let parent_dir = from_path
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        let clean_spec = specifier.trim().replace('\\', "/");
        if clean_spec.is_empty() {
            return None;
        }

        // 1. If relative path (e.g. "./models", "../models")
        if clean_spec.starts_with("./") || clean_spec.starts_with("../") {
            let resolved_relative = resolve_relative_path(&parent_dir, &clean_spec);
            if let Some(target) = self.match_path_or_mod(&resolved_relative) {
                return Some(target);
            }
        }

        // 2. Direct exact or clean path
        let stripped = clean_spec
            .trim_start_matches("@/")
            .trim_start_matches("~/")
            .trim_start_matches("./");

        if let Some(target) = self.match_path_or_mod(stripped) {
            return Some(target);
        }

        // 3. Same directory resolution
        if !parent_dir.is_empty() && parent_dir != "." {
            let in_same_dir = format!("{}/{}", parent_dir, stripped);
            if let Some(target) = self.match_path_or_mod(&in_same_dir) {
                return Some(target);
            }
        }

        // 4. Dot-separated or double-colon module path
        let slash_converted = stripped.replace("::", "/").replace('.', "/");
        if slash_converted != stripped {
            if let Some(target) = self.match_path_or_mod(&slash_converted) {
                return Some(target);
            }
            if !parent_dir.is_empty() && parent_dir != "." {
                let in_same_dir = format!("{}/{}", parent_dir, slash_converted);
                if let Some(target) = self.match_path_or_mod(&in_same_dir) {
                    return Some(target);
                }
            }
            if let Some(first_seg) = slash_converted.split('/').next() {
                if !first_seg.is_empty()
                    && first_seg != "crate"
                    && first_seg != "super"
                    && first_seg != "self"
                {
                    if let Some(target) = self.match_path_or_mod(first_seg) {
                        return Some(target);
                    }
                    if !parent_dir.is_empty() && parent_dir != "." {
                        let in_same_dir = format!("{}/{}", parent_dir, first_seg);
                        if let Some(target) = self.match_path_or_mod(&in_same_dir) {
                            return Some(target);
                        }
                    }
                }
            }
        }

        // 5. Direct filename match
        if let Some(targets) = self.filenames.get(stripped) {
            if targets.len() == 1 {
                return Some(targets[0]);
            }
            for &idx in targets {
                if self.is_in_dir(idx, &parent_dir) {
                    return Some(idx);
                }
            }
            return targets.first().copied();
        }

        // 6. Direct unique stem match
        let stem = Path::new(stripped)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(stripped);
        if let Some(targets) = self.stems.get(stem) {
            if targets.len() == 1 {
                return Some(targets[0]);
            }
            for &idx in targets {
                if self.is_in_dir(idx, &parent_dir) {
                    return Some(idx);
                }
            }
        }

        None
    }

    fn match_path_or_mod(&self, path_str: &str) -> Option<usize> {
        let p = path_str.trim_end_matches('/');
        if let Some(&idx) = self.exact_paths.get(p) {
            return Some(idx);
        }
        if let Some(targets) = self.no_ext_paths.get(p) {
            if let Some(&first) = targets.first() {
                return Some(first);
            }
        }
        for entry in &[
            "mod.rs",
            "index.ts",
            "index.js",
            "index.tsx",
            "index.jsx",
            "__init__.py",
        ] {
            let entry_path = format!("{}/{}", p, entry);
            if let Some(&idx) = self.exact_paths.get(&entry_path) {
                return Some(idx);
            }
        }
        None
    }

    fn is_in_dir(&self, idx: usize, parent_dir: &str) -> bool {
        if parent_dir.is_empty() || parent_dir == "." {
            return true;
        }
        for (path, &i) in &self.exact_paths {
            if i == idx && path.starts_with(parent_dir) {
                return true;
            }
        }
        false
    }
}

fn resolve_relative_path(base_dir: &str, relative: &str) -> String {
    let mut parts = Vec::new();
    if !base_dir.is_empty() && base_dir != "." {
        for seg in base_dir.split('/') {
            if !seg.is_empty() && seg != "." {
                parts.push(seg);
            }
        }
    }
    for seg in relative.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        } else if seg == ".." {
            parts.pop();
        } else {
            parts.push(seg);
        }
    }
    parts.join("/")
}

/// Compute in-degree dependency centrality for each file in the project.
///
/// Returns a map of relative file path -> in-degree count (how many other project files reference it).
pub fn compute_dependency_centrality(
    files: &[(String, String)],
) -> HashMap<String, usize> {
    if files.is_empty() {
        return HashMap::new();
    }

    let file_paths: Vec<String> = files
        .iter()
        .map(|(p, _)| normalize_priority_path(p))
        .collect();
    let catalog = ProjectFileCatalog::new(&file_paths);

    let mut in_degrees: Vec<std::collections::HashSet<usize>> =
        vec![std::collections::HashSet::new(); files.len()];

    for (from_idx, (from_path, content)) in files.iter().enumerate() {
        let specifiers = extract_import_specifiers(content);
        for spec in specifiers {
            if let Some(target_idx) = catalog.resolve(from_path, &spec) {
                // Do not count self-references
                if target_idx != from_idx {
                    in_degrees[target_idx].insert(from_idx);
                }
            }
        }
    }

    let mut result = HashMap::new();
    for (idx, (orig_path, _)) in files.iter().enumerate() {
        result.insert(orig_path.clone(), in_degrees[idx].len());
    }

    result
}

/// Compute centrality boost scaled to max_boost (0..=max_boost).
/// Higher in-degree = higher boost.
pub fn compute_centrality_boost(
    centrality_scores: &HashMap<String, usize>,
    max_boost: i32,
) -> HashMap<String, i32> {
    if centrality_scores.is_empty() {
        return HashMap::new();
    }

    let max_degree = centrality_scores.values().copied().max().unwrap_or(0);
    let mut result = HashMap::new();

    if max_degree == 0 {
        for path in centrality_scores.keys() {
            result.insert(path.clone(), 0);
        }
        return result;
    }

    for (path, &deg) in centrality_scores {
        let rank = deg as f64 / max_degree as f64;
        let boost = (rank * max_boost as f64).round() as i32;
        result.insert(path.clone(), boost);
    }

    result
}

/// Combine recency score and dependency centrality score using a normalized hybrid formula:
/// `score = (recency_weight * recency_score) + (centrality_weight * centrality_score)`.
///
/// If `commit_times` is empty/None, centrality takes 100% of the boost.
/// If `centrality_scores` is empty/None, recency takes 100% of the boost.
/// If both are available, weights are normalized:
/// `combined_rank = (recency_weight * r_rank + centrality_weight * c_rank) / (recency_weight + centrality_weight)`.
/// The combined rank (0.0..=1.0) is scaled by `max_boost`.
pub fn compute_hybrid_priority_boost(
    commit_times: Option<&HashMap<String, u64>>,
    centrality_scores: Option<&HashMap<String, usize>>,
    max_boost: i32,
    recency_weight: f64,
    centrality_weight: f64,
) -> HashMap<String, i32> {
    let has_recency = commit_times.map(|m| !m.is_empty()).unwrap_or(false);
    let has_centrality = centrality_scores.map(|m| !m.is_empty()).unwrap_or(false);

    if !has_recency && !has_centrality {
        return HashMap::new();
    }

    // 1. Calculate normalized recency ranks (0.0..=1.0)
    let recency_ranks: HashMap<String, f64> = if let Some(times) = commit_times {
        if times.len() <= 1 {
            times.keys().map(|k| (k.clone(), 0.0)).collect()
        } else {
            let min_time = *times.values().min().unwrap();
            let max_time = *times.values().max().unwrap();
            let range = (max_time.saturating_sub(min_time)) as f64;
            if range == 0.0 {
                times.keys().map(|k| (k.clone(), 0.0)).collect()
            } else {
                times
                    .iter()
                    .map(|(k, &t)| {
                        let rank = (t.saturating_sub(min_time)) as f64 / range;
                        (k.clone(), rank)
                    })
                    .collect()
            }
        }
    } else {
        HashMap::new()
    };

    // 2. Calculate normalized centrality ranks (0.0..=1.0)
    let centrality_ranks: HashMap<String, f64> = if let Some(centralities) = centrality_scores {
        let max_degree = centralities.values().copied().max().unwrap_or(0);
        if max_degree == 0 {
            centralities.keys().map(|k| (k.clone(), 0.0)).collect()
        } else {
            centralities
                .iter()
                .map(|(k, &deg)| {
                    let rank = deg as f64 / max_degree as f64;
                    (k.clone(), rank)
                })
                .collect()
        }
    } else {
        HashMap::new()
    };

    // 3. Collect all paths from both maps
    let mut all_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(times) = commit_times {
        all_paths.extend(times.keys().cloned());
    }
    if let Some(centralities) = centrality_scores {
        all_paths.extend(centralities.keys().cloned());
    }

    // 4. Combine ranks using normalized weights
    let rw = recency_weight.max(0.0);
    let cw = centrality_weight.max(0.0);
    let total_weight = rw + cw;

    let mut result = HashMap::new();
    for path in all_paths {
        let r_rank = recency_ranks.get(&path).copied();
        let c_rank = centrality_ranks.get(&path).copied();

        let combined_rank = match (r_rank, c_rank) {
            (Some(r), Some(c)) => {
                if total_weight > 0.0 {
                    (rw * r + cw * c) / total_weight
                } else {
                    0.0
                }
            }
            (Some(r), None) => r,
            (None, Some(c)) => c,
            (None, None) => 0.0,
        };

        let boost = (combined_rank * max_boost as f64).round() as i32;
        result.insert(path, boost);
    }

    result
}
