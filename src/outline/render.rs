//! Render extracted symbols at a chosen level by slicing the original source.
//!
//! Each declaration is sliced from its own start token and re-indented by its
//! nesting depth, so the output reads like the source with bodies elided and
//! stays correct even when several declarations share a source line
//! (e.g. `trait T { fn go(); }`).

use std::fmt::Write;

use super::lang::{ElisionStyle, Language};
use super::{Handling, OutlineLevel, Symbol};

/// Spaces used per nesting level when re-indenting rendered declarations.
const INDENT: &str = "    ";

pub(super) fn render(
    source: &str,
    lang: Language,
    symbols: &[Symbol],
    level: OutlineLevel,
) -> String {
    match level {
        OutlineLevel::Full => source.to_string(),
        OutlineLevel::Outline => render_tree(source, lang, symbols, false),
        OutlineLevel::Api => render_tree(source, lang, symbols, true),
        OutlineLevel::Symbols => render_symbols(source, symbols),
    }
}

fn render_tree(source: &str, lang: Language, symbols: &[Symbol], api_only: bool) -> String {
    let mut buf = String::new();
    for (i, sym) in symbols.iter().enumerate() {
        if sym.depth != 0 {
            continue; // nested items are emitted by their parent
        }
        if api_only && !subtree_visible(symbols, i) {
            continue;
        }
        render_one(source, lang, symbols, i, api_only, &mut buf);
    }
    buf
}

fn render_one(
    source: &str,
    lang: Language,
    symbols: &[Symbol],
    idx: usize,
    api_only: bool,
    buf: &mut String,
) {
    let sym = &symbols[idx];
    // Re-indent by depth rather than copying the source's leading whitespace, so
    // output stays correct when several declarations share a line.
    let indent = INDENT.repeat(sym.depth as usize);

    match sym.handling {
        Handling::ShowFull => {
            buf.push_str(&indent);
            buf.push_str(&source[sym.lead_start..sym.node.end]);
            buf.push('\n');
        }
        Handling::Elide => {
            let open = sym.body_open.unwrap_or(sym.node.end);
            buf.push_str(&indent);
            buf.push_str(source[sym.lead_start..open].trim_end());
            buf.push_str(&elide_marker(lang, sym.body_lines));
            buf.push('\n');
        }
        Handling::Recurse => {
            let open = sym.body_open.unwrap_or(sym.node.end);
            buf.push_str(&indent);
            buf.push_str(source[sym.lead_start..open].trim_end());
            buf.push_str(" {\n");
            for &child in &sym.children {
                if api_only && !subtree_visible(symbols, child as usize) {
                    continue;
                }
                render_one(source, lang, symbols, child as usize, api_only, buf);
            }
            buf.push_str(&indent);
            buf.push_str("}\n");
        }
    }
}

fn render_symbols(source: &str, symbols: &[Symbol]) -> String {
    let mut buf = String::new();
    for sym in symbols {
        let pad = "  ".repeat(sym.depth as usize);
        let _ = writeln!(
            buf,
            "{}{} (L{}-{})",
            pad,
            symbol_label(source, sym),
            sym.start_row + 1,
            sym.end_row + 1,
        );
    }
    buf
}

/// Display label for the `Symbols` listing. Named declarations read as
/// `kind name` (e.g. `fn area`); nameless ones (`impl`, `use`) fall back to the
/// first line of the declaration, which already carries the keyword.
fn symbol_label(source: &str, sym: &Symbol) -> String {
    if let Some(name) = &sym.name {
        return format!("{} {}", sym.kind.label(), &source[name.clone()]);
    }
    let end = sym.body_open.unwrap_or(sym.node.end);
    let head = &source[sym.node.start..end];
    head.split(['\n', '{', '('])
        .next()
        .unwrap_or(head)
        .trim()
        .to_string()
}

fn elide_marker(lang: Language, lines: u32) -> String {
    match lang.elision() {
        ElisionStyle::Braces => {
            if lines > 1 {
                format!(" {{ /* … {lines} lines … */ }}")
            } else {
                " { /* … */ }".to_string()
            }
        }
    }
}

/// Is `idx` shown at `Api` level? A leaf is shown when public; a container is
/// shown when public or when it holds any visible descendant.
fn subtree_visible(symbols: &[Symbol], idx: usize) -> bool {
    let sym = &symbols[idx];
    if sym.handling != Handling::Recurse {
        return sym.is_public;
    }
    if sym.is_public {
        return true;
    }
    sym.children
        .iter()
        .any(|&c| subtree_visible(symbols, c as usize))
}
