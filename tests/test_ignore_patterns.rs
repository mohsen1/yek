mod integration_common;
use assert_cmd::Command as AssertCommand;
use integration_common::{create_file, setup_temp_repo};

#[test]
fn respects_gitignore() {
    let repo = setup_temp_repo();
    let output_dir = repo.path().join("output");

    // Create and commit .gitignore and keep_me/foo.txt
    create_file(repo.path(), ".gitignore", b"ignore_me/**\n");
    create_file(repo.path(), "keep_me/foo.txt", b"should be included");

    // Create ignored file without adding to git (untracked)
    let ignore_me_dir = repo.path().join("ignore_me");
    std::fs::create_dir_all(&ignore_me_dir).unwrap();
    std::fs::write(
        ignore_me_dir.join("foo.txt"),
        "should be ignored".as_bytes(),
    )
    .unwrap();

    let mut cmd = AssertCommand::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

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
    let repo_path = repo.path().to_path_buf();
    let output_dir = repo_path.join("output");

    create_file(
        &repo_path,
        "yek.toml",
        r#"
ignore_patterns = [
    "dont_serialize/**"
]
"#
        .as_bytes(),
    );

    create_file(
        &repo_path,
        "dont_serialize/file.rs",
        "ignored by config".as_bytes(),
    );
    create_file(
        &repo_path,
        "do_serialize/file.rs",
        "should be included".as_bytes(),
    );

    let mut cmd = AssertCommand::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(&repo_path)
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        println!("Command failed with output:");
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());

    // Read the output file
    let output_file = output_dir.join("output.txt");
    let content = std::fs::read_to_string(output_file).expect("Failed to read output file");

    assert!(
        content.contains(">>>> do_serialize/file.rs"),
        "Should include non-ignored file"
    );
    assert!(
        !content.contains(">>>> dont_serialize/file.rs"),
        "Should not include ignored file"
    );
}
