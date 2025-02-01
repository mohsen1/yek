use assert_cmd::Command;

#[test]
fn test_main_help_output() {
    // Verify that running the binary with '--help' exits successfully.
    Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_main_version_output() {
    // Check that the binary returns a version string.
    Command::cargo_bin("yek")
        .expect("Binary 'yek' not found")
        .arg("--version")
        .assert()
        .success();
}
