use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use yek::serialize_repo;

fn create_test_data(dir: &Path, size: usize) {
    let filename = dir.join(format!("file_{}_bytes.txt", size));
    let data = vec![b'a'; size];
    fs::write(&filename, &data).unwrap();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    group.sample_size(10); // Number of samples to collect

    // Test different file sizes
    let sizes = vec![
        1024,             // 1KB
        1024 * 1024,      // 1MB
        10 * 1024 * 1024, // 10MB
    ];

    for size in sizes {
        // Create a new temporary directory for each benchmark
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("output");
        create_test_data(temp_dir.path(), size);

        group.bench_with_input(BenchmarkId::new("file_size", size), &size, |b, &size| {
            b.iter(|| {
                serialize_repo(
                    black_box(size),
                    Some(temp_dir.path()),
                    false,
                    false,
                    None,
                    Some(&output_dir),
                    None,
                )
                .unwrap();
                fs::remove_dir_all(&output_dir).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_serialization);
criterion_main!(benches);
