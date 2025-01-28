mod integration_common;
use integration_common::{create_file, setup_temp_repo};
use std::fs;
use yek::{config::FullYekConfig, serialize_repo};

#[test]
fn test_gitignore_basic() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();

    // Create test files and commit them
    create_file(repo.path(), ".gitignore", b"ignore_me.txt\n");
    create_file(repo.path(), "ignore_me.txt", b"should be ignored");
    create_file(repo.path(), "keep_me.txt", b"should be kept");

    // Run serialization
    let output_dir = repo.path().join("test_output");
    fs::create_dir_all(&output_dir)?;

    let config = FullYekConfig {
        input_dirs: vec![repo.path().to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: output_dir.to_string_lossy().to_string(),
        ignore_patterns: vec![],
        priority_rules: vec![],
        binary_extensions: vec![],
        stream: false,
        token_mode: false,
        output_file_full_path: output_dir.join("chunk-0.txt").to_string_lossy().to_string(),
    };

    serialize_repo(&config)?;

    // Read all chunk contents
    let mut combined_content = String::new();
    for entry in fs::read_dir(&output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            combined_content.push_str(&fs::read_to_string(path)?);
        }
    }

    assert!(
        !combined_content.contains(">>>> ignore_me.txt"),
        "ignore_me.txt should be ignored"
    );
    assert!(
        combined_content.contains(">>>> keep_me.txt"),
        "keep_me.txt should be kept"
    );

    Ok(())
}

#[test]
fn test_gitignore_subdirectory() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();

    // Create test files and commit them
    create_file(repo.path(), ".gitignore", b"*.temp\n");

    // Create subdirectory with its own .gitignore
    let sub_dir = repo.path().join("subdir");
    fs::create_dir_all(&sub_dir)?;
    create_file(&sub_dir, ".gitignore", b"secret.conf\n");
    create_file(&sub_dir, "secret.conf", b"password=1234");
    create_file(&sub_dir, "app.rs", b"fn main() {}");

    // Create another directory without .gitignore
    let other_dir = repo.path().join("otherdir");
    fs::create_dir_all(&other_dir)?;
    create_file(&other_dir, "settings.temp", b"key=value");

    // Run serialization
    let output_dir = repo.path().join("test_output");
    fs::create_dir_all(&output_dir)?;

    let config = FullYekConfig {
        input_dirs: vec![repo.path().to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: output_dir.to_string_lossy().to_string(),
        ignore_patterns: vec![],
        priority_rules: vec![],
        binary_extensions: vec![],
        stream: false,
        token_mode: false,
        output_file_full_path: output_dir.join("chunk-0.txt").to_string_lossy().to_string(),
    };

    serialize_repo(&config)?;

    // Read all chunk contents
    let mut combined_content = String::new();
    for entry in fs::read_dir(&output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            combined_content.push_str(&fs::read_to_string(path)?);
        }
    }

    assert!(
        !combined_content.contains(">>>> otherdir/settings.temp"),
        "settings.temp should be ignored by root .gitignore"
    );
    assert!(
        !combined_content.contains(">>>> subdir/secret.conf"),
        "secret.conf should be ignored by subdirectory .gitignore"
    );
    assert!(
        combined_content.contains(">>>> subdir/app.rs"),
        "app.rs should be kept"
    );

    Ok(())
}

#[test]
fn test_gitignore_complex_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();

    // Create test files and commit them
    create_file(
        repo.path(),
        ".gitignore",
        b"# Comment
*.log
/build/
temp/*
!temp/keep.me
",
    );

    create_file(repo.path(), "error.log", b"logs");
    create_file(repo.path(), "build/output.exe", b"binary");
    create_file(repo.path(), "temp/junk.tmp", b"tmp");
    create_file(repo.path(), "temp/keep.me", b"important");
    create_file(repo.path(), "src/main.rs", b"fn main() {}");

    // Run serialization
    let output_dir = repo.path().join("test_output");
    fs::create_dir_all(&output_dir)?;

    let config = FullYekConfig {
        input_dirs: vec![repo.path().to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: output_dir.to_string_lossy().to_string(),
        ignore_patterns: vec![],
        priority_rules: vec![],
        binary_extensions: vec![],
        stream: false,
        token_mode: false,
        output_file_full_path: output_dir.join("chunk-0.txt").to_string_lossy().to_string(),
    };

    serialize_repo(&config)?;

    // Read all chunk contents
    let mut combined_content = String::new();
    for entry in fs::read_dir(&output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            combined_content.push_str(&fs::read_to_string(path)?);
        }
    }

    assert!(
        !combined_content.contains(">>>> error.log"),
        "error.log should be ignored"
    );
    assert!(
        !combined_content.contains(">>>> build/output.exe"),
        "build/output.exe should be ignored"
    );
    assert!(
        !combined_content.contains(">>>> temp/junk.tmp"),
        "temp/junk.tmp should be ignored"
    );
    assert!(
        combined_content.contains(">>>> temp/keep.me"),
        "temp/keep.me should be kept (negated pattern)"
    );
    assert!(
        combined_content.contains(">>>> src/main.rs"),
        "src/main.rs should be kept"
    );

    Ok(())
}

#[test]
fn test_gitignore_and_yek_toml() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();

    // Create yek.toml with ignore patterns
    create_file(
        repo.path(),
        "yek.toml",
        b"ignore_patterns = [\"^exclude/.*$\"]\n",
    );

    // Create .gitignore
    create_file(
        repo.path(),
        ".gitignore",
        b"*.tmp
/node_modules/
",
    );

    // Create test files and commit them
    create_file(repo.path(), "exclude/secret.txt", b"confidential");
    create_file(repo.path(), "test.tmp", b"temporary");
    create_file(repo.path(), "node_modules/lib.js", b"junk");
    create_file(repo.path(), "src/index.rs", b"fn main() {}");

    // Run serialization
    let output_dir = repo.path().join("test_output");
    fs::create_dir_all(&output_dir)?;

    let config = FullYekConfig {
        input_dirs: vec![repo.path().to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: output_dir.to_string_lossy().to_string(),
        ignore_patterns: vec!["*.tmp".to_string()],
        priority_rules: vec![],
        binary_extensions: vec![],
        stream: false,
        token_mode: false,
        output_file_full_path: output_dir.join("chunk-0.txt").to_string_lossy().to_string(),
    };

    serialize_repo(&config)?;

    // Read all chunk contents
    let mut combined_content = String::new();
    for entry in fs::read_dir(&output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            combined_content.push_str(&fs::read_to_string(path)?);
        }
    }

    assert!(
        !combined_content.contains(">>>> exclude/secret.txt"),
        "exclude/secret.txt should be ignored by yek.toml"
    );
    assert!(
        !combined_content.contains(">>>> test.tmp"),
        "test.tmp should be ignored by .gitignore"
    );
    assert!(
        !combined_content.contains(">>>> node_modules/lib.js"),
        "node_modules/lib.js should be ignored by .gitignore"
    );
    assert!(
        combined_content.contains(">>>> src/index.rs"),
        "src/index.rs should be kept"
    );

    Ok(())
}

#[test]
fn test_gitignore_binary_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();

    // Create test files with binary content
    create_file(repo.path(), "binary.jpg", b"\xFF\xD8\xFF\xDB"); // JPEG magic bytes
    create_file(repo.path(), "text.txt", b"normal text");
    create_file(repo.path(), "unknown.xyz", b"unknown format");

    // Run serialization
    let output_dir = repo.path().join("test_output");
    fs::create_dir_all(&output_dir)?;

    let config = FullYekConfig {
        input_dirs: vec![repo.path().to_string_lossy().to_string()],
        max_size: "10MB".to_string(),
        tokens: String::new(),
        debug: false,
        output_dir: output_dir.to_string_lossy().to_string(),
        ignore_patterns: vec![],
        priority_rules: vec![],
        binary_extensions: vec![],
        stream: false,
        token_mode: false,
        output_file_full_path: output_dir.join("chunk-0.txt").to_string_lossy().to_string(),
    };

    serialize_repo(&config)?;

    // Read all chunk contents
    let mut combined_content = String::new();
    for entry in fs::read_dir(&output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            combined_content.push_str(&fs::read_to_string(path)?);
        }
    }

    assert!(
        !combined_content.contains(">>>> binary.jpg"),
        "binary.jpg should be ignored as a binary file"
    );
    assert!(
        combined_content.contains(">>>> text.txt"),
        "text.txt should be kept"
    );
    assert!(
        !combined_content.contains(">>>> unknown.xyz"),
        "unknown.xyz should be ignored as a binary file (unknown extension)"
    );

    Ok(())
}
