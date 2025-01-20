use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use yek::serialize_repo;
use yek::YekConfig;

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
            let mut config = YekConfig::default();
            config.output_dir = Some(output_dir.clone());
            serialize_repo(black_box(temp.path()), Some(&config)).unwrap()
        })
    });
}

criterion_group!(benches, bench_serialize_repo);
criterion_main!(benches);
