mod integration_common;
use assert_cmd::Command as AssertCommand;
use integration_common::{create_file, setup_temp_repo};
use std::process::Command;

#[test]
fn respects_gitignore() {
    let repo = setup_temp_repo();
    println!("Created temp repo at: {}", repo.path().display());

    create_file(repo.path(), ".gitignore", "ignore_me/**\n".as_bytes());
    println!(
        "Created .gitignore at: {}",
        repo.path().join(".gitignore").display()
    );

    create_file(
        repo.path(),
        "ignore_me/foo.txt",
        "should be ignored".as_bytes(),
    );
    println!(
        "Created ignored file at: {}",
        repo.path().join("ignore_me/foo.txt").display()
    );

    create_file(
        repo.path(),
        "keep_me/foo.txt",
        "should be included".as_bytes(),
    );
    println!(
        "Created kept file at: {}",
        repo.path().join("keep_me/foo.txt").display()
    );

    let mut cmd = AssertCommand::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .arg("--debug")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("\nSTDOUT:\n{}", stdout);
    println!("\nSTDERR:\n{}", String::from_utf8_lossy(&output.stderr));

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
    let repo_path = repo.path().to_path_buf(); // Store path before repo is moved
    println!("Created temp repo at: {}", repo_path.display());

    // Initialize git repo
    let status = Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .status()
        .expect("Failed to init git repo");
    assert!(status.success(), "git init failed");

    // Configure git user info
    let status = Command::new("git")
        .args(["config", "--global", "user.name", "Test User"])
        .status()
        .expect("Failed to configure git user name");
    assert!(status.success(), "git config user.name failed");

    let status = Command::new("git")
        .args(["config", "--global", "user.email", "test@example.com"])
        .status()
        .expect("Failed to configure git user email");
    assert!(status.success(), "git config user.email failed");

    create_file(
        &repo_path,
        "yek.toml",
        r#"
ignore_patterns = [
    "^dont_serialize/"
]
"#
        .as_bytes(),
    );
    println!(
        "Created yek.toml at: {}",
        repo_path.join("yek.toml").display()
    );

    create_file(
        &repo_path,
        "dont_serialize/file.rs",
        "ignored by config".as_bytes(),
    );
    println!(
        "Created ignored file at: {}",
        repo_path.join("dont_serialize/file.rs").display()
    );

    create_file(
        &repo_path,
        "do_serialize/file.rs",
        "should be included".as_bytes(),
    );
    println!(
        "Created kept file at: {}",
        repo_path.join("do_serialize/file.rs").display()
    );

    // Add and commit files
    let status = Command::new("git")
        .args(["add", "-f", "."])
        .current_dir(&repo_path)
        .status()
        .expect("Failed to add files to git");
    assert!(status.success(), "git add failed");

    // Print git status before commit
    let status = Command::new("git")
        .args(["status"])
        .current_dir(&repo_path)
        .status()
        .expect("Failed to get git status");
    assert!(status.success(), "git status failed");

    let status = Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .status()
        .expect("Failed to commit files");
    assert!(status.success(), "git commit failed");

    let mut cmd = AssertCommand::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(&repo_path)
        .arg("--debug")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("\nSTDOUT:\n{}", stdout);
    println!("\nSTDERR:\n{}", String::from_utf8_lossy(&output.stderr));

    // Check that only the non-ignored file is in stdout
    assert!(
        stdout.contains(">>>> do_serialize/file.rs"),
        "Should include non-ignored file"
    );
    assert!(
        !stdout.contains(">>>> dont_serialize/file.rs"),
        "Should not include ignored file"
    );

    // Keep repo alive until end of test
    drop(repo);
}
