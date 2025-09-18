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
