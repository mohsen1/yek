use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_output_dir_and_output_name_combination() {
    // Test the fix for issue where both output_dir and output_name should work together
    // to create the pattern output_dir/output_name

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

    // Create config file with both output_dir and output_name
    let config_content = r#"
output_dir: ./test-output
output_name: custom-output.txt
"#;
    fs::write(temp_dir.path().join("yek.yaml"), config_content).unwrap();

    // Run yek from the temp directory
    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .current_dir(temp_dir.path())
        .arg(".")
        .assert();

    cmd.success();

    // Check that the output file was created in the expected location: output_dir/output_name
    let expected_path = temp_dir
        .path()
        .join("test-output")
        .join("custom-output.txt");
    assert!(
        expected_path.exists(),
        "Expected file at {:?} but it doesn't exist",
        expected_path
    );

    // Verify the file contains the expected content
    let content = fs::read_to_string(&expected_path).unwrap();
    assert!(content.contains("test content"));
    assert!(content.contains(">>>> test.txt"));
}

#[test]
fn test_cli_output_dir_and_output_name_combination() {
    // Test that CLI flags for both output_dir and output_name work together

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.md"), "test content").unwrap();

    // Run yek with both --output-dir and --output-name flags
    let output_dir = temp_dir.path().join("cli-output");
    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg(temp_dir.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--output-name")
        .arg("cli-output.txt")
        .assert();

    cmd.success();

    // Check that the output file was created in the expected location
    let expected_path = output_dir.join("cli-output.txt");
    assert!(
        expected_path.exists(),
        "Expected file at {:?} but it doesn't exist",
        expected_path
    );

    // Verify the file contains the expected content
    let content = fs::read_to_string(&expected_path).unwrap();
    assert!(content.contains("test content"));
    assert!(content.contains(">>>> test.md"));
}

#[test]
fn test_output_name_only_uses_current_directory() {
    // Test that when only output_name is provided (no output_dir),
    // the file is created in the current directory

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

    // Run yek with only --output-name flag (no config file)
    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .current_dir(temp_dir.path())
        .arg(".")
        .arg("--output-name")
        .arg("only-name.txt")
        .arg("--no-config")
        .assert();

    cmd.success();

    // Check that the output file was created in the current directory (temp_dir)
    let expected_path = temp_dir.path().join("only-name.txt");
    assert!(
        expected_path.exists(),
        "Expected file at {:?} but it doesn't exist",
        expected_path
    );

    // Verify the file contains the expected content
    let content = fs::read_to_string(&expected_path).unwrap();
    assert!(content.contains("test content"));
    assert!(content.contains(">>>> test.txt"));
}

#[test]
fn test_streaming_mode_with_output_name_and_output_dir() {
    // Test that streaming mode with both output_dir and output_name creates file correctly

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

    // Create config file with both output_dir and output_name
    let config_content = r#"
output_dir: ./stream-output
output_name: stream-output.txt
"#;
    fs::write(temp_dir.path().join("yek.yaml"), config_content).unwrap();

    // Run yek in streaming mode (by providing --output-name, it creates a file even in streaming context)
    let cmd = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .current_dir(temp_dir.path())
        .arg(".")
        .assert();

    cmd.success();

    // Check that the output file was created in the expected location
    let expected_path = temp_dir
        .path()
        .join("stream-output")
        .join("stream-output.txt");
    assert!(
        expected_path.exists(),
        "Expected file at {:?} but it doesn't exist",
        expected_path
    );

    // Verify the file contains the expected content
    let content = fs::read_to_string(&expected_path).unwrap();
    assert!(content.contains("test content"));
    assert!(content.contains(">>>> test.txt"));
}
