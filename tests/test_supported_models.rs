#[path = "integration_common.rs"]
mod integration_common;
use std::fs;

use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};

#[test]
fn test_supported_models_list() {
    let repo = setup_temp_repo();
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify help output contains supported models section
    assert!(stdout.contains("SUPPORTED MODELS:"));
    assert!(stdout.contains("possible values:"));
    assert!(stdout.contains("<MODEL>")); // Added check for placeholder

    // Verify all models are listed
    let models = yek::model_manager::SUPPORTED_MODEL_FAMILIES;

    for model in models {
        assert!(
            stdout.contains(model),
            "Help output should contain model: {}",
            model
        );
    }
}

#[test]
fn test_model_validation() {
    let repo = setup_temp_repo();
    let content = "Test content";
    create_file(repo.path(), "test.txt", content.as_bytes());

    // Test with valid model
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens=openai")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command should succeed with valid model"
    );

    // Test with invalid model
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--tokens=invalid-model")
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Command should fail with invalid model"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value"),
        "Should indicate invalid model"
    );
}

#[test]
fn test_model_from_config() {
    let repo = setup_temp_repo();
    let content = "Test content";
    create_file(repo.path(), "test.txt", content.as_bytes());

    // Create config with valid model
    create_file(
        repo.path(),
        "yek.toml",
        r#"
tokenizer_model = "openai"
token_mode = true
"#
        .as_bytes(),
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command should succeed with valid model in config"
    );

    // Test with invalid model in config
    create_file(
        repo.path(),
        "yek.toml",
        r#"
tokenizer_model = "invalid-model"
token_mode = true
"#
        .as_bytes(),
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Command should fail with invalid model in config"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unsupported tokenizer model"),
        "Should indicate unsupported model"
    );
}

#[test]
fn test_token_mode_true_without_model_uses_default() {
    let repo = setup_temp_repo();
    create_file(repo.path(), "test.txt", "Test content".as_bytes());

    // Create config with token_mode = true and no tokenizer_model
    create_file(repo.path(), "yek.toml", r#"token_mode = true"#.as_bytes());

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .env("YEK_DEBUG_OUTPUT", repo.path().join("debug.log"))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command should succeed");

    // Check debug log for the correct model
    let debug_log = fs::read_to_string(repo.path().join("debug.log")).unwrap();
    assert!(
        debug_log.contains("Token mode enabled with model: openai"),
        "Should use default model 'openai' when token_mode is true and no model is specified"
    );
}

#[test]
fn test_token_mode_default_model() {
    let repo = setup_temp_repo();
    let content = "Test content";
    create_file(repo.path(), "test.txt", content.as_bytes());

    // Create config with only token_mode = true
    create_file(
        repo.path(),
        "yek.toml",
        r#"
token_mode = true
"#
        .as_bytes(),
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command should succeed with default openai tokenizer"
    );
}
