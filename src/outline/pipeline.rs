//! Apply outline rendering to processed files according to the configured mode.
//!
//! Runs after files are read and before the budget step in `concat_files`.

use std::str::FromStr;

use bytesize::ByteSize;
use rayon::prelude::*;

use super::{detect_language, extract, render, Language, OutlineLevel};
use crate::config::{OutlineFallback, OutlineLevel as CfgLevel, OutlineMode, YekConfig};
use crate::parallel::ProcessedFile;

/// Transform `files` in place according to `config`'s outline mode.
pub fn apply(files: &mut Vec<ProcessedFile>, config: &YekConfig) {
    let level = cfg_to_level(config.outline_level());
    let tag = tag(config.outline_level());
    let restrict = &config.outline_languages;

    match config.outline_mode() {
        OutlineMode::Off => {}
        OutlineMode::Always => {
            files.par_iter_mut().for_each(|f| {
                if let Some(rendered) = outline_one(&f.rel_path, &f.content, restrict, level) {
                    f.content = display(config, tag, &rendered);
                    f.outline_level = Some(tag);
                }
            });
            if config.outline_fallback() == OutlineFallback::Omit {
                files.retain(|f| f.outline_level.is_some());
            }
        }
        OutlineMode::Degrade => degrade(files, config, level, tag),
    }
}

/// Budget-aware (simple): keep the highest-priority files at full content until
/// roughly half the budget is spent, then outline the remaining supported files.
/// The final hard cap is still enforced by `concat_files`.
fn degrade(files: &mut [ProcessedFile], config: &YekConfig, level: OutlineLevel, tag: &'static str) {
    // Pre-render outlines in parallel; only supported files yield `Some`.
    let outlines: Vec<Option<String>> = files
        .par_iter()
        .map(|f| outline_one(&f.rel_path, &f.content, &config.outline_languages, level))
        .collect();

    let full_budget = budget(config) / 2;

    // Decide in priority-descending order so important files keep full content.
    let mut order: Vec<usize> = (0..files.len()).collect();
    order.sort_by(|&a, &b| {
        files[b]
            .priority
            .cmp(&files[a].priority)
            .then_with(|| files[a].rel_path.cmp(&files[b].rel_path))
    });

    let mut used = 0usize;
    for i in order {
        let full_cost = cost(config, &files[i].content);
        if used + full_cost <= full_budget {
            used += full_cost; // keep full content
        } else if let Some(rendered) = &outlines[i] {
            files[i].content = display(config, tag, rendered);
            files[i].outline_level = Some(tag);
        }
        // Unsupported files that don't fit stay full; `concat_files` caps them.
    }
}

/// Render `content` as an outline, or `None` if the language is unsupported (or
/// excluded by `restrict`), the parse is unusable, or there is nothing to show.
fn outline_one(
    rel_path: &str,
    content: &str,
    restrict: &[String],
    level: OutlineLevel,
) -> Option<String> {
    let lang = detect_language(rel_path)?;
    if !restrict.is_empty() && !restrict.iter().any(|r| Language::from_name(r) == Some(lang)) {
        return None;
    }
    let symbols = extract(content, lang)?;
    Some(render(content, lang, &symbols, level))
}

/// Text output marks outlined files so a reader knows the content is abbreviated;
/// JSON output omits the marker and relies on the structured `level` field.
fn display(config: &YekConfig, tag: &str, rendered: &str) -> String {
    if config.json {
        rendered.to_string()
    } else {
        format!("⟪yek:{tag}⟫\n{rendered}")
    }
}

fn budget(config: &YekConfig) -> usize {
    if config.token_mode {
        crate::parse_token_limit(&config.tokens).unwrap_or(usize::MAX)
    } else {
        ByteSize::from_str(&config.max_size)
            .map(|b| b.as_u64() as usize)
            .unwrap_or(usize::MAX)
    }
}

/// Approximate per-file cost. This counts the raw content only, not the template
/// or JSON wrapper that `concat_files` adds, so it slightly under-counts. The
/// `cap / 2` headroom for full content plus `concat_files`' exact final cap make
/// that approximation safe; a future budget allocator can cost levels exactly.
fn cost(config: &YekConfig, content: &str) -> usize {
    if config.token_mode {
        crate::count_tokens(content)
    } else {
        content.len()
    }
}

fn cfg_to_level(level: CfgLevel) -> OutlineLevel {
    match level {
        CfgLevel::Outline => OutlineLevel::Outline,
        CfgLevel::Api => OutlineLevel::Api,
        CfgLevel::Symbols => OutlineLevel::Symbols,
    }
}

fn tag(level: CfgLevel) -> &'static str {
    match level {
        CfgLevel::Outline => "outline",
        CfgLevel::Api => "api",
        CfgLevel::Symbols => "symbols",
    }
}
