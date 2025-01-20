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

    // Read the first chunk file
    let chunk_0 = fs::read_to_string(output_dir.join("chunk-0.txt")).unwrap();
    println!("Chunk content:\n{}", chunk_0);

    // Check that very_important appears after less_important in the output
    let very_pos = chunk_0
        .find(">>>> very_important/one.txt")
        .expect("very_important/one.txt not found");
    let less_pos = chunk_0
        .find(">>>> less_important/two.txt")
        .expect("less_important/two.txt not found");
    assert!(
        very_pos > less_pos,
        "very_important should appear after less_important since higher priority files come last"
    );
}
