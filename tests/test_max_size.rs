#[path = "integration_common.rs"]
mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, ensure_empty_output_dir, setup_temp_repo};
use std::fs;
use yek::model_manager;

/// Writes a file larger than the default 10MB limit in tokens or bytes, forcing trimming.
#[test]
fn trims_large_file_in_bytes_mode() {
    let _ = env_logger::builder().is_test(true).try_init();
    println!("Starting bytes mode test");

    let repo = setup_temp_repo();
    println!("Temp repo path: {}", repo.path().display());

    let large_content = "A ".repeat(1024 * 100); // ~ 100KB
    println!("Created content with size: {} bytes", large_content.len());

    create_file(repo.path(), "BIG.txt", large_content.as_bytes());
    println!(
        "Created test file: {}",
        repo.path().join("BIG.txt").display()
    );

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);
    println!("Cleared output directory: {}", output_dir.display());

    let mut cmd = Command::cargo_bin("yek").unwrap();
    println!("Running command with --max-size=50KB");
    let output = cmd
        .current_dir(repo.path())
        .arg("--max-size=50KB")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--debug")
        .arg(repo.path())
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Command stderr:\n{}", stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Command stdout:\n{}", stdout);
    println!("Command exit status: {}", output.status);

    assert!(output.status.success());

    let output_file = output_dir.join("output.txt");
    println!("Checking output file: {}", output_file.display());
    assert!(output_file.exists(), "output.txt should exist");

    assert!(stdout.contains(&output_file.display().to_string()));

    let content = fs::read_to_string(&output_file).expect("Failed to read output file");
    println!("Output file size: {} bytes", content.len());

    assert!(
        content.len() <= 51200, // 50KB = 50 * 1024 bytes
        "File content length should be 51200 bytes (50KB, including headers), but was {} bytes",
        content.len()
    );
}

#[test]
fn trims_large_file_in_token_mode() {
    let _ = env_logger::builder().is_test(true).try_init();
    println!("Starting token mode test");

    let repo = setup_temp_repo();
    println!("Temp repo path: {}", repo.path().display());

    // Each "word" is a token
    let large_content = r"
200 tokens exactly! Okay, let's try to figure out why the test is failing. The user mentioned that the command isn't writing to disk when using `--tokens`, and the test output shows that the stdout has the content of the file but the output.txt isn't being created.
First, I need to look at how the output is handled in the code. In `src/lib.rs`, the `process_directory` function checks if `config.stream` is true. If it is, it prints the output to stdout. Otherwise, it writes to the output directory.
Wait, in the test, when using `--tokens`, maybe the `stream` configuration is being set incorrectly. Let me check the `Args` struct in `src/main.rs`. There's a line where `config.stream` is set based on whether stdout is a terminal. But during tests, when running the command, stdout might not be a terminal, so `stream` would be true, causing it to print to stdout instead of writing";
    println!(
        "Created test content with length: {} bytes",
        large_content.len()
    );

    create_file(repo.path(), "BIG_token.txt", large_content.as_bytes());
    println!(
        "Created test file: {}",
        repo.path().join("BIG_token.txt").display()
    );

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);
    println!("Cleared output directory: {}", output_dir.display());

    let mut cmd = Command::cargo_bin("yek").unwrap();
    println!("Running command with --tokens=openai and --max-size=150");
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens=openai")
        .arg("--max-size")
        .arg("150")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--debug")
        .arg(repo.path())
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Command stderr:\n{}", stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Command stdout:\n{}", stdout);
    println!("Command exit status: {}", output.status);

    assert!(output.status.success());

    let output_file = output_dir.join("output.txt");
    println!("Checking output file: {}", output_file.display());
    assert!(output_file.exists(), "output.txt should exist");

    assert!(
        stdout.contains(&output_file.display().to_string()),
        "stdout should contain path of output file"
    );

    let content = fs::read_to_string(&output_file).expect("Failed to read output file");
    let token_count = model_manager::count_tokens(&content, "openai").unwrap();
    println!("Output file token count: {}", token_count);
    println!("Output file content:\n{}", content);

    assert!(token_count <= 150, "Should not exceed token limit");
    assert!(token_count >= 100, "Should preserve most important content");
}
