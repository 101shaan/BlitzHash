//! BlitzHash benchmark harness
//! Compares BlitzHash against SHA-256 with fair, reproducible tests

use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{Read, Write};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::path::PathBuf;

struct BenchConfig {
    file: Option<PathBuf>,
    size: usize,
    chunk: usize,
    threads: usize,
    seed: u64,
    repeat: usize,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            file: None,
            size: 100_000_000, // 100 MB default
            chunk: 65536,      // 64 KB chunks
            threads: 8,
            seed: 0,
            repeat: 3,
        }
    }
}

struct BenchResult {
    algorithm: String,
    threads: usize,
    chunk: usize,
    size: usize,
    seed: u64,
    mb_per_sec: f64,
    digest_hex: String,
}

fn parse_args() -> BenchConfig {
    let mut config = BenchConfig::default();
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--file" => {
                i += 1;
                config.file = Some(PathBuf::from(&args[i]));
            }
            "--size" => {
                i += 1;
                config.size = args[i].parse().expect("Invalid size");
            }
            "--chunk" => {
                i += 1;
                config.chunk = args[i].parse().expect("Invalid chunk size");
            }
            "--threads" => {
                i += 1;
                config.threads = args[i].parse().expect("Invalid thread count");
            }
            "--seed" => {
                i += 1;
                config.seed = args[i].parse().expect("Invalid seed");
            }
            "--repeat" => {
                i += 1;
                config.repeat = args[i].parse().expect("Invalid repeat count");
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    config
}

fn load_or_generate_data(config: &BenchConfig) -> Vec<u8> {
    if let Some(path) = &config.file {
        println!("📂 Loading file: {}", path.display());
        let mut file = File::open(path).expect("Failed to open file");
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect("Failed to read file");
        println!("   Loaded {} bytes ({:.2} MB)", data.len(), data.len() as f64 / 1_000_000.0);
        data
    } else {
        println!("🎲 Generating random data: {} bytes ({} MB)", config.size, config.size / 1_000_000);
        // Fast pseudo-random generation (not secure, just for benchmarking)
        let mut data = vec![0u8; config.size];
        let mut rng_state = 0x123456789abcdef0u64;
        for chunk in data.chunks_mut(8) {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let bytes = rng_state.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        data
    }
}

fn bench_sha256_streaming(data: &[u8], chunk_size: usize) -> (f64, String) {
    let start = Instant::now();
    let mut hasher = Sha256::new();
    
    for chunk in data.chunks(chunk_size) {
        hasher.update(chunk);
    }
    
    let result = hasher.finalize();
    let elapsed = start.elapsed().as_secs_f64();
    let mb_per_sec = (data.len() as f64 / 1_000_000.0) / elapsed;
    let digest = hex::encode(&result[..8]); // First 8 bytes for display
    
    (mb_per_sec, digest)
}

fn bench_blitzhash_single(data: &[u8], _chunk_size: usize, seed: u64) -> (f64, String) {
    let start = Instant::now();
    
    // Use optimized one-shot (no fake SIMD)
    let result = blitzhash::blitz_hash(seed, data);
    
    let elapsed = start.elapsed().as_secs_f64();
    let mb_per_sec = (data.len() as f64 / 1_000_000.0) / elapsed;
    let digest = hex::encode(&result[..8]);
    
    (mb_per_sec, digest)
}

fn bench_blitzhash_parallel(data: &[u8], threads: usize, seed: u64) -> (f64, String) {
    let start = Instant::now();
    let result = blitzhash::blitz_hash_parallel(seed, data, threads);
    let elapsed = start.elapsed().as_secs_f64();
    let mb_per_sec = (data.len() as f64 / 1_000_000.0) / elapsed;
    let digest = hex::encode(&result[..8]);
    
    (mb_per_sec, digest)
}

fn run_benchmark(config: &BenchConfig, data: &[u8]) -> Vec<BenchResult> {
    let mut results = Vec::new();
    
    println!("\n🔥 BENCHMARK CONFIGURATION");
    println!("   Data size: {} bytes ({:.2} MB)", data.len(), data.len() as f64 / 1_000_000.0);
    println!("   Chunk size: {} bytes", config.chunk);
    println!("   Threads: {}", config.threads);
    println!("   Seed: {}", config.seed);
    println!("   Repeats: {}", config.repeat);
    println!();

    // Warm-up
    print!("🔧 Warming up... ");
    std::io::stdout().flush().unwrap();
    let _ = bench_sha256_streaming(data, config.chunk);
    let _ = bench_blitzhash_single(data, config.chunk, config.seed);
    println!("done\n");

    // SHA-256 baseline (single-threaded)
    println!("📊 Running SHA-256 (baseline)...");
    let mut sha_speeds = Vec::new();
    for i in 0..config.repeat {
        print!("   Run {}/{}: ", i + 1, config.repeat);
        std::io::stdout().flush().unwrap();
        let (speed, digest) = bench_sha256_streaming(data, config.chunk);
        sha_speeds.push(speed);
        println!("{:.2} MB/s (digest: {}...)", speed, &digest[..16]);
        if i == 0 {
            results.push(BenchResult {
                algorithm: "SHA-256".to_string(),
                threads: 1,
                chunk: config.chunk,
                size: data.len(),
                seed: config.seed,
                mb_per_sec: speed,
                digest_hex: digest,
            });
        }
    }
    sha_speeds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let sha_median = sha_speeds[sha_speeds.len() / 2];
    println!("   Median: {:.2} MB/s\n", sha_median);

    // BlitzHash single-threaded
    println!("📊 Running BlitzHash-SIMD (single-threaded)...");
    let mut blitz_single_speeds = Vec::new();
    for i in 0..config.repeat {
        print!("   Run {}/{}: ", i + 1, config.repeat);
        std::io::stdout().flush().unwrap();
        let (speed, digest) = bench_blitzhash_single(data, config.chunk, config.seed);
        blitz_single_speeds.push(speed);
        println!("{:.2} MB/s (digest: {}...)", speed, &digest[..16]);
        if i == 0 {
            results.push(BenchResult {
                algorithm: "BlitzHash-SIMD".to_string(),
                threads: 1,
                chunk: config.chunk,
                size: data.len(),
                seed: config.seed,
                mb_per_sec: speed,
                digest_hex: digest,
            });
        }
    }
    blitz_single_speeds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let blitz_single_median = blitz_single_speeds[blitz_single_speeds.len() / 2];
    println!("   Median: {:.2} MB/s ({}x SHA-256)\n", 
             blitz_single_median, blitz_single_median / sha_median);

    // BlitzHash parallel
    println!("📊 Running BlitzHash (parallel, {} threads)...", config.threads);
    let mut blitz_parallel_speeds = Vec::new();
    for i in 0..config.repeat {
        print!("   Run {}/{}: ", i + 1, config.repeat);
        std::io::stdout().flush().unwrap();
        let (speed, digest) = bench_blitzhash_parallel(data, config.threads, config.seed);
        blitz_parallel_speeds.push(speed);
        println!("{:.2} MB/s (digest: {}...)", speed, &digest[..16]);
        if i == 0 {
            results.push(BenchResult {
                algorithm: "BlitzHash-MT".to_string(),
                threads: config.threads,
                chunk: config.chunk,
                size: data.len(),
                seed: config.seed,
                mb_per_sec: speed,
                digest_hex: digest,
            });
        }
    }
    blitz_parallel_speeds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let blitz_parallel_median = blitz_parallel_speeds[blitz_parallel_speeds.len() / 2];
    println!("   Median: {:.2} MB/s ({}x SHA-256)\n", 
             blitz_parallel_median, blitz_parallel_median / sha_median);

    results
}

fn print_results_table(results: &[BenchResult]) {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║                    BENCHMARK RESULTS                      ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║ Algorithm         │ Threads │  Chunk  │    MB/s │ Speedup ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    
    let baseline = results[0].mb_per_sec;
    for result in results {
        let speedup = result.mb_per_sec / baseline;
        println!("║ {:16} │ {:7} │ {:7} │ {:7.2} │ {:6.2}x ║",
                 result.algorithm,
                 result.threads,
                 format!("{}K", result.chunk / 1024),
                 result.mb_per_sec,
                 speedup);
    }
    
    println!("╚═══════════════════════════════════════════════════════════╝\n");
}

fn append_to_csv(results: &[BenchResult]) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let file_exists = std::path::Path::new("bench_results.csv").exists();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("bench_results.csv")
        .expect("Failed to open CSV file");
    
    if !file_exists {
        writeln!(file, "algorithm,threads,chunk,size,seed,mb_s,timestamp")
            .expect("Failed to write CSV header");
    }
    
    for result in results {
        writeln!(file, "{},{},{},{},{},{:.2},{}",
                 result.algorithm,
                 result.threads,
                 result.chunk,
                 result.size,
                 result.seed,
                 result.mb_per_sec,
                 timestamp)
            .expect("Failed to write CSV row");
    }
    
    println!("✅ Results appended to bench_results.csv");
}

fn main() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║                      BLITZHASH v0.1                       ║");
    println!("║            High-Performance Hash Benchmark                ║");
    println!("║                                                           ║");
    println!("║  ⚠️  NOT CRYPTOGRAPHICALLY SECURE - DEMO ONLY ⚠️           ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let config = parse_args();
    let data = load_or_generate_data(&config);
    let results = run_benchmark(&config, &data);
    
    print_results_table(&results);
    append_to_csv(&results);
    
    println!("\n🎉 Benchmark complete!");
    println!("\nNext steps:");
    println!("  1. Run: python viz/plot_results.py (to generate charts)");
    println!("  2. Try larger files: --size 1000000000 (1 GB)");
    println!("  3. Experiment with: --threads <n> --chunk <bytes>");
    println!();
}