use anyhow::Result;
use std::{fs, process::Command};
use tempfile::TempDir;

struct TestSetup {
    dir: TempDir,
    config: Option<String>,
    git: bool,
}

impl TestSetup {
    fn new() -> Self {
        TestSetup {
            dir: TempDir::new().unwrap(),
            config: None,
            git: false,
        }
    }

    fn with_config(&mut self, config: &str) -> &mut Self {
        self.config = Some(config.to_string());
        self
    }

    fn with_git(&mut self) -> &mut Self {
        self.git = true;
        self.git_init();
        self
    }

    fn create_file(&mut self, path: &str, contents: impl AsRef<[u8]>) -> &mut Self {
        let full_path = self.dir.path().join(path);
        fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        fs::write(&full_path, contents).unwrap();
        if self.git {
            self.git_add_and_commit(&format!("Add {}", path));
        }
        self
    }

    fn create_binary_file(&mut self, path: &str, size: usize) -> &mut Self {
        let full_path = self.dir.path().join(path);
        let content = vec![0u8; size];
        fs::write(&full_path, content).unwrap();
        if self.git {
            self.git_add_and_commit(&format!("Add {}", path));
        }
        self
    }

    fn run(&self, args: &[&str]) -> (String, String) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_yek"));

        // Create output directory
        let output_dir = self.dir.path().join("yek-output");
        fs::create_dir_all(&output_dir).unwrap();
        cmd.arg("--output-dir").arg(&output_dir);

        if let Some(config) = &self.config {
            let config_path = self.dir.path().join("yek.toml");
            fs::write(&config_path, config).unwrap();
            cmd.arg("--config").arg(config_path);
        }

        cmd.current_dir(self.dir.path());
        cmd.args(args);

        let output = cmd.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        // Read output file if it exists
        let output_file = output_dir.join("output.txt");
        if output_file.exists() {
            let content = fs::read_to_string(output_file).unwrap();
            (content, stderr)
        } else {
            (stdout, stderr)
        }
    }

    fn git_init(&self) {
        // Initialize git repo
        Command::new("git")
            .args(["init", "--quiet", "--initial-branch=main"])
            .current_dir(self.dir.path())
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.name", "test-user"])
            .current_dir(self.dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(self.dir.path())
            .output()
            .unwrap();

        // Create initial empty commit
        Command::new("git")
            .args(["commit", "--quiet", "--allow-empty", "-m", "Initial commit"])
            .current_dir(self.dir.path())
            .output()
            .unwrap();
    }

    fn git_add_and_commit(&self, message: &str) {
        Command::new("git")
            .args(["add", "."])
            .current_dir(self.dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(self.dir.path())
            .env("GIT_AUTHOR_DATE", "2024-01-01T00:00:00")
            .env("GIT_COMMITTER_DATE", "2024-01-01T00:00:00")
            .output()
            .unwrap();
    }
}

#[test]
fn test_basic_processing() -> Result<()> {
    let mut setup = TestSetup::new();
    setup
        .with_git()
        .create_file("src/main.rs", "fn main() {}")
        .create_file("image.png", "binary data")
        .create_binary_file("big.bin", 1024);

    let (output, _) = setup.run(&["--max-size=200KB"]);

    assert!(output.contains("src/main.rs"));
    assert!(!output.contains("image.png"));
    assert!(!output.contains("big.bin"));

    Ok(())
}

#[test]
fn test_ignore_file() -> Result<()> {
    let config = r#"ignore_patterns = ["temp/**"]"#;
    let mut setup = TestSetup::new();
    setup
        .with_git()
        .with_config(config)
        .create_file("temp/file.txt", "ignore")
        .create_file("app.log", "logs");
    let (output, _) = setup.run(&["--max-size=200KB"]);
    assert!(!output.contains("temp/file.txt"));
    assert!(output.contains("app.log"));
    Ok(())
}

#[test]
fn test_include_file() -> Result<()> {
    let config = r#"
        [[priority_rules]]
        pattern = "^tests/"
        score = 100
    "#;
    let mut setup = TestSetup::new();
    setup
        .with_git()
        .with_config(config)
        .create_file("src/a.rs", "a")
        .create_file("tests/b.rs", "b");
    let (output, _) = setup.run(&["--max-size=200KB"]);
    // Higher priority files (tests/) should appear last
    let pos_b = output.find("tests/b.rs").unwrap();
    let pos_a = output.find("src/a.rs").unwrap();
    assert!(pos_a < pos_b, "Higher priority file should come last");
    Ok(())
}

#[test]
fn test_git_integration() -> Result<()> {
    let mut setup = TestSetup::new();
    setup
        .with_git()
        .create_file("file1.txt", "1")
        .create_file("file2.txt", "2");

    let (output, _) = setup.run(&["--max-size=200KB"]);

    assert!(output.contains("file1.txt"));
    assert!(output.contains("file2.txt"));

    Ok(())
}

#[test]
fn test_dir_config() -> Result<()> {
    let global_config = r#"
        [[rules]]
        glob = "**/*"
        ignore = true
    "#;

    let dir_config = r#"
        [[rules]]
        glob = "**/*"
        include = true
    "#;

    let mut setup = TestSetup::new();
    setup
        .with_git()
        .with_config(global_config)
        .create_file("yek.toml", dir_config)
        .create_file("global_ignore", "");

    let (output, _) = setup.run(&["--max-size=200KB"]);

    assert!(output.contains("global_ignore"));

    Ok(())
}

#[test]
fn test_max_size() -> Result<()> {
    let mut setup = TestSetup::new();
    setup.with_git().create_file("test.txt", &"A".repeat(5000));

    // Test with size smaller than header + minimal content
    let (output, _) = setup.run(&["--max-size=10"]);
    assert!(!output.contains("test.txt"));

    // Test with sufficient size
    let (output, _) = setup.run(&["--max-size=200KB"]);
    assert!(output.contains("test.txt"));

    Ok(())
}

#[test]
fn test_invalid_config() -> Result<()> {
    let mut setup = TestSetup::new();
    setup.with_config("invalid toml");
    let (_, stderr) = setup.run(&["--max-size=200KB"]);
    assert!(stderr.contains("Failed to parse config"));
    Ok(())
}
