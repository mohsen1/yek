use super::{detect_language, extract, render, Language, OutlineLevel};

const SAMPLE: &str = r#"use std::collections::HashMap;

/// A point in 2D space.
pub struct Point {
    pub x: i32,
    pub y: i32,
}

enum Shape {
    Circle(f64),
    Square(f64),
}

pub fn area(s: &Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Square(a) => a * a,
    }
}

fn helper() -> i32 {
    let mut total = 0;
    for i in 0..10 {
        total += i;
    }
    total
}

pub trait Drawable {
    fn draw(&self);
}

impl Point {
    pub fn origin() -> Point {
        Point { x: 0, y: 0 }
    }
    fn private_helper(&self) -> i32 {
        self.x + self.y
    }
}

impl Drawable for Point {
    fn draw(&self) {
        println!("draw");
    }
}
"#;

fn outline(level: OutlineLevel) -> String {
    let symbols = extract(SAMPLE, Language::Rust).expect("sample has symbols");
    render(SAMPLE, Language::Rust, &symbols, level)
}

#[test]
fn detects_language_by_extension() {
    assert_eq!(detect_language("src/main.rs"), Some(Language::Rust));
    assert_eq!(detect_language("a/b/lib.rs"), Some(Language::Rust));
    assert_eq!(detect_language("README.md"), None);
    assert_eq!(detect_language("Makefile"), None);
    assert_eq!(detect_language("script.py"), None);
}

#[test]
fn full_level_is_verbatim() {
    let symbols = extract(SAMPLE, Language::Rust).unwrap();
    assert_eq!(render(SAMPLE, Language::Rust, &symbols, OutlineLevel::Full), SAMPLE);
}

#[test]
fn outline_keeps_signatures_and_elides_bodies() {
    let out = outline(OutlineLevel::Outline);

    // Signatures are present.
    assert!(out.contains("pub fn area(s: &Shape) -> f64"), "got:\n{out}");
    assert!(out.contains("fn helper() -> i32"), "private fn shown at Outline");

    // Bodies are gone.
    assert!(!out.contains("3.14159"), "area body should be elided:\n{out}");
    assert!(!out.contains("total += i"), "helper body should be elided:\n{out}");
    assert!(out.contains("/* …"), "elision marker missing:\n{out}");

    // Declarations (struct fields, enum variants) are shown verbatim.
    assert!(out.contains("pub struct Point"));
    assert!(out.contains("pub x: i32"));
    assert!(out.contains("Circle(f64)"));

    // Containers recurse: header, nested signature, closing brace.
    assert!(out.contains("impl Drawable for Point {"), "got:\n{out}");
    assert!(out.contains("fn draw(&self)"));

    // A bodyless trait method is kept verbatim (function_signature_item).
    assert!(out.contains("fn draw(&self);"), "trait method dropped:\n{out}");
}

#[test]
fn api_level_drops_private_symbols() {
    let out = outline(OutlineLevel::Api);

    assert!(out.contains("pub fn area"), "got:\n{out}");
    assert!(out.contains("pub fn origin"), "public method kept:\n{out}");
    assert!(out.contains("fn draw"), "trait impl method is public API:\n{out}");

    assert!(!out.contains("fn helper"), "private free fn dropped:\n{out}");
    assert!(!out.contains("private_helper"), "private method dropped:\n{out}");
}

#[test]
fn symbols_level_lists_declarations() {
    let out = outline(OutlineLevel::Symbols);
    assert!(out.contains("struct Point"));
    assert!(out.contains("fn area"));
    assert!(out.contains("trait Drawable"));
    // Line ranges are emitted.
    assert!(out.contains("(L"));
    // Nested methods are indented under their container.
    assert!(out.contains("  fn draw"), "got:\n{out}");
    // Nameless declarations are not double-prefixed with their keyword.
    assert!(!out.contains("use use"), "doubled keyword:\n{out}");
    assert!(!out.contains("impl impl"), "doubled keyword:\n{out}");
    assert!(out.contains("impl Point"));
}

#[test]
fn returns_none_when_nothing_to_outline() {
    // No top-level declarations to summarize.
    assert!(extract("let x = 1 + 2;\n", Language::Rust).is_none());
    assert!(extract("", Language::Rust).is_none());
}

#[test]
fn language_name_round_trips() {
    for &lang in Language::all() {
        assert_eq!(Language::from_name(lang.name()), Some(lang));
    }
}

#[test]
fn single_line_container_does_not_duplicate_header_or_leak_bodies() {
    // Regression: rendering a container whose members share its line must not
    // re-emit the header or leak un-elided bodies.
    let src = "impl S { pub fn a(&self) -> i32 { 1 } pub fn b(&self) -> i32 { 2 } }\n";
    let symbols = extract(src, Language::Rust).unwrap();
    let out = render(src, Language::Rust, &symbols, OutlineLevel::Outline);

    assert_eq!(out.matches("impl S").count(), 1, "header duplicated:\n{out}");
    assert!(!out.contains("{ 1 }"), "body leaked un-elided:\n{out}");
    assert!(!out.contains("{ 2 }"), "body leaked un-elided:\n{out}");
    assert!(out.contains("pub fn a(&self) -> i32"));
    assert!(out.contains("pub fn b(&self) -> i32"));
}

#[test]
fn outline_keeps_attributes_and_doc_comments() {
    let src = "/// The config.\n#[derive(Debug, Clone)]\npub struct Config {\n    pub name: String,\n}\n";
    let symbols = extract(src, Language::Rust).unwrap();
    let out = render(src, Language::Rust, &symbols, OutlineLevel::Outline);

    assert!(out.contains("/// The config."), "doc dropped:\n{out}");
    assert!(out.contains("#[derive(Debug, Clone)]"), "attribute dropped:\n{out}");
    assert!(out.contains("pub struct Config"));
}

#[test]
fn elide_marker_counts_hidden_content_lines() {
    // Body spans rows 0..4 ({ on row 0, } on row 4) with 3 content lines.
    let src = "pub fn f() {\n    a();\n    b();\n    c();\n}\n";
    let symbols = extract(src, Language::Rust).unwrap();
    let out = render(src, Language::Rust, &symbols, OutlineLevel::Outline);
    assert!(out.contains("3 lines"), "wrong hidden line count:\n{out}");
}

#[test]
fn handles_multibyte_source_without_panicking() {
    let src = "/// café ☕ 漢字\npub fn naïve() -> &'static str {\n    \"日本語 😀\"\n}\n";
    let symbols = extract(src, Language::Rust).unwrap();
    for level in [OutlineLevel::Outline, OutlineLevel::Api, OutlineLevel::Symbols] {
        let _ = render(src, Language::Rust, &symbols, level);
    }
}
