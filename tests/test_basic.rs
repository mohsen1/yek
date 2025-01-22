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
    let config = YekConfig {
        output_dir: Some(output_dir.clone()),
        ..Default::default()
    };
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

    let outputfile = output_dir.join("output.txt");
    assert!(outputfile.exists(), "Should write output file");
    let content = fs::read_to_string(outputfile).unwrap();
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
    let config = YekConfig {
        stream: true,
        ..Default::default()
    };
    serialize_repo(temp.path(), Some(&config)).unwrap();

    // The output should be written to stdout, which we can't easily capture in a test
    // So we just verify that the function runs without error
}
