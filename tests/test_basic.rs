mod integration_common;
use std::fs;
use tempfile::TempDir;
use yek::serialize_repo;
use yek::YekConfig;

#[test]
fn basic_file_output_test() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    // Create a test file
    let test_file = temp.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Run serialization
    let mut config = YekConfig::default();
    config.output_dir = Some(output_dir.clone());
    serialize_repo(temp.path(), Some(&config)).unwrap();

    // Verify output
    println!("Output directory exists: {}", output_dir.exists());
    println!("Output directory contents:");
    for entry in fs::read_dir(&output_dir).unwrap() {
        let entry = entry.unwrap();
        println!("  {}", entry.path().display());
        let content = fs::read_to_string(entry.path()).unwrap();
        println!("File contents:\n{}", content);
    }

    // Check that the first chunk exists and contains our test file
    let chunk_0 = output_dir.join("test.txt.txt");
    assert!(chunk_0.exists(), "Should write first chunk");
    let content = fs::read_to_string(chunk_0).unwrap();
    assert!(
        content.contains("test content"),
        "Should contain file content"
    );
}

#[test]
fn basic_pipe_test() {
    let temp = TempDir::new().unwrap();

    // Create a test file
    let test_file = temp.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Run serialization in stream mode
    let mut config = YekConfig::default();
    config.stream = true;
    serialize_repo(temp.path(), Some(&config)).unwrap();

    // The output should be written to stdout, which we can't easily capture in a test
    // So we just verify that the function runs without error
}
