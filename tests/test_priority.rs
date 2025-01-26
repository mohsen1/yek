mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use std::fs;

#[test]
fn priority_rules_are_applied() {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        r#"
[[priority_rules]]
score = 1
pattern = "^very_important/"

[[priority_rules]]
score = 10
pattern = "^less_important/"
"#
        .as_bytes(),
    );
    create_file(
        repo.path(),
        "very_important/one.txt",
        "high priority".as_bytes(),
    );
    create_file(
        repo.path(),
        "less_important/two.txt",
        "lower priority".as_bytes(),
    );

    let output_dir = repo.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::cargo_bin("yek").unwrap();
    cmd.current_dir(repo.path())
        .arg("--output-dir")
        .arg(&output_dir)
        .assert()
        .success();

    // Read the output file
    let output = fs::read_to_string(output_dir.join("output.txt")).unwrap();
    println!("Output content:\n{}", output);

    // Check that very_important appears after less_important in the output
    let very_pos = output
        .find(">>>> very_important/one.txt")
        .expect("very_important/one.txt not found");
    let less_pos = output
        .find(">>>> less_important/two.txt")
        .expect("less_important/two.txt not found");
    assert!(
        very_pos > less_pos,
        "very_important should appear after less_important since higher priority files come last"
    );
}
