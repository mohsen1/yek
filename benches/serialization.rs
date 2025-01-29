use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use rand::{distributions::Alphanumeric, Rng};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;
use yek::{config::YekConfig, serialize_repo};

/// Creates a text file of a specified size in bytes.
fn create_test_data_bytes(dir: &Path, size: usize, file_name: &str) {
    let filename = dir.join(file_name);
    let data = vec![b'a'; size];
    fs::write(&filename, &data).expect("Unable to write test data");
}

/// Creates a file with a specified approximate number of tokens.
fn create_test_data_tokens(dir: &Path, tokens: usize, file_name: &str) {
    let filename = dir.join(file_name);
    // Each "token" is a short random word followed by a space
    let mut rng = rand::thread_rng();
    let mut file = File::create(&filename).expect("Unable to create file");

    for _ in 0..tokens {
        let word: String = (0..4).map(|_| rng.sample(Alphanumeric) as char).collect();
        write!(file, "{} ", word).expect("Unable to write token");
    }
    file.flush().unwrap();
}

/// Creates multiple files of given sizes in a single directory.
fn create_multiple_files(dir: &Path, sizes: &[usize], prefix: &str) {
    for (i, &size) in sizes.iter().enumerate() {
        let file_name = format!("{}_{}.txt", prefix, i);
        create_test_data_bytes(dir, size, &file_name);
    }
}

/// Creates multiple files with a given token count each.
fn create_multiple_token_files(dir: &Path, tokens: &[usize], prefix: &str) {
    for (i, &token_count) in tokens.iter().enumerate() {
        let file_name = format!("{}_{}.txt", prefix, i);
        create_test_data_tokens(dir, token_count, &file_name);
    }
}

fn bench_single_small_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("SingleFile_ByteMode");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);
    let temp_dir = TempDir::new().unwrap();
    create_test_data_bytes(temp_dir.path(), 10 * 1024, "small_file.txt"); // 10 KB

    group.throughput(Throughput::Bytes((10 * 1024) as u64));
    group.bench_function("single_small_file", |b| {
        b.iter_batched(
            || {
                let output_dir = temp_dir.path().join("output");
                fs::create_dir_all(&output_dir).unwrap();
                output_dir
            },
            |output_dir| {
                let config = YekConfig::extend_config_with_defaults(
                    vec![temp_dir.path().to_string_lossy().to_string()],
                    output_dir.to_string_lossy().to_string(),
                );
                serialize_repo(&config).unwrap();
                fs::remove_dir_all(&output_dir).ok();
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn single_large_file_byte_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("SingleFile_ByteMode_Large");
    let temp_dir = TempDir::new().unwrap();

    let size = 10 * 1024 * 1024; // 10 MB
    create_test_data_bytes(temp_dir.path(), size, "large_file.txt");

    let output_dir = temp_dir.path().join("output");

    group.throughput(Throughput::Bytes(size as u64));
    group.bench_function("single_large_file", |b| {
        b.iter(|| {
            let config = YekConfig::extend_config_with_defaults(
                vec![temp_dir.path().to_string_lossy().to_string()],
                output_dir.to_string_lossy().to_string(),
            );
            serialize_repo(&config).unwrap();
            fs::remove_dir_all(&output_dir).ok();
        });
    });
    group.finish();
}

fn single_large_file_token_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("SingleFile_TokenMode_Large");
    let temp_dir = TempDir::new().unwrap();

    let token_count = 200_000;
    create_test_data_tokens(temp_dir.path(), token_count, "large_tokens.txt");

    let output_dir = temp_dir.path().join("output");

    group.throughput(Throughput::Elements(token_count as u64));
    group.bench_function("single_large_token_file", |b| {
        b.iter(|| {
            let config = YekConfig::extend_config_with_defaults(
                vec![temp_dir.path().to_string_lossy().to_string()],
                output_dir.to_string_lossy().to_string(),
            );
            serialize_repo(&config).unwrap();
            fs::remove_dir_all(&output_dir).ok();
        });
    });
    group.finish();
}

fn multiple_small_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("MultipleFiles_Small");
    group.bench_function("multiple_small_files", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                // Create a set of small files
                let sizes = vec![1024; 50]; // 50 files of 1KB each
                create_multiple_files(temp_dir.path(), &sizes, "small");
                let output_dir = temp_dir.path().join("output");
                (temp_dir, output_dir)
            },
            |(temp_dir, output_dir)| {
                let config = YekConfig::extend_config_with_defaults(
                    vec![temp_dir.path().to_string_lossy().to_string()],
                    output_dir.to_string_lossy().to_string(),
                );
                serialize_repo(&config).unwrap();
                fs::remove_dir_all(&output_dir).ok();
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn multiple_medium_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("MultipleFiles_Medium");
    group.bench_function("multiple_medium_files", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                // Create 20 files with sizes from 100KB to 500KB
                let sizes = (100..=500)
                    .step_by(20)
                    .map(|kb| kb * 1024)
                    .collect::<Vec<_>>();
                create_multiple_files(temp_dir.path(), &sizes, "medium");
                let output_dir = temp_dir.path().join("output");
                (temp_dir, output_dir)
            },
            |(temp_dir, output_dir)| {
                let config = YekConfig::extend_config_with_defaults(
                    vec![temp_dir.path().to_string_lossy().to_string()],
                    output_dir.to_string_lossy().to_string(),
                );
                serialize_repo(&config).unwrap();
                fs::remove_dir_all(&output_dir).ok();
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn multiple_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("MultipleFiles_Large");
    group.bench_function("multiple_large_files", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                // Create 5 large files, each ~ 5 MB
                let sizes = vec![5_242_880; 5]; // ~5 MB x 5
                create_multiple_files(temp_dir.path(), &sizes, "large");
                let output_dir = temp_dir.path().join("output");
                (temp_dir, output_dir)
            },
            |(temp_dir, output_dir)| {
                let config = YekConfig::extend_config_with_defaults(
                    vec![temp_dir.path().to_string_lossy().to_string()],
                    output_dir.to_string_lossy().to_string(),
                );
                serialize_repo(&config).unwrap();
                fs::remove_dir_all(&output_dir).ok();
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn multiple_token_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("MultipleFiles_TokenMode");
    group.bench_function("multiple_token_files", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                // Create 10 files with 10k tokens each
                let tokens = vec![10_000; 10];
                create_multiple_token_files(temp_dir.path(), &tokens, "token");
                let output_dir = temp_dir.path().join("output");
                (temp_dir, output_dir)
            },
            |(temp_dir, output_dir)| {
                let config = YekConfig::extend_config_with_defaults(
                    vec![temp_dir.path().to_string_lossy().to_string()],
                    output_dir.to_string_lossy().to_string(),
                );
                serialize_repo(&config).unwrap();
                fs::remove_dir_all(&output_dir).ok();
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// Demonstrates using a custom config (e.g. extra ignores or priority rules).
fn custom_config_test(c: &mut Criterion) {
    let mut group = c.benchmark_group("CustomConfig");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("output");
    let config_template = YekConfig::extend_config_with_defaults(
        vec![temp_dir.path().to_string_lossy().to_string()],
        output_dir.to_string_lossy().to_string(),
    );

    group.bench_function("custom_config_test", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                // Create mixed files
                create_test_data_bytes(temp_dir.path(), 1024, "test.txt");
                create_test_data_bytes(temp_dir.path(), 1024, "test.rs");
                let output_dir = temp_dir.path().join("output");
                let mut config = config_template.clone();
                config.input_dirs = vec![temp_dir.path().to_string_lossy().to_string()];
                config.output_dir = Some(output_dir.to_string_lossy().to_string());
                (temp_dir, output_dir, config)
            },
            |(_temp_dir, output_dir, config)| {
                serialize_repo(&config).unwrap();
                fs::remove_dir_all(&output_dir).ok();
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(1));
    targets = bench_single_small_file,
             single_large_file_byte_mode,
             single_large_file_token_mode,
             multiple_small_files,
             multiple_medium_files,
             multiple_large_files,
             multiple_token_files,
             custom_config_test
}

criterion_main!(benches);
