use anyhow::{anyhow, Result};
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

    fn create_file(&mut self, path: &str, contents: impl AsRef<[u8]>) -> Result<&mut Self> {
        let full_path = self.dir.path().join(path);
        fs::create_dir_all(full_path.parent().ok_or_else(|| anyhow!("Invalid path"))?)?;
        fs::write(&full_path, contents)?;
        if self.git {
            self.git_add_and_commit(&format!("Add {}", path))?;
        }
        Ok(self)
    }

    fn create_binary_file(&mut self, path: &str, size: usize) -> Result<&mut Self> {
        let full_path = self.dir.path().join(path);
        let content = vec![0u8; size];
        fs::write(&full_path, content)?;
        if self.git {
            self.git_add_and_commit(&format!("Add {}", path))?;
        }
        Ok(self)
    }

    fn run(&self, args: &[&str]) -> Result<(String, String)> {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_yek"));
        cmd.args(args).current_dir(&self.dir);

        if let Some(config) = &self.config {
            let config_path = self.dir.path().join("yek.toml");
            fs::write(&config_path, config)?;
        }

        let output = cmd
            .output()
            .map_err(|e| anyhow!("Failed to run command: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        Ok((stdout, stderr))
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

    fn git_add_and_commit(&self, message: &str) -> Result<()> {
        Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(&self.dir)
            .output()
            .map_err(|e| anyhow!("Failed to git add: {}", e))?;

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.dir)
            .output()
            .map_err(|e| anyhow!("Failed to git commit: {}", e))?;

        Ok(())
    }
}

#[test]
fn test_basic_processing() -> Result<()> {
    let mut setup = TestSetup::new();
    setup
        .create_file("file1.txt", "content1")?
        .create_file("file2.txt", "content2")?;

    let (stdout, _) = setup.run(&[])?;
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    Ok(())
}

#[test]
fn test_ignore_file() -> Result<()> {
    let mut setup = TestSetup::new();
    setup
        .create_file(".gitignore", "*.txt")?
        .create_file("file1.txt", "content1")?
        .create_file("file2.rs", "content2")?;

    let (stdout, _) = setup.run(&[])?;
    assert!(!stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.rs"));
    Ok(())
}

#[test]
fn test_include_file() -> Result<()> {
    let mut setup = TestSetup::new();
    setup
        .create_file("file1.txt", "content1")?
        .create_file("file2.rs", "content2")?;

    let (stdout, _) = setup.run(&["--include", "*.txt"])?;
    assert!(stdout.contains("file1.txt"));
    assert!(!stdout.contains("file2.rs"));
    Ok(())
}

#[test]
fn test_git_integration() -> Result<()> {
    let mut setup = TestSetup::new();
    setup.with_git();
    setup
        .create_file("file1.txt", "content1")?
        .create_file("file2.txt", "content2")?;

    let (stdout, _) = setup.run(&[])?;
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    Ok(())
}

#[test]
fn test_dir_config() -> Result<()> {
    let mut setup = TestSetup::new();
    setup
        .with_config(
            r#"
            ignore_patterns = ["*.txt"]
            "#,
        )
        .create_file("file1.txt", "content1")?
        .create_file("file2.rs", "content2")?;

    let (stdout, _) = setup.run(&[])?;
    assert!(!stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.rs"));
    Ok(())
}

#[test]
fn test_max_size() -> Result<()> {
    let mut setup = TestSetup::new();
    setup
        .create_file("src/main.rs", "fn main() {}")?
        .create_file("image.png", "binary data")?
        .create_binary_file("big.bin", 1024)?;

    let (stdout, _) = setup.run(&["--max-size=200KB"])?;
    assert!(stdout.contains("src/main.rs"));
    assert!(!stdout.contains("image.png"));
    assert!(!stdout.contains("big.bin"));
    Ok(())
}

#[test]
fn test_invalid_config() -> Result<()> {
    let mut setup = TestSetup::new();
    setup.with_config("invalid toml");
    let (_, stderr) = setup.run(&["--max-size=200KB"])?;
    assert!(stderr.contains("Failed to parse config"));
    Ok(())
}
