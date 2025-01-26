mod integration_common;
use integration_common::assert_output_file_contains;
use predicates::prelude::*;
use std::fs;

#[test]
fn cli_model_overrides_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("yek.toml");
    fs::write(
        &config_path,
        "tokenizer_model = \"mistral\"\ntokens = true\n",
    )
    .unwrap();

    let test_file_path = temp_dir.path().join("test.txt");
    fs::write(
        &test_file_path,
        "This is a simple file with some words in it.\n",
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("yek").unwrap();
    cmd.arg("--config")
        .arg(config_path)
        .arg("--tokens=deepseek") // Should override config
        .arg(temp_dir.path())
        .arg("--debug") // Add debug flag to enable debug logging
        .env("YEK_DEBUG_OUTPUT", temp_dir.path().join("debug.log"))
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Token mode enabled with model: deepseek",
        ));

    assert_output_file_contains(
        temp_dir.path(),
        &[
            ">>>> test.txt",
            "This is a simple file with some words in it.\n",
        ],
    );

    // Clean up the temporary directory
    temp_dir.close().unwrap();
}

#[test]
fn accepts_model_from_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("yek.toml");
    fs::write(
        &config_path,
        "tokenizer_model = \"openai\"\ntokens = true\n",
    )
    .unwrap();

    // Create a dummy test.txt file
    fs::write(temp_dir.path().join("test.txt"), "Test content\n").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("yek").unwrap();
    cmd.arg("--config")
        .arg(config_path)
        .arg(temp_dir.path())
        .env("YEK_DEBUG_OUTPUT", temp_dir.path().join("debug.log"))
        .assert()
        .success();

    // Verify that the debug log contains the expected message
    let debug_log = fs::read_to_string(temp_dir.path().join("debug.log")).unwrap();
    assert!(
        debug_log.contains("Token mode enabled with model: openai"),
        "Should enable token mode with model from config"
    );

    // Clean up the temporary directory
    temp_dir.close().unwrap();
}

#[test]
fn default_tokens_is_false() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("yek").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--debug") // Add debug flag to enable debug logging
        .assert()
        .success()
        .stdout(predicate::str::contains("Token mode enabled").not());

    // Clean up the temporary directory
    temp_dir.close().unwrap();
}

#[test]
fn cli_tokens_enables_token_mode() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("yek").unwrap();
    cmd.arg("--tokens")
        .arg(temp_dir.path())
        .arg("--debug") // Add debug flag to enable debug logging
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Token mode enabled with model: openai",
        ));

    // Clean up the temporary directory
    temp_dir.close().unwrap();
}

#[test]
fn counts_tokens_using_tokenizer() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file_path = temp_dir.path().join("test.txt");
    fs::write(
        &test_file_path,
        "This is a simple file with some words in it.\n",
    )
    .unwrap();

    // Create a temporary config file specifying the model
    let config_path = temp_dir.path().join("yek.toml");
    fs::write(
        &config_path,
        "tokenizer_model = \"deepseek\"\ntokens = true\n",
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("yek").unwrap();
    cmd.arg("--config")
        .arg(&config_path)
        .arg(temp_dir.path())
        .arg("--debug") // Add debug flag to enable debug logging
        .assert()
        .success()
        .stdout(predicate::str::contains("deepseek"));

    // Verify that the test file content is in the output
    assert_output_file_contains(
        temp_dir.path(),
        &[
            ">>>> test.txt",
            "This is a simple file with some words in it.\n",
        ],
    );

    // Clean up the temporary directory
    temp_dir.close().unwrap();
}

#[test]
fn unsupported_model() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("yek").unwrap();
    cmd.arg("--tokens=unsupported_model")
        .arg(temp_dir.path())
        .arg("--debug") // Add debug flag to enable debug logging
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported model"));
}
