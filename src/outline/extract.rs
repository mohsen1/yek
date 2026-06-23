//! Parse a file once and extract its declarations as a flat, tree-linked
//! [`Symbol`] vector. Parsers are reused per thread.

use std::cell::RefCell;
use std::collections::HashMap;

use tree_sitter::{Node, Parser};

use super::lang::{Language, VisibilityRule};
use super::{Handling, Symbol};

/// Hard cap on declarations recorded per file, so a pathological generated file
/// cannot blow up memory or output.
const MAX_SYMBOLS: usize = 2000;

/// If more than this fraction of a file sits under error/missing nodes, the
/// parse is too unreliable to outline and we fall back to full content.
const MAX_ERROR_RATIO: f64 = 0.33;

thread_local! {
    /// One parser per (thread, language). `Parser` is `Send` but not `Sync`,
    /// so a thread-local pool is the cheap way to reuse it across `par_iter`.
    static PARSERS: RefCell<HashMap<Language, Parser>> = RefCell::new(HashMap::new());
}

pub(super) fn extract(source: &str, lang: Language) -> Option<Vec<Symbol>> {
    let tree = PARSERS.with(|cell| {
        let mut map = cell.borrow_mut();
        let parser = map.entry(lang).or_insert_with(|| {
            let mut p = Parser::new();
            p.set_language(&lang.ts_language())
                .expect("built-in grammar must load");
            p
        });
        parser.parse(source.as_bytes(), None)
    })?;

    let root = tree.root_node();
    if root.has_error() && error_ratio(root) > MAX_ERROR_RATIO {
        return None;
    }

    let mut out: Vec<Symbol> = Vec::new();
    // The outermost call's sink (top-level indices) is not needed: top-level
    // symbols are identified by `depth == 0` at render time.
    walk(root, source, lang, 0, None, &mut out, &mut Vec::new());

    // No declarations ⇒ keeping full content is strictly better.
    if out.is_empty() {
        return None;
    }
    Some(out)
}

/// Recursively record declarations. `sink` receives the indices of the symbols
/// recorded at this level (used to wire up `children`).
fn walk(
    node: Node,
    source: &str,
    lang: Language,
    depth: u16,
    parent_kind: Option<super::SymbolKind>,
    out: &mut Vec<Symbol>,
    sink: &mut Vec<u32>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if out.len() >= MAX_SYMBOLS {
            break;
        }
        let Some((kind, handling)) = lang.classify(child.kind()) else {
            continue;
        };

        let sym = build_symbol(&child, source, lang, kind, handling, depth, parent_kind);
        let recurse_into = if sym.handling == Handling::Recurse {
            child.child_by_field_name("body")
        } else {
            None
        };

        let idx = out.len() as u32;
        out.push(sym);
        sink.push(idx);

        if let Some(body) = recurse_into {
            // A trait `impl` makes its members public API, like trait members.
            let child_parent = match kind {
                super::SymbolKind::Impl if child.child_by_field_name("trait").is_some() => {
                    super::SymbolKind::Trait
                }
                other => other,
            };
            let mut children = Vec::new();
            walk(
                body,
                source,
                lang,
                depth + 1,
                Some(child_parent),
                out,
                &mut children,
            );
            out[idx as usize].children = children;
        }
    }
}

fn build_symbol(
    node: &Node,
    source: &str,
    lang: Language,
    kind: super::SymbolKind,
    handling: Handling,
    depth: u16,
    parent_kind: Option<super::SymbolKind>,
) -> Symbol {
    let name = node.child_by_field_name("name").map(|n| n.byte_range());

    let has_marker = match lang.visibility() {
        VisibilityRule::Marker(marker) => has_child_kind(node, marker),
    };
    // Members of a trait are part of its public interface even without a marker.
    let is_public = has_marker || parent_kind == Some(super::SymbolKind::Trait);

    let body = node.child_by_field_name("body");
    let (handling, body_open, body_lines) = match (handling, body) {
        (Handling::Elide, Some(b)) => (
            Handling::Elide,
            Some(b.start_byte()),
            // Lines of hidden content: the `{` and `}` lines stay, so exclude them.
            (b.end_position()
                .row
                .saturating_sub(b.start_position().row)
                .saturating_sub(1)) as u32,
        ),
        (Handling::Recurse, Some(b)) => (Handling::Recurse, Some(b.start_byte()), 0),
        // No body to elide or recurse into (e.g. `mod foo;`, `fn f();`, a struct)
        // ⇒ show the declaration verbatim.
        _ => (Handling::ShowFull, None, 0),
    };

    Symbol {
        kind,
        is_public,
        handling,
        name,
        node: node.byte_range(),
        lead_start: lead_start(node, source),
        body_open,
        body_lines,
        start_row: node.start_position().row,
        end_row: node.end_position().row,
        depth,
        children: Vec::new(),
    }
}

/// Extend a declaration's start backward over any contiguous leading attributes
/// (`#[...]`) and doc comments (`///`, `//!`, `/** */`, `/*! */`), which the
/// grammar represents as preceding siblings rather than children.
fn lead_start(node: &Node, source: &str) -> usize {
    let mut start = node.start_byte();
    let mut top_row = node.start_position().row;
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        let is_doc = matches!(s.kind(), "line_comment" | "block_comment")
            && is_doc_comment(&source[s.byte_range()]);
        if !(s.kind() == "attribute_item" || is_doc) {
            break;
        }
        // Stop if a blank line detaches the trivia from the declaration.
        if s.end_position().row + 1 < top_row {
            break;
        }
        start = s.start_byte();
        top_row = s.start_position().row;
        sibling = s.prev_sibling();
    }
    start
}

fn is_doc_comment(text: &str) -> bool {
    text.starts_with("///")
        || text.starts_with("//!")
        || text.starts_with("/**")
        || text.starts_with("/*!")
}

/// Whether `node` has a direct child of the given kind. The intermediate
/// binding is required: it drops the `Children` iterator (and its borrow of
/// `cursor`) before `cursor` itself goes out of scope.
fn has_child_kind(node: &Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|c| c.kind() == kind);
    found
}

/// Fraction of bytes covered by error/missing nodes (computed only when the tree
/// is known to contain at least one error).
fn error_ratio(root: Node) -> f64 {
    let total = root.byte_range().len().max(1);
    let mut errored = 0usize;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            errored += node.byte_range().len();
            continue; // children are already counted within this range
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    errored as f64 / total as f64
}
