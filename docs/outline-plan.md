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

## 10. Output format

So the model (and humans) can tell a file was abbreviated, annotate the template
context. Add an optional `LEVEL` placeholder and a default suffix marker:

```text
>>>> src/parallel.rs (outline)
…signatures…
```

- Default template stays `>>>> FILE_PATH\nFILE_CONTENT`; when a file is rendered
  below L0, `FILE_PATH` is suffixed with ` (outline)` / ` (api)` / ` (symbols)`,
  or a new `LEVEL` token is exposed for custom templates.
- `--json` output gains a `"level": "outline"` field per object so programmatic
  consumers (and an MCP server) can distinguish abbreviated entries.

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

## 15. Appendix — illustrative before/after

A 1,200-line, 5-file TypeScript service, budget `--tokens 8k`:

- **Today:** ~3 files fit verbatim, 2 dropped. The model never learns the
  dropped files exist.
- **`--outline-mode degrade`:** the 2 highest-priority files stay full; the other
  3 collapse to outlines (exports + signatures). All 5 files are represented, the
  budget is respected, and the model has a complete structural map plus full
  detail where it matters most.
