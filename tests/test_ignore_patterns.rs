mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};

#[test]
fn respects_gitignore() {
    let repo = setup_temp_repo();
    println!("Created temp repo at: {}", repo.path().display());

    create_file(repo.path(), ".gitignore", "ignore_me/**\n");
    println!(
        "Created .gitignore at: {}",
        repo.path().join(".gitignore").display()
    );

    create_file(repo.path(), "ignore_me/foo.txt", "should be ignored");
    println!(
        "Created ignored file at: {}",
        repo.path().join("ignore_me/foo.txt").display()
    );

    create_file(repo.path(), "keep_me/foo.txt", "should be included");
    println!(
        "Created kept file at: {}",
        repo.path().join("keep_me/foo.txt").display()
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .arg("--stream")
        .arg("--debug")
        .assert()
        .success();

    // Print full output for debugging
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    println!("\nSTDOUT:\n{}", stdout);
    println!(
        "\nSTDERR:\n{}",
        String::from_utf8_lossy(&assert.get_output().stderr)
    );

    // Check that only the non-ignored file is in stdout
    assert!(
        stdout.contains(">>>> keep_me/foo.txt"),
        "Should include non-ignored file"
    );
    assert!(
        !stdout.contains(">>>> ignore_me/foo.txt"),
        "Should not include ignored file"
    );
}

#[test]
fn respects_custom_config_file() {
    let repo = setup_temp_repo();
    println!("Created temp repo at: {}", repo.path().display());

    create_file(
        repo.path(),
        "yek.toml",
        r#"
[ignore_patterns]
patterns = ["^dont_serialize/"]
"#,
    );
    println!(
        "Created yek.toml at: {}",
        repo.path().join("yek.toml").display()
    );

    create_file(repo.path(), "dont_serialize/file.rs", "ignored by config");
    println!(
        "Created ignored file at: {}",
        repo.path().join("dont_serialize/file.rs").display()
    );

    create_file(repo.path(), "do_serialize/file.rs", "should be included");
    println!(
        "Created kept file at: {}",
        repo.path().join("do_serialize/file.rs").display()
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .arg("--stream")
        .arg("--debug")
        .assert()
        .success();

    // Print full output for debugging
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    println!("\nSTDOUT:\n{}", stdout);
    println!(
        "\nSTDERR:\n{}",
        String::from_utf8_lossy(&assert.get_output().stderr)
    );

    // Check that only the non-ignored file is in stdout
    assert!(
        stdout.contains(">>>> do_serialize/file.rs"),
        "Should include non-ignored file"
    );
    assert!(
        !stdout.contains(">>>> dont_serialize/file.rs"),
        "Should not include ignored file"
    );
}
