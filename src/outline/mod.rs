//! Outline mode: compact structural skeletons of source files so more of a repo
//! fits in a fixed context budget. Opt-in, behind the `outline` Cargo feature.
//!
//! The engine parses a file **once** with tree-sitter, extracts declarations as
//! [`Symbol`]s that hold byte *ranges* into the original source (no copies), and
//! renders any [`OutlineLevel`] on demand by slicing that source. This keeps
//! memory low and lets the budget allocator pick a level per file cheaply.

mod extract;
mod lang;
mod render;

#[cfg(test)]
mod tests;

pub use lang::{detect_language, Language};

use std::ops::Range;

/// Levels of detail for a file, richest first. `Omit` (dropping a file) is the
/// allocator's job, not this module's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineLevel {
    /// Verbatim file content.
    Full,
    /// Signatures + declarations; function/method bodies elided.
    Outline,
    /// Like `Outline` but public/exported symbols only.
    Api,
    /// One line per symbol: kind, name, and line range.
    Symbols,
}

/// What a declaration is. Used for the `Symbols` listing and to decide rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Module,
    Import,
    Struct,
    Enum,
    Union,
    Trait,
    Impl,
    Function,
    Type,
    Const,
    Static,
    Macro,
}

impl SymbolKind {
    fn label(self) -> &'static str {
        match self {
            SymbolKind::Module => "mod",
            SymbolKind::Import => "use",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Union => "union",
            SymbolKind::Trait => "trait",
            SymbolKind::Impl => "impl",
            SymbolKind::Function => "fn",
            SymbolKind::Type => "type",
            SymbolKind::Const => "const",
            SymbolKind::Static => "static",
            SymbolKind::Macro => "macro",
        }
    }
}

/// How a symbol's body is treated below `Full`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handling {
    /// Code body replaced by an elision marker (functions/methods).
    Elide,
    /// Container: print the header, then render child items (impl/trait/mod).
    Recurse,
    /// Print the whole declaration verbatim (struct/enum/type/const/use/...).
    ShowFull,
}

/// One extracted declaration. All positions are byte ranges into the file's
/// source string, so rendering never copies text until the final slice.
#[derive(Clone, Debug)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub is_public: bool,
    pub handling: Handling,
    /// Identifier range, when the node exposes a `name` field (impl blocks do not).
    pub name: Option<Range<usize>>,
    /// Full declaration range (the item itself, excluding leading trivia).
    pub node: Range<usize>,
    /// Byte where rendering begins: `node.start`, extended back over any
    /// contiguous leading attributes (`#[...]`) and doc comments (`///`).
    pub lead_start: usize,
    /// Byte offset of the body's opening delimiter (for `Elide`/`Recurse`).
    pub body_open: Option<usize>,
    /// Number of source lines hidden by an `Elide`, for the marker.
    pub body_lines: u32,
    pub start_row: usize,
    pub end_row: usize,
    /// Nesting depth (0 = top level).
    pub depth: u16,
    /// Indices into the owning file's symbol vector.
    pub children: Vec<u32>,
}

/// Parse `source` and extract its declarations.
///
/// Returns `None` when the language is unsupported, the parse is too broken to
/// trust, or the outline would carry no signal — in every such case the caller
/// should fall back to the full file content.
pub fn extract(source: &str, lang: Language) -> Option<Vec<Symbol>> {
    extract::extract(source, lang)
}

/// Render previously-extracted `symbols` at `level` by slicing `source`.
pub fn render(source: &str, lang: Language, symbols: &[Symbol], level: OutlineLevel) -> String {
    render::render(source, lang, symbols, level)
}
