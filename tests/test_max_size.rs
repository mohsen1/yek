mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use std::fs;

/// Writes a file larger than the default 10MB limit in tokens or bytes, forcing multiple chunks.
#[test]
fn splits_large_file_in_chunks_bytes_mode() {
    let repo = setup_temp_repo();
    let large_content = "A ".repeat(1024 * 1024 * 11); // ~ 11MB
    create_file(repo.path(), "BIG.txt", &large_content);

    // Create temp file for debug output
    let debug_output = repo.path().join("debug_output.txt");
    let debug_output_path = debug_output.to_str().unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--max-size")
        .arg("10MB")
        .arg("--debug")
        .env("YEK_DEBUG_OUTPUT", debug_output_path)
        .assert()
        .success();

    // Read debug output from file
    let debug_content = fs::read_to_string(debug_output).unwrap();

    // Check debug messages
    assert!(
        debug_content.contains("File exceeds chunk size, splitting into multiple chunks"),
        "Should indicate file exceeds chunk size"
    );
    assert!(
        debug_content.contains("Written chunk 0"),
        "Should write first chunk"
    );
    assert!(
        debug_content.contains("Written chunk 1"),
        "Should write second chunk"
    );
}

#[test]
fn splits_large_file_in_chunks_token_mode() {
    let repo = setup_temp_repo();
    // Each "word" is a token
    let large_content = "TOKEN ".repeat(200_000); // enough tokens to exceed default
    create_file(repo.path(), "BIG_token.txt", &large_content);

    // Create temp file for debug output
    let debug_output = repo.path().join("debug_output.txt");
    let debug_output_path = debug_output.to_str().unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--tokens")
        .arg("--max-size")
        .arg("150000") // ~150k tokens
        .arg("--debug")
        .env("YEK_DEBUG_OUTPUT", debug_output_path)
        .assert()
        .success();

    // Read debug output from file
    let debug_content = fs::read_to_string(debug_output).unwrap();

    // Check debug messages
    assert!(
        debug_content.contains("File exceeds chunk size, splitting into multiple chunks"),
        "Should indicate file exceeds chunk size"
    );
    assert!(
        debug_content.contains("Written chunk 0"),
        "Should write first chunk"
    );
    assert!(
        debug_content.contains("Written chunk 1"),
        "Should write second chunk"
    );
}
