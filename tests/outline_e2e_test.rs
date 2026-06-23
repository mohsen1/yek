#![cfg(feature = "outline")]
//! End-to-end tests for outline mode wired through `serialize_repo`.

use std::fs;
use tempfile::TempDir;
use yek::config::{OutlineFallback, OutlineMode, YekConfig};
use yek::priority::PriorityRule;
use yek::serialize_repo;

fn config_for(dir: &TempDir) -> YekConfig {
    YekConfig::extend_config_with_defaults(
        vec![dir.path().to_string_lossy().to_string()],
        dir.path().join("out").to_string_lossy().to_string(),
    )
}

const RS_FILE: &str = "pub fn foo() {\n    let x = 1;\n    let y = 2;\n    let z = 3;\n}\n";

#[test]
fn always_outlines_rust_files() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), RS_FILE).unwrap();
    let mut cfg = config_for(&dir);
    cfg.outline_mode = Some(OutlineMode::Always);

    let (out, files) = serialize_repo(&cfg).unwrap();

    assert!(out.contains("pub fn foo()"), "signature missing:\n{out}");
    assert!(out.contains("/* …"), "body not elided:\n{out}");
    assert!(!out.contains("let x = 1"), "body leaked:\n{out}");
    assert!(out.contains("⟪yek:outline⟫"), "marker missing:\n{out}");

    let a = files.iter().find(|f| f.rel_path == "a.rs").unwrap();
    assert_eq!(a.outline_level, Some("outline"));
}

#[test]
fn fallback_full_keeps_unsupported_verbatim() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), RS_FILE).unwrap();
    fs::write(dir.path().join("notes.md"), "# Title\nbody text\n").unwrap();
    let mut cfg = config_for(&dir);
    cfg.outline_mode = Some(OutlineMode::Always); // fallback defaults to full

    let (out, files) = serialize_repo(&cfg).unwrap();

    assert!(
        out.contains("body text"),
        "unsupported file should be verbatim"
    );
    let md = files.iter().find(|f| f.rel_path == "notes.md").unwrap();
    assert_eq!(md.outline_level, None);
}

#[test]
fn fallback_omit_drops_unsupported() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), RS_FILE).unwrap();
    fs::write(dir.path().join("notes.md"), "# Title\nbody text\n").unwrap();
    let mut cfg = config_for(&dir);
    cfg.outline_mode = Some(OutlineMode::Always);
    cfg.outline_fallback = Some(OutlineFallback::Omit);

    let (out, files) = serialize_repo(&cfg).unwrap();

    assert!(files.iter().any(|f| f.rel_path == "a.rs"));
    assert!(
        !files.iter().any(|f| f.rel_path == "notes.md"),
        "md not omitted"
    );
    assert!(!out.contains("body text"));
}

#[test]
fn json_output_includes_level_field() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), RS_FILE).unwrap();
    let mut cfg = config_for(&dir);
    cfg.outline_mode = Some(OutlineMode::Always);
    cfg.json = true;

    let (out, _) = serialize_repo(&cfg).unwrap();
    assert!(
        out.contains("\"level\""),
        "json level field missing:\n{out}"
    );
    assert!(out.contains("\"outline\""));
}

#[test]
fn degrade_keeps_high_priority_full_and_outlines_rest() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("important.rs"),
        "pub fn important_thing() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("minor.rs"),
        "pub fn minor_thing() {\n    let x = 10;\n    let y = 20;\n    let z = 30;\n}\n",
    )
    .unwrap();

    let mut cfg = config_for(&dir);
    cfg.outline_mode = Some(OutlineMode::Degrade);
    cfg.token_mode = true;
    cfg.tokens = "80".to_string();
    cfg.priority_rules = vec![
        PriorityRule {
            pattern: "important.rs".to_string(),
            score: 100,
        },
        PriorityRule {
            pattern: "minor.rs".to_string(),
            score: 1,
        },
    ];

    let (_out, files) = serialize_repo(&cfg).unwrap();

    let important = files.iter().find(|f| f.rel_path == "important.rs").unwrap();
    let minor = files.iter().find(|f| f.rel_path == "minor.rs").unwrap();
    assert_eq!(
        important.outline_level, None,
        "high-priority file should stay full"
    );
    assert_eq!(
        minor.outline_level,
        Some("outline"),
        "low-priority file should be outlined"
    );
}
