mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use predicates::prelude::*;
use std::fs;

#[test]
fn skips_known_binary_files() {
    let repo = setup_temp_repo();
    let binary_data = vec![0u8; 1024];
    let binary_path = repo.path().join("test.png");
    fs::write(&binary_path, &binary_data).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--stream")
        .assert()
        .success()
        // Ensure we don't see "test.png" in output
        .stdout(predicate::str::contains("test.png").not());
}

#[test]
fn respects_custom_binary_extensions() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
binary_extensions = [".xyz"]
"#,
    );
    // Create a file with .xyz extension
    let binary_data = vec![0u8; 1024];
    let xyz_path = repo.path().join("sample.xyz");
    fs::write(&xyz_path, &binary_data).unwrap();

    // Also create a normal text file
    create_file(repo.path(), "normal.txt", "some text");

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--stream")
        .assert()
        .success()
        // "sample.xyz" must be skipped
        .stdout(predicate::str::contains(">>>> normal.txt"))
        .stdout(predicate::str::contains("sample.xyz").not());
}
