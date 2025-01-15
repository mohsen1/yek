mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};

#[test]
fn priority_rules_are_applied() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
[[priority_rules]]
score = 100
patterns = ["^very_important/"]

[[priority_rules]]
score = 10
patterns = ["^less_important/"]
"#,
    );
    create_file(repo.path(), "very_important/one.txt", "high priority");
    create_file(repo.path(), "less_important/two.txt", "lower priority");

    let mut cmd = Command::cargo_bin("yek").unwrap();
    let output = cmd
        .current_dir(repo.path())
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that less_important appears before very_important in the output
    let very_pos = stdout
        .find("very_important")
        .expect("very_important not found");
    let less_pos = stdout
        .find("less_important")
        .expect("less_important not found");
    assert!(
        less_pos < very_pos,
        "less_important should appear before very_important since higher priority files come last"
    );
}
