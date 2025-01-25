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
    cmd.current_dir(repo.path())
        .arg("--debug")
        .arg("--output-dir")
        .arg(&output_dir)
        .output()
        .expect("Failed to execute command");

    // Read the output file
    let output_file = output_dir.join("output.txt");
    let content = std::fs::read_to_string(output_file).expect("Failed to read output file");

    assert!(
        content.contains(">>>> keep_me/foo.txt"),
        "Should include non-ignored file"
    );
    assert!(
        !content.contains(">>>> ignore_me/foo.txt"),
        "Should not include ignored file"
    );
}
