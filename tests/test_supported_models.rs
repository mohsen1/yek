#[path = "integration_common.rs"]
mod integration_common;
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
    assert!(stdout.contains("Use with --tokens=MODEL"));
    assert!(stdout.contains("Available models:"));

    // Verify all models are listed
    let models = [
        "openai",   // OpenAI models
        "claude",   // Anthropic Claude models
        "mistral",  // Mistral models
        "deepseek", // DeepSeek models
        "llama",    // Meta Llama models
    ];

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
        .arg("--tokens=gpt-4o")
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
        stderr.contains("Unsupported model"),
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
tokenizer_model = "gpt-4o"
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
        stderr.contains("Unsupported model"),
        "Should indicate invalid model"
    );
}
