mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use std::fs;
use tracing::Level;
use tracing_subscriber::fmt;

#[test]
fn skips_known_binary_files() {
    // Setup logging
    fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .try_init()
        .ok();

    let repo = setup_temp_repo();
    create_file(repo.path(), "test.jpg", "binary content");
    create_file(repo.path(), "test.txt", "text content");

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("TERM", "xterm-256color")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("Written chunk 0 with"),
        "Should write first chunk"
    );

    // Check output directory
    let output_dir = repo.path().join("yek-output");
    assert!(output_dir.exists(), "Output directory should exist");

    // Check chunk file
    let chunk_file = output_dir.join("chunk-0.txt");
    assert!(chunk_file.exists(), "Chunk file should exist");

    // Verify content
    let content = fs::read_to_string(chunk_file).unwrap();
    assert!(
        !content.contains("test.jpg"),
        "Should not contain binary file"
    );
    assert!(content.contains("test.txt"), "Should contain text file");
}

#[test]
fn respects_custom_binary_extensions() {
    // Setup logging
    fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .try_init()
        .ok();

    let repo = setup_temp_repo();
    create_file(repo.path(), "test.custom", "binary content");
    create_file(repo.path(), "test.txt", "text content");

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    // Create config file with custom binary extension
    create_file(
        repo.path(),
        "yek.toml",
        r#"
        binary_extensions = [".custom"]
        "#,
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("TERM", "xterm-256color")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("Written chunk 0 with"),
        "Should write first chunk"
    );

    // Check output directory
    let output_dir = repo.path().join("yek-output");
    assert!(output_dir.exists(), "Output directory should exist");

    // Check chunk file
    let chunk_file = output_dir.join("chunk-0.txt");
    assert!(chunk_file.exists(), "Chunk file should exist");

    // Verify content
    let content = fs::read_to_string(chunk_file).unwrap();
    assert!(
        !content.contains("test.custom"),
        "Should not contain custom binary file"
    );
    assert!(content.contains("test.txt"), "Should contain text file");
}
