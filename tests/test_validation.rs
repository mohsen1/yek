mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use predicates::prelude::*;

#[test]
fn fails_on_invalid_regex_in_config() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
[ignore_patterns]
patterns = ["["] # invalid regex
"#,
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .assert()
        .success() // The tool doesn't "fail," it just logs invalid config
        .stderr(
            predicate::str::contains("Invalid configuration in")
                .and(predicate::str::contains("Invalid regex pattern")),
        );
}

#[test]
fn fails_on_negative_priority() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
[[priority_rules]]
score = -10
patterns = [".*"]
"#,
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path()).assert().success().stderr(
        predicate::str::contains("Invalid configuration in")
            .and(predicate::str::contains("must be between 0 and 1000")),
    );
}
