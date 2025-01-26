use std::fs;

#[test]
fn accepts_model_from_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("yek.toml");
    fs::write(
        &config_path,
        "tokenizer_model = \"openai\"\ntoken_mode = true\n",
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
