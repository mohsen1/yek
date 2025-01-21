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
        // OpenAI models
        "gpt-4o",
        "gpt-4o-2024-08-06",
        "chatgpt-4o-latest",
        "gpt-4o-mini",
        "gpt-4o-mini-2024-07-18",
        "o1",
        "o1-2024-12-17",
        "o1-mini",
        "o1-mini-2024-09-12",
        "o1-preview",
        "o1-preview-2024-09-12",
        "gpt-4o-realtime-preview",
        "gpt-4o-realtime-preview-2024-12-17",
        "gpt-4o-mini-realtime-preview",
        "gpt-4o-mini-realtime-preview-2024-12-17",
        "gpt-4o-audio-preview",
        "gpt-4o-audio-preview-2024-12-17",
        // Claude models
        "claude-3-5-sonnet-20241022",
        "claude-3-5-sonnet-latest",
        "claude-3-5-haiku-20241022",
        "claude-3-5-haiku-latest",
        "claude-3-opus-20240229",
        "claude-3-opus-latest",
        "claude-3-sonnet-20240229",
        "claude-3-haiku-20240307",
        // Mistral models
        "mistral-7b-v0-3",
        "mistral-nemo-12b",
        "mistral-openorca-7b",
        "mistral-large-123b",
        "mistral-small-22b",
        "mistrallite-7b",
        "mixtral-8x7b",
        "mixtral-8x22b",
        // Llama models
        "llama-3-3-70b",
        "llama-3-2-1b",
        "llama-3-2-3b",
        "llama-3-2-vision-11b",
        "llama-3-2-vision-90b",
        "llama-3-1-8b",
        "llama-3-1-70b",
        "llama-3-1-405b",
        "llama-3-8b",
        "llama-3-70b",
        "llama-2-7b",
        "llama-2-13b",
        "llama-2-70b",
        // Code Llama models
        "codellama-7b",
        "codellama-13b",
        "codellama-34b",
        "codellama-70b",
        // Tiny Llama models
        "tinyllama-1-1b",
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
