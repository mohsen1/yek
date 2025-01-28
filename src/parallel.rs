use crate::{config::FullYekConfig, priority::get_file_priority, Result};
use content_inspector::{inspect, ContentType};
use path_slash::PathBufExt;
use rayon::prelude::*;
use regex::Regex;
use std::sync::mpsc;
use std::{collections::HashMap, fs, path::Path};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ProcessedFile {
    pub priority: i32,
    pub file_index: usize,
    #[allow(unused)]
    pub rel_path: String,
    pub content: String,
}

pub fn process_files_parallel(
    base_dir: &Path,
    config: &FullYekConfig,
    boost_map: &HashMap<String, i32>,
) -> Result<Vec<ProcessedFile>> {
    let mut walk_builder = ignore::WalkBuilder::new(base_dir);
    walk_builder
        .hidden(true)
        .git_ignore(true)
        .follow_links(false)
        .standard_filters(true)
        .require_git(false);

    let (processed_files_tx, processed_files_rx) = mpsc::channel();

    walk_builder.build_parallel().run(|| {
        Box::new(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return ignore::WalkState::Continue,
            };
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                return ignore::WalkState::Continue;
            }

            let path = entry.path().to_path_buf();
            let rel_path = normalize_path(&path, base_dir);

            processed_files_tx.send((path, rel_path)).unwrap();
            ignore::WalkState::Continue
        })
    });

    let processed_files = processed_files_rx
        .iter()
        .filter_map(|(path, rel_path)| {
            let ignore_patterns = config.ignore_patterns.clone();
            let is_ignored = ignore_patterns
                .iter()
                .any(|p| Regex::new(p).map_or(false, |re| re.is_match(&rel_path)));
            if is_ignored {
                debug!("Skipping {} - matched ignore pattern", rel_path);
                return None;
            }

            if let Ok(content) = fs::read(&path) {
                if inspect(&content) == ContentType::BINARY {
                    debug!("Skipping binary file: {}", rel_path);
                    return None;
                }

                let rule_priority = get_file_priority(&rel_path, &config.priority_rules);
                let boost = boost_map.get(&rel_path).copied().unwrap_or(0);
                let combined_priority = rule_priority + boost;

                Some(ProcessedFile {
                    priority: combined_priority,
                    file_index: 0, // Placeholder, will be updated later
                    rel_path,
                    content: String::from_utf8_lossy(&content).to_string(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Assign unique file_index within each priority group
    let mut file_index_counters = HashMap::new();
    let mut results = processed_files
        .into_iter()
        .map(|mut file| {
            let counter = file_index_counters.entry(file.priority).or_insert(0);
            file.file_index = *counter;
            *counter += 1;
            file
        })
        .collect::<Vec<_>>();

    debug!("Processed {} files in parallel", results.len());

    results.par_sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .reverse()
            .then_with(|| a.file_index.cmp(&b.file_index))
    });

    Ok(results)
}

/// Returns a relative, normalized path string (forward slashes on all platforms).
pub fn normalize_path(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_path_buf()
        .to_slash()
        .unwrap_or_default()
        .to_string()
}
