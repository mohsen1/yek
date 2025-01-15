mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use std::fs;
use tracing::Level;
use tracing_subscriber::fmt;

#[test]
fn basic_file_output_test() {
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
    create_file(repo.path(), "test.txt", "test content");

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--debug")
        .env("TERM", "xterm-256color")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    println!("Stdout output: {}", stdout);
    println!("Stderr output: {}", stderr);
    println!("Output directory exists: {}", output_dir.exists());
    if output_dir.exists() {
        println!("Output directory contents:");
        for entry in fs::read_dir(&output_dir).unwrap() {
            let entry = entry.unwrap();
            println!("  {}", entry.path().display());
            if entry.path().is_file() {
                println!("File contents:");
                println!("{}", fs::read_to_string(entry.path()).unwrap());
            }
        }
    }
    assert!(
        stdout.contains("Written chunk 0 with"),
        "Should write first chunk"
    );

    // Check output directory
    assert!(output_dir.exists(), "Output directory should exist");

    // Check chunk file
    let chunk_file = output_dir.join("chunk-0.txt");
    assert!(chunk_file.exists(), "Chunk file should exist");

    // Verify content
    let content = fs::read_to_string(chunk_file).unwrap();
    assert!(content.contains("test.txt"), "Should contain file name");
    assert!(
        content.contains("test content"),
        "Should contain file content"
    );
}

#[test]
fn basic_pipe_test() {
    let repo = setup_temp_repo();
    create_file(repo.path(), "test.txt", "test content");

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .env("TERM", "dumb") // Force non-interactive mode
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("test.txt"), "Should contain file name");
    assert!(
        stdout.contains("test content"),
        "Should contain file content"
    );
}
