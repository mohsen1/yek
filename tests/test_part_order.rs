mod integration_common;
use std::fs;
use tempfile::TempDir;
use yek::serialize_repo;
use yek::PriorityRule;
use yek::YekConfig;

/// Tests that files are written in descending priority order within a part.
#[test]
fn part_order_reflects_priority() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    // Create test files with different priorities
    let files = vec![
        ("a.txt", "content a", 1),
        ("b.txt", "content b", 2),
        ("c.txt", "content c", 3),
    ];

    for (name, content, _) in &files {
        let path = temp.path().join(name);
        fs::write(&path, content).unwrap();
    }

    // Run serialization with priority rules
    let config = YekConfig {
        output_dir: Some(output_dir.clone()),
        priority_rules: vec![
            PriorityRule {
                pattern: "a.txt".to_string(),
                score: 1,
            },
            PriorityRule {
                pattern: "b.txt".to_string(),
                score: 2,
            },
            PriorityRule {
                pattern: "c.txt".to_string(),
                score: 3,
            },
        ],
        ..Default::default()
    };
    serialize_repo(temp.path(), Some(&config)).unwrap();

    // All files should be in output.txt
    let output_path = output_dir.join("output.txt");
    let content = fs::read_to_string(output_path).unwrap();

    // Check that files appear in ascending priority order (higher priority files last)
    let a_pos = content.find("a.txt").unwrap();
    let b_pos = content.find("b.txt").unwrap();
    let c_pos = content.find("c.txt").unwrap();

    assert!(
        a_pos < b_pos && b_pos < c_pos,
        "Files should be ordered by ascending priority with higher priority files last"
    );
}
