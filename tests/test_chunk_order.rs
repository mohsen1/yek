mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use std::fs;
use tracing::Level;
use tracing_subscriber::fmt;

/// This test ensures that the last-written chunk contains the highest-priority file.
#[test]
fn chunk_order_reflects_priority() {
    // Setup logging - fail fast if logging setup fails
    fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .try_init()
        .expect("Failed to initialize logging");

    let repo = setup_temp_repo();
    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Create a Yek config that makes `high_priority/` have a very high score
    create_file(
        repo.path(),
        "yek.toml",
        r#"
[[priority_rules]]
score = 10
patterns = ["^low_priority/"]

[[priority_rules]]
score = 999
patterns = ["^high_priority/"]
"#,
    );

    // Create a small file in low_priority
    create_file(repo.path(), "low_priority/foo.txt", "low priority content");

    // Create a bigger file in high_priority that will definitely be split
    // Using chunk size of 1KB, we need more than that to force splitting
    let chunk_size_bytes = 1024;
    let min_content_size = chunk_size_bytes * 2; // At least 2 chunks
    let line = "HIGH PRIORITY\n";
    let repeat_count = (min_content_size + line.len() - 1) / line.len();
    let big_content = line.repeat(repeat_count);
    create_file(repo.path(), "high_priority/foo.txt", &big_content);

    // We'll force extremely small max-size to ensure multiple chunks.
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .arg("--max-size")
        .arg("1KB") // force chunking
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--debug")
        .env("TERM", "xterm-256color")
        .assert()
        .success();

    // Read chunk-0.txt to verify it contains the low priority file
    let chunk0_path = output_dir.join("chunk-0.txt");
    assert!(chunk0_path.exists(), "chunk-0.txt should exist");
    let chunk0_content = fs::read_to_string(&chunk0_path).expect("Failed to read chunk-0.txt");
    assert!(
        chunk0_content.contains("low_priority/foo.txt"),
        "Low priority file should be in chunk 0"
    );

    // Verify that high priority file appears in later chunks
    let mut found_high_priority = false;
    for entry in fs::read_dir(&output_dir).expect("Failed to read output directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.file_name().unwrap().to_string_lossy() == "chunk-0.txt" {
            continue;
        }
        let content =
            fs::read_to_string(&path).expect(&format!("Failed to read {}", path.display()));
        if content.contains("high_priority/foo.txt") {
            found_high_priority = true;
            break;
        }
    }
    assert!(
        found_high_priority,
        "High priority file should be in a later chunk"
    );
}
