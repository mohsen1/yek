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
git_boost_max = 0

[[priority_rules]]
score = 10
pattern = "^very_important/"

[[priority_rules]]
score = 1
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

    let out_file = output_dir.with_extension("txt");
    assert!(out_file.exists(), "Expected output file");
    let content = fs::read_to_string(out_file).unwrap();
    println!("Output:\n{}", content);

    // "less_important" has lower priority => should appear first in the file
    // "very_important" is higher priority => should appear last
    let less_pos = content
        .find(">>>> less_important/two.txt")
        .expect("less_important/two.txt not found");
    let very_pos = content
        .find(">>>> very_important/one.txt")
        .expect("very_important/one.txt not found");

    // Higher priority => appears later in final text
    assert!(
        very_pos > less_pos,
        "very_important should appear later in the single output"
    );
}
