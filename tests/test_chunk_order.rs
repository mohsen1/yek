mod integration_common;
use std::fs;
use tempfile::TempDir;
use yek::{serialize_repo, PriorityRule, YekConfig};

/// Tests that files are written in ascending priority order within a chunk.
/// Lower priority files should appear first, and higher priority files should appear last.
#[test]
fn chunk_order_reflects_priority() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    // Create test files
    fs::write(temp.path().join("test.txt"), "low priority content").unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(temp.path().join("src/lib.rs"), "medium priority content").unwrap();
    fs::write(temp.path().join("README.md"), "high priority content").unwrap();

    // Configure priority rules
    let config = YekConfig {
        priority_rules: vec![
            PriorityRule {
                pattern: "^README.md$".to_string(),
                score: 100,
            },
            PriorityRule {
                pattern: "^src/".to_string(),
                score: 50,
            },
        ],
        output_dir: Some(output_dir.clone()),
        ..Default::default()
    };
    serialize_repo(temp.path(), Some(&config)).unwrap();

    // Debug output for file contents
    for entry in fs::read_dir(&output_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        println!("  {}", path.display());
        if path.is_file() {
            println!("Contents of {}:", path.display());
            println!("{}", fs::read_to_string(&path).unwrap());
        }
    }

    // All files should be in chunk-0.txt since it's the first chunk
    let output_path = output_dir.join("chunk-0.txt");
    let content = fs::read_to_string(&output_path).unwrap();

    // Check that files appear in ascending priority order (lower priority first)
    let test_pos = content.find("test.txt").expect("test.txt not found");
    let lib_pos = content.find("src/lib.rs").expect("src/lib.rs not found");
    let readme_pos = content.find("README.md").expect("README.md not found");

    // Verify ascending priority order (lower priority first)
    assert!(
        test_pos < lib_pos && lib_pos < readme_pos,
        "Files should appear in ascending priority order (lower priority first)"
    );

    // Verify file contents
    assert!(
        content.contains("low priority content"),
        "Should contain low priority content"
    );
    assert!(
        content.contains("medium priority content"),
        "Should contain medium priority content"
    );
    assert!(
        content.contains("high priority content"),
        "Should contain high priority content"
    );
}
