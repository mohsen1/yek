#[cfg(test)]
mod tree_config_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_tree_header_from_config_file() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        // Create yek.yaml with tree_header: true
        let config_content = "tree_header: true";
        fs::write(temp_dir.path().join("yek.yaml"), config_content).unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.current_dir(temp_dir.path()).arg(".");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("└── test.rs"))
            .stdout(predicate::str::contains(">>>> test.rs"));
    }

    #[test]
    fn test_tree_only_from_config_file() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        // Create yek.yaml with tree_only: true
        let config_content = "tree_only: true";
        fs::write(temp_dir.path().join("yek.yaml"), config_content).unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.current_dir(temp_dir.path()).arg(".");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("└── test.rs"))
            .stdout(predicate::str::contains(">>>> test.rs").not());
    }

    #[test]
    fn test_cli_overrides_config_boolean_fields() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        // Create yek.yaml with tree_only: true
        let config_content = "tree_only: true";
        fs::write(temp_dir.path().join("yek.yaml"), config_content).unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.current_dir(temp_dir.path())
            .arg("--tree-header")
            .arg(".");

        // CLI --tree-header should override config tree_only: true
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Directory structure:"))
            .stdout(predicate::str::contains("└── test.rs"))
            .stdout(predicate::str::contains(">>>> test.rs"));
    }

    #[test]
    fn test_config_mutual_exclusivity_validation() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        // Create yek.yaml with both tree_header and tree_only
        let config_content = "tree_header: true\ntree_only: true";
        fs::write(temp_dir.path().join("yek.yaml"), config_content).unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.current_dir(temp_dir.path()).arg(".");

        cmd.assert().failure().stderr(predicate::str::contains(
            "tree_header and tree_only cannot both be enabled",
        ));
    }

    #[test]
    fn test_config_json_tree_conflict_validation() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();

        // Create yek.yaml with both tree_header and json
        let config_content = "tree_header: true\njson: true";
        fs::write(temp_dir.path().join("yek.yaml"), config_content).unwrap();

        let mut cmd = Command::cargo_bin("yek").unwrap();
        cmd.current_dir(temp_dir.path()).arg(".");

        cmd.assert().failure().stderr(predicate::str::contains(
            "JSON output not supported with tree header mode",
        ));
    }
}
