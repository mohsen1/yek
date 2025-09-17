use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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
#[test]
fn test_main_with_directory_input() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--stream")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_file_input() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(file_path)
        .arg("--stream")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_json_output() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--json")
        .arg("--stream")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_tree_header() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--tree-header")
        .arg("--stream")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_line_numbers() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "line1\nline2").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--line-numbers")
        .arg("--stream")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_output_name() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let output_name = temp_dir.path().join("output.txt");

    let mut cmd = Command::cargo_bin("yek")
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
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--debug")
        .arg("--stream")
        .assert();

    cmd.success();
}
#[test]
fn test_main_non_streaming_mode() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-dir")
        .arg(temp_dir.path())
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_token_mode() {
    use tempfile::tempdir;
    use std::fs;
#[test]
fn test_main_with_force_tty() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--stream")
        .env("FORCE_TTY", "1")
        .assert();

    cmd.success();
}

#[test]
fn test_main_with_invalid_output_template() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-template")
        .arg("INVALID_TEMPLATE")
        .arg("--stream")
        .assert();

    // Should fail due to invalid template
    cmd.failure();
}

#[test]
fn test_main_with_zero_max_size() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--max-size")
        .arg("0")
        .arg("--stream")
        .assert();

    // Should fail due to zero max size
    cmd.failure();
}

#[test]
fn test_main_with_invalid_ignore_pattern() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--ignore-patterns")
        .arg("[invalid")
        .arg("--stream")
        .assert();

    // Should fail due to invalid ignore pattern
    cmd.failure();
}

#[test]
fn test_main_with_invalid_priority_rule() {
    use tempfile::tempdir;
    use std::fs;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--priority-rules")
        .arg("*.rs:1001")  // Score too high
        .arg("--stream")
        .assert();

    // Should fail due to invalid priority rule
    cmd.failure();
}

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--tokens")
        .arg("1000")
        .arg("--stream")
        .assert();

    cmd.success();
}
}
