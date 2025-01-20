#[path = "integration_common.rs"]
mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, ensure_empty_output_dir, setup_temp_repo};
use std::fs;

/// Writes a file larger than the default 10MB limit in tokens or bytes, forcing multiple chunks.
#[test]
fn splits_large_file_in_chunks_bytes_mode() {
    let repo = setup_temp_repo();
    let large_content = "A ".repeat(1024 * 1024 * 11); // ~ 11MB
    create_file(repo.path(), "BIG.txt", large_content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let debug_output = repo.path().join("debug.log");
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--max-size")
        .arg("10MB")
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("YEK_DEBUG_OUTPUT", &debug_output)
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stderr: {}", stderr);

    // Read debug output
    let debug_log = fs::read_to_string(&debug_output).expect("Failed to read debug log");
    println!("debug log: {}", debug_log);

    // Check debug messages
    assert!(
        debug_log.contains("File exceeds chunk size, splitting into multiple chunks"),
        "Should indicate file exceeds chunk size"
    );
    assert!(
        debug_log.contains("Writing large file part 0"),
        "Should write first part"
    );
    assert!(
        debug_log.contains("Writing large file part 1"),
        "Should write second part"
    );
}

#[test]
fn splits_large_file_in_chunks_token_mode() {
    let repo = setup_temp_repo();
    // Each "word" is a token
    let large_content = "TOKEN ".repeat(200_000); // enough tokens to exceed default
    create_file(repo.path(), "BIG_token.txt", large_content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let debug_output = repo.path().join("debug.log");
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens")
        .arg("--max-size")
        .arg("150000") // ~150k tokens
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("YEK_DEBUG_OUTPUT", &debug_output)
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stderr: {}", stderr);

    // Read debug output
    let debug_log = fs::read_to_string(&debug_output).expect("Failed to read debug log");
    println!("debug log: {}", debug_log);

    // Check debug messages
    assert!(
        debug_log.contains("File exceeds chunk size, splitting into multiple chunks"),
        "Should indicate file exceeds chunk size"
    );
    assert!(
        debug_log.contains("Writing large file part 0"),
        "Should write first part"
    );
    assert!(
        debug_log.contains("Writing large file part 1"),
        "Should write second part"
    );
}
