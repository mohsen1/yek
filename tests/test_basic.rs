mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use predicates::prelude::*;

#[test]
fn basic_stream_test() {
    let repo = setup_temp_repo();
    // Create a few files
    create_file(repo.path(), "src/main.rs", "fn main() {}");
    create_file(repo.path(), ".gitignore", "target/\n");

    // Invoke `yek --stream` so it outputs to stdout
    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--stream")
        .assert()
        .success()
        // Because we wrote one file, we expect ">>>> src/main.rs" in output
        .stdout(predicate::str::contains(">>>> src/main.rs"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn basic_file_output_test() {
    let repo = setup_temp_repo();
    create_file(repo.path(), "src/lib.rs", "// test content");
    // No .gitignore here for minimal config

    // `yek` will output to a temporary directory by default
    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Written chunk 0 with 1 files"));
}
