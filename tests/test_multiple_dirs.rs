mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};

#[test]
fn multiple_directories_test() {
    let repo1 = setup_temp_repo();
    let repo2 = setup_temp_repo();

    create_file(repo1.path(), "file1.txt", "content1");
    create_file(repo2.path(), "file2.txt", "content2");

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.arg(repo1.path().to_str().unwrap())
        .arg(repo2.path().to_str().unwrap())
        .env("TERM", "dumb") // Force non-interactive mode
        .assert()
        .success()
        .stdout(predicates::str::contains("file1.txt"))
        .stdout(predicates::str::contains("content1"))
        .stdout(predicates::str::contains("file2.txt"))
        .stdout(predicates::str::contains("content2"));
}
