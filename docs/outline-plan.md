# Plan: Outline mode — squeeze more of a repo into a context budget

> Status: proposal / design doc
> Tracking issue: [#245 "Support lazy retrieval"](https://github.com/mohsen1/yek/issues/245) (related: [#232 "MCP for Yek"](https://github.com/mohsen1/yek/issues/232))
> Inspiration: [ast-grep `outline`](https://ast-grep.github.io/blog/ast-grep-outline.html)

## 1. Summary

Add an **outline mode** to `yek` that emits a structural *skeleton* of source
files — declarations, signatures, types, imports/exports — with bodies elided,
instead of (or alongside) full file content. Outlines are far cheaper in tokens
than full source, so a fixed `--tokens` budget can cover a much larger slice of
a repository while preserving the "map" an LLM needs to reason about the code.

The headline capability is **budget-aware degradation**: when the repo does not
fit in the budget, instead of dropping whole files (today's behavior), `yek`
downgrades the least important files to their outline (and, if needed, to a bare
symbol index) so the most important files keep full fidelity and everything else
still contributes structural signal.

ast-grep's own benchmarks for the same idea report **35–55% median token
reductions** on large repos while keeping 100% symbol coverage. That is the win
we are chasing.

## 2. Motivation

`yek` exists to pack a repository into an LLM context window. Today it operates
at **whole-file granularity**: a file is either included verbatim or omitted.
Under a tight `--tokens` budget that is wasteful — a 2,000-line file might cost
20k tokens, but its 60 function signatures cost ~800 tokens and still tell the
model what the file *is*, what it exposes, and where to look next.

The [ast-grep outline post](https://ast-grep.github.io/blog/ast-grep-outline.html)
frames the same observation: agents waste tokens opening whole files just to
learn their structure. A compact, syntax-aware "table of contents" (declarations
+ source ranges) lets an agent move from *directory → symbols → targeted slice*
incrementally, instead of paying for full reads up front.

For `yek` this unlocks two things:

1. **More repo per budget.** Fit the structure of a whole codebase into the same
   window that previously held a handful of files.
2. **A foundation for lazy retrieval** (issue #245 / CodeRLM-style): outline
   first, then fetch full bodies for specific symbols on demand — naturally
   exposed over MCP (issue #232).

## 3. How `yek` works today (the parts this touches)

```
init_config (config.rs)
  └─ serialize_repo (lib.rs)
       ├─ get_recent_commit_times_git2 / compute_recentness_boost (priority.rs)
       ├─ process_files_parallel (parallel.rs)
       │     → walk, skip ignored + binary, read FULL content
       │     → Vec<ProcessedFile { priority, file_index, rel_path, content }>
       ├─ sort by (priority asc, rel_path)
       └─ concat_files (lib.rs)
             → greedily accumulate until token/byte cap, then stop
```

Key facts that shape this design:

- **`ProcessedFile.content` is the full file text** (`parallel.rs`). There is no
  per-file representation other than "the whole thing."
- **`concat_files` enforces the budget at whole-file granularity** (`lib.rs:111`).
  It sorts files and includes them until the next file would exceed `cap`, then
  `break`s. There is no notion of a cheaper representation of a file.
- **`yek` is intentionally language-agnostic.** `parallel.rs` reads bytes and
  only distinguishes text vs binary (`content_inspector`). Nothing parses code.
- The output is rendered through a **template** (`output_template`, default
  `>>>> FILE_PATH\nFILE_CONTENT`), or JSON when `--json` is set.

> Note: the current allocator iterates in **ascending** priority and `break`s at
> the first file that overflows, so under a tight budget it can drop the
> *highest*-priority files (they sort last). The new allocator below processes
> **most-important-first** and degrades the rest, which also corrects this edge.

## 4. Constraints from the maintainer (issue #245)

The owner already tried this manually and flagged the real risks
([comment](https://github.com/mohsen1/yek/issues/245#issuecomment-3900331706)):

> "Building project skeleton is something I have tried manually and was
> successful using ast-grep. need to design it well. sometimes projects are too
> big that even the skeleton won't fit in context so we should be smart about
> what is important. Yek is not language specific and this would add 'supported
> languages' which is a concern."

Three hard constraints fall out of that, and the design is organized around them:

| Concern | Design response |
| --- | --- |
| **"Yek is not language specific."** Adding parsers risks changing the project's identity and bloating the default binary. | Outline is **opt-in** and **additive**, gated behind a Cargo feature (`outline`). The default build stays byte-only and language-agnostic. Unsupported languages **gracefully fall back** to existing behavior. |
| **"Even the skeleton won't fit."** | **Multiple levels of detail** (full → outline → public-API-only → symbol index) plus a **budget-aware degradation allocator** that spends fidelity on high-priority files first. |
| **"Need to design it well."** | A small, isolated `outline` module with a single trait boundary; per-language extractors are data (queries), not code; everything is tested with fixtures and benchmarked. |

## 5. Core concept: Levels of Detail (LoD)

Define an ordered set of representations for a file, cheapest last:

| Level | Name | What it contains | Rough cost |
| --- | --- | --- | --- |
| L0 | `Full` | Verbatim file content (today). | 100% |
| L1 | `Outline` | Imports/exports, type/class/struct/enum decls, function & method **signatures**, leading doc comments; bodies replaced by an elision marker. | ~10–25% |
| L2 | `Api` | Same as L1 but **public/exported symbols only** (drop private). | ~5–15% |
| L3 | `Symbols` | File path + flat list of top-level symbol names (+ line ranges). | ~1–3% |
| L4 | `Omit` | File dropped entirely (today's only fallback). | 0% |

L2 leans on the visibility/export metadata the ast-grep blog describes
(extraction rules carry a "public/export" flag). Being able to keep *public API*
and drop private internals is exactly the "be smart about what's important"
lever the maintainer asked for.

Example (Rust), L1 outline:

```rust
>>>> src/parallel.rs (outline)
use crate::{config::YekConfig, priority::get_file_priority, Result};
use rayon::prelude::*;

pub struct ProcessedFile {
    pub priority: i32,
    pub file_index: usize,
    pub rel_path: String,
    pub content: String,
}

fn process_single_file(file_path: &Path, config: &YekConfig, boost_map: &HashMap<String, i32>) -> Result<Vec<ProcessedFile>> { /* … 51 lines … */ }
pub fn process_files_parallel(base_path: &Path, config: &YekConfig, boost_map: &HashMap<String, i32>) -> Result<Vec<ProcessedFile>> { /* … 33 lines … */ }
pub fn normalize_path(path: &Path, base: &Path) -> String { /* … 9 lines … */ }
```

The elision marker (`/* … N lines … */`, `# … N lines …`, etc.) keeps line
counts so the model knows how much was hidden and the output stays valid-ish for
the language.

## 6. Technical approach for extraction

We need to turn source text into an outline per language. Three options:

**Option A — Embed tree-sitter + grammars + `tags.scm` queries (recommended).**
Tree-sitter already ships community `tags.scm` queries designed for exactly this
(ctags-style symbol extraction). We embed `tree-sitter` plus a curated set of
grammar crates and run a tags/outline query per file.
- ➕ Single static binary, no runtime dependency, fast, matches `yek`'s ethos.
- ➕ Queries are *data* — adding a language is adding a grammar crate + a `.scm`.
- ➖ Heavier dependency tree, bigger binary, longer compile (mitigated by the
  Cargo feature gate so default builds are unaffected).

**Option B — `ast-grep-core` + grammars, with declarative rules.**
Use ast-grep's Rust crates and ship YAML extraction rules per language (the
exact mechanism in the blog). Very close to Option A technically; the rules are
arguably more readable/overridable than `.scm`, and we inherit ast-grep's
range/metadata model.
- ➕ User-overridable rules in `yek.yaml` fit `yek`'s config-driven design.
- ➖ Same binary-size cost; ties us to ast-grep's crate API surface.

**Option C — Shell out to the `ast-grep` CLI `outline` command.**
- ➕ Least code; reuse a maintained implementation.
- ➖ Re-introduces an external runtime dependency, breaks single-binary
  distribution, and adds process-spawn overhead. **Rejected** — it conflicts
  with `yek`'s "one fast binary" value.

**Recommendation:** Option A as the engine, with Option B's *user-overridable
rules* idea layered on later (Phase 4). Hide a trait so the engine choice stays
swappable:

```rust
// src/outline/mod.rs
pub enum OutlineLevel { Full, Outline, Api, Symbols, Omit }

pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,      // Function, Method, Class, Struct, Enum, Trait, Type, Const, Import…
    pub is_public: bool,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: String,     // text from decl start to body open
}

pub trait OutlineExtractor: Send + Sync {
    fn language(&self) -> Language;
    fn symbols(&self, source: &str) -> Vec<Symbol>;
}

/// Render a file at a given level. Returns None if the language is unsupported,
/// signalling the caller to fall back to Full content.
pub fn render(source: &str, lang: Language, level: OutlineLevel) -> Option<String>;

/// Map an extension to a supported language, or None.
pub fn detect_language(rel_path: &str) -> Option<Language>;
```

Language detection reuses the extension-driven style already in `defaults.rs`.

### Initial language set (Phase 1–2)

Rust, TypeScript/JavaScript (+ TSX/JSX), Python, Go, Java. These cover the bulk
of real usage and all have mature tree-sitter grammars and `tags.scm` queries.
Everything else falls back to L0 (or is configurable).

## 7. Pipeline integration

Outline extraction is **per-file and embarrassingly parallel**, so it slots into
the existing rayon/threaded walk in `parallel.rs` with no architectural change.

1. `ProcessedFile` gains lazily-computed alternate representations. To avoid
   parsing when outline is off, keep `content` as-is and add an optional,
   on-demand outline cache:

   ```rust
   pub struct ProcessedFile {
       pub priority: i32,
       pub file_index: usize,
       pub rel_path: String,
       pub content: String,                 // L0, as today
       pub language: Option<Language>,      // detected once
       // computed only when outline mode is active:
       pub outline: Option<String>,         // L1
       pub api: Option<String>,             // L2
       pub symbols: Option<String>,         // L3
   }
   ```

2. When outline mode is active, the worker that already reads the file also runs
   the extractor and fills the alternate representations. Parsing only happens
   for supported languages and only when requested.

3. `concat_files` is replaced/extended by the **degradation allocator** (§9).

> Refined in §15.3/§15.6: store `Option<Vec<Symbol>>` (parse once, byte ranges,
> no string copies) and render the *chosen* level lazily, rather than caching
> three rendered strings per file — a memory and token-counting win.

This keeps the hot path for the default (outline-off) build byte-for-byte
identical to today.

## 8. CLI & config surface

```text
--outline                     Shorthand for --outline-mode always
--outline-mode <MODE>         off | always | degrade           [default: off]
--outline-level <LEVEL>       outline | api | symbols           [default: outline]
--outline-languages <L>...    Restrict to a subset (e.g. rust,ts,python)
--outline-fallback <F>        full | omit  (unsupported langs)  [default: full]
```

- **`off`** — today's behavior, no parsing. (Default; protects the
  language-agnostic identity.)
- **`always`** — every supported file rendered at `--outline-level`; unsupported
  files follow `--outline-fallback`. Great for "give me a map of this repo."
- **`degrade`** — the budget-aware mode (§9): full content for as many
  high-priority files as fit, outlines/symbols for the rest. The flagship.

`yek.yaml` equivalents (config-only keys, consistent with existing style):

```yaml
outline_mode: degrade           # off | always | degrade
outline_level: outline          # outline | api | symbols
outline_languages: [rust, typescript, python, go, java]
outline_fallback: full          # full | omit
# Phase 4 — user-overridable extraction rules:
# outline_rules:
#   rust:
#     - kind: function
#       public_when: "pub"
```

## 9. Budget-aware degradation allocator (the core win)

Replace the greedy single-pass cut in `concat_files` with an allocator that
degrades least-important files instead of dropping them. Sketch:

```text
INPUT: files sorted by priority DESC (most important first), token budget `cap`
levels = [Full, Outline, Api, Symbols]    # Omit is the implicit floor

# 1. Seed every file at its cheapest non-omitted level and verify the floor fits.
assign each file = Symbols
if sum(cost) > cap:
    drop lowest-priority files (Symbols → Omit) until sum(cost) <= cap

# 2. Greedily upgrade, most-important-first, as long as budget allows.
for file in files (priority DESC):
    for level in [Api, Outline, Full]:          # progressively richer
        delta = cost(file, level) - cost(file, current_level(file))
        if used + delta <= cap:
            upgrade file to level; used += delta
        else:
            break
```

Properties:

- **Important files get full fidelity**; the tail degrades to outline/symbols
  rather than vanishing. This is the "be smart about what is important" answer.
- Reuses the existing **priority signal** (path rules + git recency) unchanged —
  outline is a new *axis* (how much of each file), orthogonal to *which* files.
- Costs are measured with the existing `count_tokens` (tiktoken) in token mode,
  or byte length in byte mode, so it composes with `--tokens` and `--max-size`.
- Degenerate case `outline_mode = off` ⇒ only `Full`/`Omit` exist ⇒ behaves like
  today (and fixes the most-important-first ordering noted in §3).

A simpler v1 that still delivers most of the value: **"full for high priority,
outline for the rest"** — include full content top-down until the budget is, say,
60% spent, then switch every remaining file to outline, then to symbols, then
stop. Ship that first; generalize to the upgrade loop in Phase 3.

> The production allocator — single-pass suffix-floor, lazy/memoized token
> counting, and the "even the skeleton won't fit" global fallback — is specified
> with perf bounds in §15.7. The seed-then-upgrade sketch above recomputes costs
> repeatedly; §15.7 avoids that.

## 10. Output format

So the model (and humans) can tell a file was abbreviated, annotate the template
context. Add an optional `LEVEL` placeholder and a default suffix marker:

```text
>>>> src/parallel.rs
⟪outline: bodies elided⟫
…signatures…
```

- Default template stays `>>>> FILE_PATH\nFILE_CONTENT`, and **`FILE_PATH` is
  never mutated** — suffixing it (e.g. `(outline)`) would break round-tripping
  (`unyek`, issue #64) and any path-keyed consumer. Instead, expose a new
  `LEVEL` placeholder that expands to `""` for full files and ` (outline)` etc.
  for abbreviated ones, opt-in via custom templates (see §15.9).
- For LLM legibility, abbreviated files get an in-block hint line
  (`⟪outline: bodies elided⟫`) so the model knows the content is partial without
  inspecting the header.
- `--json` output gains a `"level": "full|outline|api|symbols|index"` field per
  object so programmatic consumers (and an MCP server) can distinguish
  abbreviated entries; full files keep today's shape (the field is additive).

## 11. Performance

`yek`'s headline is speed; outline must not regress the default path.

- Parsing happens **only** when outline mode is active and **only** for supported
  languages, inside the existing parallel workers. Default build = no parser code
  at all (feature-gated), zero overhead.
- Add criterion benchmarks in `benches/serialization.rs` for an outline run over
  a mixed-language fixture tree, tracking both wall-clock and output token count
  (to demonstrate the budget win, not just speed).
- Cache compiled tree-sitter languages/queries in a `OnceLock` (mirrors the
  existing `TOKENIZER` pattern in `lib.rs`).
- Token counting for multiple levels per file can be deferred: compute a level's
  cost only when the allocator actually considers that level.

## 12. Testing

- **Per-language fixtures**: small source files → asserted outlines, one per
  supported language (mirrors the `tests/*_test.rs` layout).
- **Fallback**: unsupported extension (e.g. `.toml`, `.md`) stays L0 under
  `always`; `--outline-fallback omit` drops it.
- **Allocator**: synthetic files with known token costs assert that (a) the
  budget is never exceeded, (b) higher-priority files end at richer levels than
  lower-priority ones, (c) `off` mode reproduces current output exactly.
- **Determinism**: stable ordering and stable elision markers (snapshot tests).
- **Property test**: outline cost ≤ full cost for every fixture.

## 13. Phased rollout

| Phase | Deliverable |
| --- | --- |
| **1. Scaffold** | `src/outline/` module, `Language` + `detect_language`, tree-sitter behind Cargo feature `outline`, **Rust** extractor, `--outline` / `--outline-mode always`, `(outline)` marker, fixtures. |
| **2. Languages** | TS/JS(+TSX), Python, Go, Java extractors; `--outline-languages`, `--outline-fallback`. |
| **3. Degradation** | The budget-aware allocator + L2 `api` / L3 `symbols`; `--outline-mode degrade`; allocator tests + benchmark proving the token win. |
| **4. Custom rules** | User-overridable extraction rules in `yek.yaml` (Option B layer); document how to add a language without a code change. |
| **5. Lazy retrieval / MCP** | Outline-first serialization + an on-demand "expand symbol/file" path, exposed as MCP tools (issues #245, #232): return the repo outline, let the agent pull full bodies for chosen symbols. |

Phases 1–3 deliver the full standalone value. Phases 4–5 connect it to the
broader roadmap.

## 14. Risks & open questions

- **Binary size / build time.** Grammar crates are large. Mitigation: Cargo
  feature gate (default off), curated language set, document a slim vs. full
  build. Open question: should release artifacts ship `outline` on by default?
- **Grammar/version drift.** Tree-sitter grammars update independently. Pin
  versions; treat outline output as best-effort (always falls back to L0).
- **Elision correctness.** Replacing bodies can produce technically-invalid
  source. We optimize for LLM legibility, not compilability; markers make the
  elision explicit. Acceptable.
- **Token accounting cost.** Counting several levels per file adds tiktoken work.
  Mitigation: lazy per-level costing inside the allocator.
- **Scope of "public".** Visibility rules differ per language (Rust `pub`, JS
  `export`, Python `_name` convention, Go capitalization). Encode per-extractor;
  document the heuristic per language.

## 15. Implementation plan (detailed)

This section turns §5–§11 into concrete, perf-bounded engineering work, and folds
in the gaps found while detailing it (§15.12).

### 15.1 Dependencies, build, and binary-size strategy

Engine = `tree-sitter` + per-language outline queries, **gated behind a Cargo
feature** so the default build is byte-identical to today.

```toml
[features]
default = []
outline = [
  "dep:tree-sitter", "dep:tree-sitter-rust", "dep:tree-sitter-typescript",
  "dep:tree-sitter-javascript", "dep:tree-sitter-python",
  "dep:tree-sitter-go", "dep:tree-sitter-java",
]

[dependencies]
tree-sitter      = { version = "0.24", optional = true }   # representative; pin a tested set
tree-sitter-rust = { version = "0.23", optional = true }
# … one optional dep per grammar …
```

Hard rules (each closes a gap):

- **ABI pinning.** Core and every grammar must agree on the tree-sitter language
  ABI. Pin an exact, tested set; prefer grammar crates that depend on the
  lightweight `tree-sitter-language` shim (decouples them from a specific core
  version). A CI job builds `--features outline` and smoke-parses one fixture per
  language so an ABI bump fails loudly instead of at runtime.
- **C toolchain on every release target.** Grammars compile C via `cc`. yek's
  release matrix (Build/Stress per target: apple-darwin aarch64/x86, linux
  gnu/**musl**, **windows-msvc**) all have a C compiler on GH runners and in
  `cross` images — but musl and msvc grammar builds must be verified in CI before
  the feature is enabled on prebuilt artifacts.
- **Binary size / compile time.** Each grammar adds ~0.5–2 MB and C compile time;
  with the existing `lto = true` + `strip = true` profile the 5-language set is a
  few MB. Keep the feature **off by default**; decide separately whether prebuilt
  release binaries ship it on (recommended: yes for prebuilt, off for the
  crates.io default so `cargo install yek` stays slim).
- **`panic = "abort"`.** Extraction must never panic — every tree-sitter call
  returns `Option`/`Result` and falls back to L0.

Built **without** the feature, any `--outline-mode` other than `off` prints a
one-line stderr warning and proceeds as `off` (no hard error → scripts stay
portable).

### 15.2 Module layout

```
src/outline/
  mod.rs        # public API: detect_language, extract, render, OutlineLevel, Symbol
  lang.rs       # Language enum, extension→Language, per-language LanguageConfig (data)
  registry.rs   # OnceLock registry of CompiledLanguage (holds the compiled Query)
  extract.rs    # parse-once → Vec<Symbol> (ranges only, no copies)
  render.rs     # render-at-level (slices content by range), elision, nesting
  queries/      # rust.scm typescript.scm javascript.scm python.scm go.scm java.scm
```

`.scm` files are `include_str!`-embedded (zero runtime file IO).

### 15.3 Core types

```rust
pub enum OutlineLevel { Full, Outline, Api, Symbols }   // Omit handled by allocator

pub enum SymbolKind { Module, Import, Class, Struct, Enum, Trait, Interface,
                      Function, Method, Type, Const, Field, Macro, Other }

pub struct Symbol {
    kind: SymbolKind,
    is_public: bool,
    name_range: Range<usize>,    // byte ranges INTO ProcessedFile.content — no String copies
    sig_range: Range<usize>,     // decl start .. body open (whole decl if bodyless)
    doc_range: Option<Range<usize>>,
    body_open: Option<usize>,    // byte offset where body starts (None ⇒ no body to elide)
    start_row: usize, end_row: usize,   // for the "… N lines …" marker (free from nodes)
    depth: u16,                  // nesting depth → indentation
    children: Vec<u32>,          // indices into this file's symbol vec (a tree)
}
```

Storing **ranges, not strings** is the key memory decision: file content already
lives in `ProcessedFile.content`, so every level renders by slicing it. Per-file
overhead is one small `Vec<Symbol>`.

### 15.4 Extraction (parse once per file)

A single, data-driven code path. Each language ships one `.scm` using a uniform
capture protocol:

| Capture | Meaning |
| --- | --- |
| `@item` | the whole declaration node |
| `@item.name` | identifier to display |
| `@item.body` | body node to elide (absent ⇒ no body, e.g. a type alias) |
| `@item.doc` | leading doc-comment node(s) |

`SymbolKind` is fixed by *which* pattern matched; `is_public` comes from a
per-language rule (this is the gap "visibility differs per language"):

```rust
enum VisibilityRule {
    Marker(&'static str),  // Rust: child node kind "visibility_modifier"
    Exported,              // JS/TS: nearest ancestor is export_statement
    Capitalized,           // Go: identifier starts uppercase
    Underscore,            // Python: name does not start with '_'
    AlwaysPublic,
}
```

Algorithm (`extract.rs`), per file, on a worker thread:

1. Look up `CompiledLanguage` from the registry (`OnceLock`; query compiled once).
2. Thread-local `Parser` (set language once per thread); `parse(content.as_bytes(), None)`.
3. If a high fraction of bytes are under `ERROR` nodes ⇒ return `None` (caller
   keeps L0). A few `ERROR` nodes are tolerated.
4. Run the outline `Query` with a **reused** `QueryCursor`; collect flat `Symbol`s.
5. Rebuild nesting: sort by `sig_range.start`, push/pop a stack by end-byte
   containment to set `depth`/`children`.
6. Guard rails: cap at `MAX_SYMBOLS` (~2000) per file; beyond it keep top-level
   only and append a synthetic "… +N nested elided".
7. Empty/near-empty (≤1 trivial symbol) ⇒ return `None` (outline wouldn't save
   tokens; keeping L0 is strictly better).

Perf: parse is O(bytes) at tree-sitter's MB/s; the `Query` compiles **once per
language** and is shared (`&Query: Sync`); `QueryCursor` is reused; no source
text is copied.

### 15.5 Rendering (render the chosen level only)

`render(content, &[Symbol], level) -> String`, slicing `content` by range:

- **Full** — content verbatim.
- **Outline (L1)** — preorder over the symbol tree, indented by `depth`: emit
  `@doc` (if any) + `content[sig_range]`; if `body_open` is `Some`, append the
  language's elision marker carrying the hidden line count
  (`{ /* … 42 lines … */ }` for brace langs, `: …  # 42 lines` for Python).
  Container bodies (class/module) are **not** elided — we recurse into members.
- **Api (L2)** — L1 minus non-public symbols (and their private-only subtrees),
  and without `@doc`.
- **Symbols (L3)** — one line per symbol: `kind name (Lstart–Lend)`, indented.

### 15.6 Pipeline integration (parallel, perf-first)

The read path is unchanged. Add **one parallel CPU stage** after files are read
and before sorting, only when outline is active:

```rust
// lib.rs::serialize_repo, right after process_files_parallel(...) returns `files`
#[cfg(feature = "outline")]
if config.outline_active() {
    files.par_iter_mut().for_each(|f| {
        if let Some(lang) = outline::detect_language(&f.rel_path) {
            f.language = Some(lang);
            f.symbols  = outline::extract(&f.content, lang); // Option<Vec<Symbol>>
        }
    });
}
```

Why a separate stage: extraction is CPU-bound and embarrassingly parallel, so it
wants the full rayon pool. Today's pipeline reads files on a **single** processing
thread (the channel consumer in `parallel.rs`); parsing there would serialize it.
`par_iter_mut` over the already-collected, already-in-memory files avoids that
bottleneck with no extra IO. Parsers are `Send` but not `Sync`, so use a
thread-local pool:

```rust
thread_local! { static PARSERS: RefCell<HashMap<Language, Parser>> = /* … */; }
```

### 15.7 Budget-aware allocator (replaces the `concat_files` cut)

Refines §9 into a **single forward pass with bounded token counting**. Files are
processed priority-DESC; per-file candidate levels `Full ▸ Outline ▸ Api ▸
Symbols` (cheapest last); `Omit` is the implicit floor.

```text
cost(f, L) = token_mode ? count_tokens(format(f, L)) : len(format(f, L))   # memoized per (f,L)
floor(f)   = cost(f, Symbols)            # cheapest non-omit form of f

# Precompute in parallel (par_iter):
#   floor(f) for all f
#   suffix_floor[i] = Σ floor(f) for files i..n      # what the tail still needs
# If suffix_floor[0] > cap  →  GLOBAL FALLBACK (below).

used = 0
for i, f in files:                        # priority DESC, sequential
    budget_here = cap - used - suffix_floor[i+1]    # room that still lets the tail reach its floor
    pick = Symbols
    for L in [Api, Outline, Full]:        # try richer; first that doesn't fit stops the climb
        if cost(f, L) <= budget_here { pick = L } else { break }
    used += cost(f, pick); assign(f, pick)
```

Perf properties / gaps closed:

- **Hard cap respected** (token or byte) — every choice is checked against exact
  `cost`.
- **Priority-greedy, not knapsack** — *by design*: a high-priority file is never
  downgraded to fund a lower-priority one. Matches yek's "importance is king"
  model and is O(n·levels), not NP-hard search.
- **Bounded tiktoken work.** Exact counts: `floor` for all files (parallel) +
  `Full` only for files whose `budget_here` could admit it (the long
  low-priority tail never pays for a `Full` count) + ≤2 intermediate counts for
  frontier files. Low-priority files cost exactly one count. This is comparable
  to today's token-mode cost — `Full` counts reuse the same per-file-formatted
  counting `concat_files` already does — not a 4× blowup.
- **Byte mode is nearly free** (`len`, no tiktoken).
- **Template overhead is counted.** `format(f, L)` includes the `>>>> PATH`
  header, so the per-file header cost is in the budget (it can dominate for many
  tiny files — see fallback).
- **GLOBAL FALLBACK** for the maintainer's "even the skeleton won't fit" case:
  when all-`Symbols` exceeds `cap`, collapse the lowest-priority tail into a
  **single repo-level compact index** block (one header; `path: symA, symB, …`
  per line), amortizing the per-file header that otherwise dominates at thousands
  of files; keep upgrading the high-priority head. A bare **file-tree** (paths
  only) is the terminal rung before `Omit`.
- **Debug metrics** (cheap): under `--debug`, log
  `full M · outline N · api A · symbols K · index/omit J — saved ≈X tokens (Y%)`.

This also corrects the §3 ordering issue: processing priority-DESC means tight
budgets keep the *most* important files, not drop them.

### 15.8 CLI & config plumbing

`config.rs` gains fields in the existing `#[config_arg]` style:

```rust
#[config_arg(default_value = "off")]      pub outline_mode: OutlineMode,       // off|always|degrade
#[config_arg(default_value = "outline")]  pub outline_level: OutlineLevel,     // outline|api|symbols
#[config_arg(accept_from = "config_only")] pub outline_languages: Vec<String>,
#[config_arg(default_value = "full")]     pub outline_fallback: OutlineFallback, // full|omit
```

`--outline` is sugar for `--outline-mode always`. Add a computed
`outline_active()`. `validate()` rejects unknown language names and, when built
without the `outline` feature, downgrades non-`off` modes to `off` with a stderr
warning. All defaults reproduce current behavior.

### 15.9 Output format & round-tripping

Refines §10, closing a correctness gap: **never mutate `FILE_PATH`** (that breaks
`unyek`/#64 and path-keyed tools). Instead expose a `LEVEL` placeholder
(`""` for full, ` (outline)`/` (api)`/` (symbols)`/` (index)` otherwise),
opt-in via custom templates; add an in-block `⟪outline: bodies elided⟫` hint for
the LLM; add `"level"` to `--json`. Document loudly that non-`full` output is
**lossy and irreversible** — `unyek` must refuse to reconstruct it.

### 15.10 Testing & CI

- **Golden/snapshot** (add `insta` dev-dep) per language: fixture source → L1/L2/L3.
- **Fallback**: unsupported extension stays L0 under `always`; `--outline-fallback
  omit` drops it; high-`ERROR`-ratio source ⇒ L0.
- **Allocator unit tests** with synthetic known costs: (a) never exceeds cap;
  (b) monotonic — no lower-priority file ends richer than a higher-priority one;
  (c) `off` reproduces today's bytes exactly; (d) global-fallback triggers and
  stays under cap.
- **Determinism**: stable order + stable markers (snapshot).
- **Property**: `cost(Symbols) ≤ cost(Api) ≤ cost(Outline) ≤ cost(Full)` per fixture.
- **CI matrix**: build & test `--no-default-features` (today's path) **and**
  `--features outline`; smoke-parse each language (ABI guard); verify musl +
  windows-msvc grammar builds; add a binary-size report step.
- **Bench**: extend `benches/serialization.rs` with a mixed-language tree
  measuring (1) extraction wall-clock vs. read time and (2) tokens emitted per
  mode, to quantify the budget win.

### 15.11 Milestones (file-level)

- **M1 — Scaffold (Rust only).** `Cargo.toml` feature+deps; `src/outline/*` with
  the Rust query; `config.rs` flags+validation; `lib.rs` parallel extract stage +
  allocator restricted to `Full/Omit` for `off`-equivalence; `LEVEL` token;
  fixtures + snapshots; two-config CI build. *Acceptance:* `--outline` on a Rust
  dir emits signatures; `off` output is byte-for-byte unchanged;
  `--no-default-features` builds.
- **M2 — Languages.** TS/JS(+TSX/JSX), Python, Go, Java queries + visibility
  rules + fixtures; `--outline-languages`, `--outline-fallback`.
- **M3 — Degradation.** Suffix-floor allocator, `Api`/`Symbols`, global
  compact-index fallback, `--outline-mode degrade`, debug metrics; allocator +
  property tests; bench proving reduction. *Acceptance:* tight `--tokens` keeps
  every file represented under the cap with high-priority files full.
- **M4 — Custom rules.** User-overridable queries/visibility in `yek.yaml`;
  document adding a language with no code change.
- **M5 — Lazy retrieval / MCP.** Outline-first serialization + on-demand
  symbol/file expansion as MCP tools (issues #245, #232).

### 15.12 Gaps found while detailing, and resolutions

| Gap in the v1 sketch | Resolution |
| --- | --- |
| Counting tokens for 4 levels × all files is costly | Parse once → `Vec<Symbol>` (ranges); render lazily; precompute only `floor` (parallel) + `Full` for the head; memoize per (file,level); suffix-floor skips `Full` counts for the tail (§15.7). |
| Caching 4 rendered strings per file wastes memory | Store ranges only; render the chosen level once at the end (§15.3). |
| Per-file template header dominates at thousands of tiny files (skeleton still won't fit) | Global compact-index fallback (one header, many files) + bare file-tree rung (§15.7). |
| Suffixing `FILE_PATH` with `(outline)` breaks round-trip/#64 and path-keyed tools | Separate `LEVEL` placeholder; path stays exact; in-block hint; JSON `level` (§15.9). |
| Single processing thread would serialize CPU-bound parsing | Dedicated `par_iter_mut` extract stage on the rayon pool (§15.6). |
| Files with no / huge symbol sets | Empty ⇒ fall back to L0; huge ⇒ `MAX_SYMBOLS` cap with elision (§15.4). |
| Parse errors / partial files | Tolerate few `ERROR` nodes; high error ratio ⇒ L0 (§15.4). |
| Grammar/core ABI drift across many release targets | Pin a tested set (prefer `tree-sitter-language` shim) + CI smoke-parse + musl/msvc build checks (§15.1). |
| Building slim/default without grammars | Cargo feature gate; non-`off` modes warn and act as `off` (§15.1/§15.8). |
| `from_utf8_lossy` could desync byte ranges | Parse and slice the *same* lossy string ⇒ self-consistent ranges (§15.4). |
| Nested members (methods in classes) flattened | Stack-based nesting reconstruction → `depth`/`children`, indented render (§15.4/§15.5). |

### 15.13 Performance targets (acceptance guards)

- **Outline OFF:** zero overhead — identical code path and binary as today
  (feature-gated out); enforced by the byte-identical `off` snapshot test.
- **Outline ON:** added latency dominated by parse (target ≲ 1× the warm
  file-read time, run in parallel) plus bounded tiktoken counts (≈ today's
  token-mode counts + the frontier). Peak memory grows only by `Vec<Symbol>`
  per file.
- **Token win:** target the ast-grep-reported **35–55%** reduction on large repos
  at equal symbol coverage, tracked by the new benchmark.

## 16. Appendix — illustrative before/after

A 1,200-line, 5-file TypeScript service, budget `--tokens 8k`:

- **Today:** ~3 files fit verbatim, 2 dropped. The model never learns the
  dropped files exist.
- **`--outline-mode degrade`:** the 2 highest-priority files stay full; the other
  3 collapse to outlines (exports + signatures). All 5 files are represented, the
  budget is respected, and the model has a complete structural map plus full
  detail where it matters most.
