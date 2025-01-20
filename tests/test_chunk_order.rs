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

    // Create test files with different priorities
    let high_priority_path = temp.path().join("README.md");
    fs::write(&high_priority_path, "high priority content").unwrap();

    fs::create_dir_all(temp.path().join("src")).unwrap();
    let medium_priority_path = temp.path().join("src").join("lib.rs");
    fs::write(&medium_priority_path, "medium priority content").unwrap();

    let low_priority_path = temp.path().join("test.txt");
    fs::write(&low_priority_path, "low priority content").unwrap();

    // Create config with priority rules
    let mut config = YekConfig::default();
    config.output_dir = Some(output_dir.clone());
    config.max_size = Some(1024); // 1KB should be enough for our small test files
    config.priority_rules = vec![
        PriorityRule {
            pattern: "README.md".to_string(),
            score: 100,
        },
        PriorityRule {
            pattern: "src/lib.rs".to_string(),
            score: 50,
        },
        PriorityRule {
            pattern: "test.txt".to_string(),
            score: 10,
        },
    ];
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

    // Check that files appear in ascending priority order
    let test_pos = content.find("test.txt").expect("test.txt not found");
    let lib_pos = content.find("src/lib.rs").expect("src/lib.rs not found");
    let readme_pos = content.find("README.md").expect("README.md not found");

    // Verify ascending priority order
    assert!(
        test_pos < lib_pos && lib_pos < readme_pos,
        "Files should appear in ascending priority order"
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
