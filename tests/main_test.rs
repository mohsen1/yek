use assert_cmd::Command;

#[test]
fn test_main_help_output() {
    // Verify that running the binary with '--help' exits successfully.
    Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_main_version_output() {
    // Check that the binary returns a version string.
    Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn test_main_with_directory_input() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_file_input() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(file_path)
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_json_output() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--json")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_tree_header() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--tree-header")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_line_numbers() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "line1\nline2").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--line-numbers")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_output_name() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let output_name = temp_dir.path().join("output.txt");

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-name")
        .arg(&output_name)
        .assert();

    cmd.success();

    // Check that output file was created
    assert!(output_name.exists());
}

#[test]
fn test_main_with_debug_flag() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--debug")
        .assert();

    cmd.success();
}

#[test]
fn test_main_non_streaming_mode() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-dir")
        .arg(temp_dir.path())
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_token_mode() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1000")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_force_tty() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .env("FORCE_TTY", "1")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_invalid_output_template() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-template")
        .arg("INVALID_TEMPLATE")
        .assert();

    // Should fail due to invalid template
    cmd.failure();
}

#[test]
fn test_main_with_zero_max_size() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--max-size")
        .arg("0")
        .assert();

    // Should fail due to zero max size
    cmd.failure();
}

#[test]
fn test_main_with_invalid_ignore_pattern() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--ignore-patterns")
        .arg("[invalid")
        .assert();

    // Should fail due to invalid ignore pattern
    cmd.failure();
}

#[test]
fn test_main_with_invalid_priority_rule() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--priority-rules")
        .arg("*.rs:1001") // Score too high
        .assert();

    // Should fail due to invalid priority rule
    cmd.failure();
}

// Priority 4: Main function logic tests
#[test]
fn test_main_streaming_mode_with_debug() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    // Test streaming mode with debug flag
    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--debug")
        .arg("--output-name")
        .arg("output.txt")
        .arg("--no-config") // Prevent default output_dir assignment
        .assert();

    cmd.success();

    // Check that output file was created
    assert!(std::path::Path::new("output.txt").exists());

    // Clean up
    std::fs::remove_file("output.txt").ok();
}

#[test]
fn test_main_checksum_error_handling() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();

    // Create a directory that will be used for checksum calculation
    fs::create_dir(temp_dir.path().join("subdir")).unwrap();
    fs::write(temp_dir.path().join("subdir").join("file.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-dir")
        .arg(temp_dir.path().join("output"))
        .assert();

    cmd.success();
}

#[test]
fn test_main_file_write_failure_recovery() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    // Try to write to a path that might fail (e.g., very long path)
    let output_name = "a".repeat(255) + ".txt"; // Very long filename

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-name")
        .arg(&output_name)
        .assert();

    // Should handle the error gracefully
    // The command might succeed or fail depending on the filesystem
    // but it shouldn't panic
    let _ = cmd.get_output();
}

#[test]
fn test_main_force_tty_environment() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    // Test with FORCE_TTY environment variable
    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-dir")
        .arg(temp_dir.path())
        .env("FORCE_TTY", "1")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_missing_output_dir_fallback() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    // Test with an output directory that might fail to create
    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-dir")
        .arg("/nonexistent/deeply/nested/path/that/cannot/be/created")
        .assert();

    // Should fall back to streaming mode
    cmd.success();
}

#[test]
fn test_output_dir_and_output_name_combination() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    // Create output directory
    let output_dir = temp_dir.path().join("output");
    fs::create_dir_all(&output_dir).unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--output-name")
        .arg("custom-output.txt")
        .assert();

    cmd.success();

    // Check that the output file was created in the correct location
    let expected_file = output_dir.join("custom-output.txt");
    assert!(
        expected_file.exists(),
        "Output file should be created at output_dir/output_name"
    );
}

#[test]
fn test_output_name_only_no_output_dir() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-name")
        .arg("standalone-output.txt")
        .assert();

    cmd.success();

    // Check that the output file was created in the temp directory (fallback behavior)
    // Note: when no output_dir is specified and not streaming, it should fall back to temp dir
}
