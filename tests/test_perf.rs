use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
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
fn test_serialization_performance() {
    const WARMUP_RUNS: usize = 2;
    const BENCH_RUNS: usize = 5;

    // Create test data directory
    let test_dir = "test_perf_data";
    fs::create_dir_all(test_dir).unwrap();

    // Create test files of different sizes
    let sizes = vec![1024, 1024 * 1024, 10 * 1024 * 1024]; // 1KB, 1MB, 10MB

    println!("\nPerformance Benchmark Results:");
    println!("------------------------------");

    for size in sizes {
        let filename = format!("{}/file_{}_bytes.txt", test_dir, size);
        let data = vec![b'a'; size];
        fs::write(&filename, &data).unwrap();

        // Warmup runs
        println!("\nFile size: {}B", size);
        println!("Warmup runs...");
        for _ in 0..WARMUP_RUNS {
            serialize_repo(
                size,
                Some(Path::new(test_dir)),
                false,
                false,
                None,
                Some(Path::new("perf_output")),
                None,
            )
            .unwrap();
            fs::remove_dir_all("perf_output").unwrap();
        }

        // Benchmark runs
        let mut stats = PerfStats::new();
        println!("Benchmark runs...");

        for run in 1..=BENCH_RUNS {
            let start = Instant::now();
            serialize_repo(
                size,
                Some(Path::new(test_dir)),
                false,
                false,
                None,
                Some(Path::new("perf_output")),
                None,
            )
            .unwrap();
            let duration = start.elapsed();
            stats.update(duration);

            println!("  Run {}: {:?}", run, duration);
            fs::remove_dir_all("perf_output").unwrap();
        }

        println!("\nStats for {}B:", size);
        println!("  Min: {:?}", stats.min);
        println!("  Max: {:?}", stats.max);
        println!("  Avg: {:?}", stats.avg);
    }

    // Final cleanup
    fs::remove_dir_all(test_dir).unwrap();
}
