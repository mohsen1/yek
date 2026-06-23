//! Per-language configuration for outline extraction.
//!
//! Languages are described as *data*: a node-kind classification table plus a
//! visibility rule and an elision style. Adding a language is adding a grammar
//! dependency and a few match arms — no new control flow.

use std::path::Path;

use super::{Handling, SymbolKind};

/// A source language yek can outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
}

/// How to decide whether a declaration is part of the public API.
#[derive(Clone, Copy)]
pub(crate) enum VisibilityRule {
    /// Public iff the node has a direct child of this kind (e.g. Rust `pub`).
    Marker(&'static str),
}

/// How elided bodies are rendered.
#[derive(Clone, Copy)]
pub(crate) enum ElisionStyle {
    /// C-like: ` { /* … N lines … */ }`.
    Braces,
}

impl Language {
    /// All languages compiled into this build.
    pub fn all() -> &'static [Language] {
        &[Language::Rust]
    }

    /// Canonical lowercase name, used by `--outline-languages` and `--json`.
    pub fn name(self) -> &'static str {
        match self {
            Language::Rust => "rust",
        }
    }

    /// Parse a user-supplied language name (for `--outline-languages`).
    pub fn from_name(name: &str) -> Option<Language> {
        match name.trim().to_ascii_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            _ => None,
        }
    }

    /// The tree-sitter grammar for this language.
    pub(crate) fn ts_language(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        }
    }

    /// Map a grammar node kind to a symbol kind and how to render its body.
    /// Returns `None` for nodes that are not outline-worthy declarations.
    pub(crate) fn classify(self, node_kind: &str) -> Option<(SymbolKind, Handling)> {
        match self {
            Language::Rust => match node_kind {
                "function_item" => Some((SymbolKind::Function, Handling::Elide)),
                // Bodyless trait method / associated declarations: show verbatim.
                "function_signature_item" => Some((SymbolKind::Function, Handling::ShowFull)),
                "associated_type" => Some((SymbolKind::Type, Handling::ShowFull)),
                // `macro_definition` has no `body` field, so its rules cannot be
                // elided generically yet; show it verbatim rather than pretending.
                "macro_definition" => Some((SymbolKind::Macro, Handling::ShowFull)),
                "struct_item" => Some((SymbolKind::Struct, Handling::ShowFull)),
                "enum_item" => Some((SymbolKind::Enum, Handling::ShowFull)),
                "union_item" => Some((SymbolKind::Union, Handling::ShowFull)),
                "type_item" => Some((SymbolKind::Type, Handling::ShowFull)),
                "const_item" => Some((SymbolKind::Const, Handling::ShowFull)),
                "static_item" => Some((SymbolKind::Static, Handling::ShowFull)),
                "use_declaration" => Some((SymbolKind::Import, Handling::ShowFull)),
                "trait_item" => Some((SymbolKind::Trait, Handling::Recurse)),
                "impl_item" => Some((SymbolKind::Impl, Handling::Recurse)),
                "mod_item" => Some((SymbolKind::Module, Handling::Recurse)),
                _ => None,
            },
        }
    }

    pub(crate) fn visibility(self) -> VisibilityRule {
        match self {
            Language::Rust => VisibilityRule::Marker("visibility_modifier"),
        }
    }

    pub(crate) fn elision(self) -> ElisionStyle {
        match self {
            Language::Rust => ElisionStyle::Braces,
        }
    }
}

/// Detect a supported language from a file path's extension.
pub fn detect_language(rel_path: &str) -> Option<Language> {
    let ext = Path::new(rel_path).extension()?.to_str()?;
    match ext {
        "rs" => Some(Language::Rust),
        _ => None,
    }
}
