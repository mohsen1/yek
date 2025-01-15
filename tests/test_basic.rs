mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use predicates::prelude::*;
use std::process::Stdio;

#[test]
fn basic_pipe_test() {
    let repo = setup_temp_repo();
    // Create a few files
    create_file(repo.path(), "src/main.rs", "fn main() {}");
    create_file(repo.path(), ".gitignore", "target/\n");

    // Run with stdout piped to simulate piping
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(">>>> src/main.rs"));
}

#[test]
fn basic_file_output_test() {
    let repo = setup_temp_repo();
    create_file(repo.path(), "src/lib.rs", "// test content");
    // No .gitignore here for minimal config

    // `yek` will output to a temporary directory by default when not piped
    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Written chunk 0 with 1 files"));
}
