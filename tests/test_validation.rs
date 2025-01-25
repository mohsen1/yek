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
        r#"ignore_patterns = ["["]  # invalid regex
"#
        .as_bytes(),
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path()).assert().failure().stderr(
        predicate::str::contains("Invalid configuration")
            .and(predicate::str::contains("Invalid pattern")),
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
pattern = ".*"
"#
        .as_bytes(),
    );

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path()).assert().failure().stderr(
        predicate::str::contains("Invalid configuration")
            .and(predicate::str::contains("must be between 0 and 1000")),
    );
}
