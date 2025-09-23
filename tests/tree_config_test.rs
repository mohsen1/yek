use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_tree_options_from_config_file() {
    // Create a test directory structure
    let test_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = test_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    fs::write(src_dir.join("main.rs"), "fn main() {}").expect("Failed to write main.rs");
    fs::write(test_dir.path().join("test.txt"), "test content").expect("Failed to write test.txt");

    // Create config file with tree_header option
    let config_content = format!(
        "tree_header: true\ninput_paths:\n  - \"{}\"",
        test_dir.path().to_string_lossy()
    );
    let config_file = test_dir.path().join("yek.yaml");
    fs::write(&config_file, config_content).expect("Failed to write config file");

    // Test with config file
    let output = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg("--config-file")
        .arg(&config_file)
        .output()
        .expect("Failed to execute command");

    let output_str = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should contain directory structure if tree_header is working
    assert!(
        output_str.contains("Directory structure:"),
        "tree_header option not working from config file. Output: {}",
        output_str
    );
}

#[test]
fn test_tree_only_from_config_file() {
    // Create a test directory structure
    let test_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = test_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    fs::write(src_dir.join("main.rs"), "fn main() {}").expect("Failed to write main.rs");
    fs::write(test_dir.path().join("test.txt"), "test content").expect("Failed to write test.txt");

    // Create config file with tree_only option
    let config_content = format!(
        "tree_only: true\ninput_paths:\n  - \"{}\"",
        test_dir.path().to_string_lossy()
    );
    let config_file = test_dir.path().join("yek.yaml");
    fs::write(&config_file, config_content).expect("Failed to write config file");

    // Test with config file
    let output = Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg("--config-file")
        .arg(&config_file)
        .output()
        .expect("Failed to execute command");

    let output_str = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should only contain directory structure, not file contents
    assert!(
        output_str.contains("Directory structure:"),
        "tree_only option not working from config file. Output: {}",
        output_str
    );
    assert!(
        !output_str.contains("fn main() {}"),
        "tree_only should not show file contents. Output: {}",
        output_str
    );
}
