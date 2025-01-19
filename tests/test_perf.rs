use std::fs;
use std::path::Path;
use std::time::Instant;
use yek::serialize_repo;

#[test]
fn test_serialization_performance() {
    // Create test data directory
    let test_dir = "test_perf_data";
    fs::create_dir_all(test_dir).unwrap();

    // Create test files of different sizes
    let sizes = vec![1024, 1024 * 1024, 10 * 1024 * 1024]; // 1KB, 1MB, 10MB

    for size in sizes {
        let filename = format!("{}/file_{}_bytes.txt", test_dir, size);
        let data = vec![b'a'; size];
        fs::write(&filename, &data).unwrap();

        // Measure serialization time
        let start = Instant::now();
        serialize_repo(
            size,                           // max_size
            Some(Path::new(test_dir)),      // base_path
            false,                          // stream
            false,                          // count_tokens
            None,                           // config
            Some(Path::new("perf_output")), // output_dir
            None,                           // max_files
        )
        .unwrap();
        let duration = start.elapsed();

        println!("Serializing {}B took: {:?}", size, duration);

        // Cleanup
        fs::remove_dir_all("perf_output").unwrap();
    }

    // Final cleanup
    fs::remove_dir_all(test_dir).unwrap();
}
