use std::fs;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use yek::serialize_repo;

struct PerfStats {
    min: Duration,
    max: Duration,
    avg: Duration,
    total_runs: usize,
}

impl PerfStats {
    fn new() -> Self {
        PerfStats {
            min: Duration::from_secs(u64::MAX),
            max: Duration::from_secs(0),
            avg: Duration::from_secs(0),
            total_runs: 0,
        }
    }

    fn update(&mut self, duration: Duration) {
        self.min = self.min.min(duration);
        self.max = self.max.max(duration);
        self.total_runs += 1;
        // Compute running average
        self.avg = (self.avg * (self.total_runs - 1) as u32 + duration) / self.total_runs as u32;
    }
}

#[test]
fn test_serialization_performance() -> Result<(), Box<dyn std::error::Error>> {
    const WARMUP_RUNS: usize = 2;
    const BENCH_RUNS: usize = 5;

    // Create test data directory using tempfile
    let test_dir = TempDir::new()?;
    let output_dir = TempDir::new()?;

    // Create test files of different sizes
    let sizes = vec![1024, 1024 * 1024, 10 * 1024 * 1024]; // 1KB, 1MB, 10MB

    println!("\nPerformance Benchmark Results:");
    println!("------------------------------");

    for size in sizes {
        let filename = test_dir.path().join(format!("file_{}_bytes.txt", size));
        let data = vec![b'a'; size];
        fs::write(&filename, &data)?;

        // Warmup runs
        println!("\nFile size: {}B", size);
        println!("Warmup runs...");
        for _ in 0..WARMUP_RUNS {
            serialize_repo(
                size,
                Some(test_dir.path()),
                false,
                false,
                None,
                Some(output_dir.path()),
                None,
            )?;
            // Clean output directory between runs
            for entry in fs::read_dir(output_dir.path())? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    fs::remove_file(entry.path())?;
                }
            }
        }

        // Benchmark runs
        let mut stats = PerfStats::new();
        println!("Benchmark runs...");

        for run in 1..=BENCH_RUNS {
            let start = Instant::now();
            serialize_repo(
                size,
                Some(test_dir.path()),
                false,
                false,
                None,
                Some(output_dir.path()),
                None,
            )?;
            let duration = start.elapsed();
            stats.update(duration);

            println!("  Run {}: {:?}", run, duration);
            // Clean output directory between runs
            for entry in fs::read_dir(output_dir.path())? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    fs::remove_file(entry.path())?;
                }
            }
        }

        println!("\nResults for {}B files:", size);
        println!("  Min: {:?}", stats.min);
        println!("  Max: {:?}", stats.max);
        println!("  Avg: {:?}", stats.avg);
    }

    Ok(())
}
