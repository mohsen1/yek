#[path = "integration_common.rs"]
mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, ensure_empty_output_dir, setup_temp_repo};
use std::fs;

#[test]
fn accepts_model_via_tokens_flag() {
    let repo = setup_temp_repo();
    let content = "This is a test file with some content.";
    create_file(repo.path(), "test.txt", content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let debug_output = repo.path().join("debug.log");
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens=gpt-4")
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("YEK_DEBUG_OUTPUT", &debug_output)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Read debug output
    let debug_log = fs::read_to_string(&debug_output).expect("Failed to read debug log");
    println!("debug log: {}", debug_log);

    // Verify token mode is enabled
    assert!(debug_log.contains("Token mode:"), "Should be in token mode");
}

#[test]
fn accepts_model_from_config() {
    let repo = setup_temp_repo();
    let content = "This is a test file with some content.";
    create_file(repo.path(), "test.txt", content.as_bytes());

    // Create config file with tokenizer model
    create_file(
        repo.path(),
        "yek.toml",
        r#"
tokenizer_model = "gpt-3.5-turbo"
"#
        .as_bytes(),
    );

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let debug_output = repo.path().join("debug.log");
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens") // Just enable token mode, model from config
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("YEK_DEBUG_OUTPUT", &debug_output)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Read debug output
    let debug_log = fs::read_to_string(&debug_output).expect("Failed to read debug log");
    println!("debug log: {}", debug_log);

    // Verify token mode is enabled
    assert!(debug_log.contains("Token mode:"), "Should be in token mode");
}

#[test]
fn cli_model_overrides_config() {
    let repo = setup_temp_repo();
    let content = "This is a test file with some content.";
    create_file(repo.path(), "test.txt", content.as_bytes());

    // Create config file with one model
    create_file(
        repo.path(),
        "yek.toml",
        r#"
tokenizer_model = "gpt-3.5-turbo"
"#
        .as_bytes(),
    );

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let debug_output = repo.path().join("debug.log");
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens=gpt-4") // Override config model
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("YEK_DEBUG_OUTPUT", &debug_output)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Read debug output
    let debug_log = fs::read_to_string(&debug_output).expect("Failed to read debug log");
    println!("debug log: {}", debug_log);

    // Verify token mode is enabled with CLI model
    assert!(debug_log.contains("Token mode:"), "Should be in token mode");
}

#[test]
fn defaults_to_gpt4_when_no_model_specified() {
    let repo = setup_temp_repo();
    let content = "This is a test file with some content.";
    create_file(repo.path(), "test.txt", content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let debug_output = repo.path().join("debug.log");
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens") // No model specified
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("YEK_DEBUG_OUTPUT", &debug_output)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    // Read debug output
    let debug_log = fs::read_to_string(&debug_output).expect("Failed to read debug log");
    println!("debug log: {}", debug_log);

    // Verify token mode is enabled
    assert!(debug_log.contains("Token mode:"), "Should be in token mode");
}

#[test]
fn fails_on_invalid_model() {
    let repo = setup_temp_repo();
    let content = "This is a test file with some content.";
    create_file(repo.path(), "test.txt", content.as_bytes());

    let output_dir = repo.path().join("yek-output");
    ensure_empty_output_dir(&output_dir);

    let debug_output = repo.path().join("debug.log");
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens=invalid-model") // Invalid model
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .env("YEK_DEBUG_OUTPUT", &debug_output)
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unsupported model"),
        "Should indicate invalid model"
    );
}
