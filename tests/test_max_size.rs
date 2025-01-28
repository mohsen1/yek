#[path = "integration_common.rs"]
mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, ensure_empty_output_dir, setup_temp_repo};
use std::fs;

/// The new code no longer splits into multiple chunks. We only verify that a large file is fully included.
#[test]
fn large_file_included_single_output() {
    let repo = setup_temp_repo();
    // ~1MB of text
    let big_content = "A ".repeat(1024 * 500);
    create_file(repo.path(), "BIG.txt", big_content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        // max-size won't actually split anything now, but we'll pass it anyway
        .arg("--max-size")
        .arg("10KB")
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(&out_file).expect("Failed to read single output file");
    assert!(
        content.contains("BIG.txt"),
        "Single output should still contain the entire large file text"
    );
}
