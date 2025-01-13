mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};

/// Writes a file larger than the default 10MB limit in tokens or bytes, forcing multiple chunks.
#[test]
fn splits_large_file_in_chunks_bytes_mode() {
    let repo = setup_temp_repo();
    let large_content = "A ".repeat(1024 * 1024 * 11); // ~ 11MB
    create_file(repo.path(), "BIG.txt", &large_content);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        // Setting max-size to 10MB in bytes mode
        .arg("--max-size")
        .arg("10")
        .arg("--debug") // Enable debug output
        .assert()
        .success();

    // Print full output for debugging
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    println!("\nSTDOUT:\n{}", stdout);
    println!("\nSTDERR:\n{}", stderr);

    // Check debug message in stdout
    assert!(
        stdout.contains("File exceeds chunk size, splitting into multiple chunks"),
        "Should indicate file exceeds chunk size"
    );

    // Check chunk messages in stderr
    assert!(
        stderr.contains("Written chunk 0"),
        "Should write first chunk"
    );
    assert!(
        stderr.contains("Written chunk 1"),
        "Should write second chunk"
    );
}

#[test]
fn splits_large_file_in_chunks_token_mode() {
    let repo = setup_temp_repo();
    // Each "word" is a token
    let large_content = "TOKEN ".repeat(200_000); // enough tokens to exceed default
    create_file(repo.path(), "BIG_token.txt", &large_content);

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        // Switch to token mode
        .arg("--tokens")
        .arg("--max-size")
        .arg("150000") // ~150k tokens
        .arg("--debug") // Enable debug output
        .assert()
        .success();

    // Print full output for debugging
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    println!("\nSTDOUT:\n{}", stdout);
    println!("\nSTDERR:\n{}", stderr);

    // Check debug message in stdout
    assert!(
        stdout.contains("File exceeds chunk size, splitting into multiple chunks"),
        "Should indicate file exceeds chunk size"
    );

    // Check chunk messages in stderr
    assert!(
        stderr.contains("Written chunk 0"),
        "Should write first chunk"
    );
    assert!(
        stderr.contains("Written chunk 1"),
        "Should write second chunk"
    );
}
