use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use yek::config::FullYekConfig;
use yek::serialize_repo;

fn create_test_files(dir: &PathBuf, num_files: usize, file_size: usize) {
    for i in 0..num_files {
        let content = "a".repeat(file_size);
        let file_path = dir.join(format!("file_{}.txt", i));
        fs::write(file_path, content).unwrap();
    }
}

fn bench_serialize_repo(c: &mut Criterion) {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().join("yek-output");
    fs::create_dir_all(&output_dir).unwrap();

    // Create test files
    create_test_files(&temp.path().to_path_buf(), 100, 1000);

    c.bench_function("serialize_repo", |b| {
        b.iter(|| {
            let config = FullYekConfig {
                input_dirs: vec![temp.path().to_string_lossy().to_string()],
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
            serialize_repo(black_box(&config)).unwrap()
        })
    });
}

criterion_group!(benches, bench_serialize_repo);
criterion_main!(benches);
