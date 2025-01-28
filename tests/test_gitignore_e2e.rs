mod integration_common;
use integration_common::{create_file, setup_temp_repo};
use std::fs;
use yek::{config::FullYekConfig, serialize_repo};

#[test]
fn test_gitignore_basic() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    create_file(repo.path(), ".gitignore", b"ignore_me.txt\n");
    create_file(repo.path(), "ignore_me.txt", b"should be ignored");
    create_file(repo.path(), "keep_me.txt", b"should be kept");

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
        output_file_full_path: output_dir
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string(),
    };
    serialize_repo(&config)?;

    let out_file = output_dir.with_extension("txt");
    let combined_content = fs::read_to_string(&out_file)?;

    assert!(!combined_content.contains(">>>> ignore_me.txt"));
    assert!(combined_content.contains(">>>> keep_me.txt"));
    Ok(())
}

#[test]
fn test_gitignore_subdirectory() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    create_file(repo.path(), ".gitignore", b"*.temp\n");

    let sub_dir = repo.path().join("subdir");
    fs::create_dir_all(&sub_dir)?;
    create_file(&sub_dir, ".gitignore", b"secret.conf\n");
    create_file(&sub_dir, "secret.conf", b"password=1234");
    create_file(&sub_dir, "app.rs", b"fn main() {}");

    let other_dir = repo.path().join("otherdir");
    fs::create_dir_all(&other_dir)?;
    create_file(&other_dir, "settings.temp", b"key=value");

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
        output_file_full_path: output_dir
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string(),
    };
    serialize_repo(&config)?;

    let out_file = output_dir.with_extension("txt");
    let combined_content = fs::read_to_string(&out_file)?;

    assert!(!combined_content.contains(">>>> otherdir/settings.temp"));
    assert!(!combined_content.contains(">>>> subdir/secret.conf"));
    assert!(combined_content.contains(">>>> subdir/app.rs"));
    Ok(())
}

#[test]
fn test_gitignore_complex_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        ".gitignore",
        b"# Comment\n*.log\n/build/\ntemp/*\n!temp/keep.me\n",
    );
    create_file(repo.path(), "error.log", b"logs");
    create_file(repo.path().join("build").as_path(), "output.exe", b"binary");
    create_file(repo.path().join("temp").as_path(), "junk.tmp", b"tmp");
    create_file(repo.path().join("temp").as_path(), "keep.me", b"important");
    create_file(
        repo.path().join("src").as_path(),
        "main.rs",
        b"fn main() {}",
    );

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
        output_file_full_path: output_dir
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string(),
    };
    serialize_repo(&config)?;

    let out_file = output_dir.with_extension("txt");
    let combined_content = fs::read_to_string(&out_file)?;

    assert!(!combined_content.contains(">>>> error.log"));
    assert!(!combined_content.contains(">>>> build/output.exe"));
    assert!(!combined_content.contains(">>>> temp/junk.tmp"));
    assert!(combined_content.contains(">>>> temp/keep.me"));
    assert!(combined_content.contains(">>>> src/main.rs"));
    Ok(())
}

#[test]
fn test_gitignore_and_yek_toml() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    create_file(
        repo.path(),
        "yek.toml",
        b"ignore_patterns = [\"^exclude/.*$\"]\n",
    );
    create_file(repo.path(), ".gitignore", b"*.tmp\n/node_modules/\n");
    create_file(repo.path(), "exclude/secret.txt", b"confidential");
    create_file(repo.path(), "test.tmp", b"temporary");
    create_file(
        repo.path().join("node_modules").as_path(),
        "lib.js",
        b"junk",
    );
    create_file(
        repo.path().join("src").as_path(),
        "index.rs",
        b"fn main() {}",
    );

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
        output_file_full_path: output_dir
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string(),
    };
    serialize_repo(&config)?;

    let out_file = output_dir.with_extension("txt");
    let combined_content = fs::read_to_string(&out_file)?;

    assert!(!combined_content.contains(">>>> exclude/secret.txt"));
    assert!(!combined_content.contains(">>>> test.tmp"));
    assert!(!combined_content.contains(">>>> node_modules/lib.js"));
    assert!(combined_content.contains(">>>> src/index.rs"));
    Ok(())
}

#[test]
fn test_gitignore_binary_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo = setup_temp_repo();
    create_file(repo.path(), "binary.jpg", b"\xFF\xD8\xFF\xDB");
    create_file(repo.path(), "text.txt", b"normal text");
    create_file(repo.path(), "unknown.xyz", b"unknown format");

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
        output_file_full_path: output_dir
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string(),
    };
    serialize_repo(&config)?;

    let out_file = output_dir.with_extension("txt");
    let content = fs::read_to_string(&out_file)?;

    assert!(!content.contains(">>>> binary.jpg"));
    assert!(content.contains(">>>> text.txt"));
    assert!(!content.contains(">>>> unknown.xyz"));
    Ok(())
}
