//! Comprehensive benchmarking suite for stark-v zkVM.
//!
//! This benchmark provides detailed metrics including:
//! - Proof size measurements
//! - Component-level performance breakdown
//! - Memory profiling (with peak-alloc feature)
//! - Verification time measurements
//!
//! Usage:
//! ```bash
//! # Basic benchmark
//! cargo bench --package prover --bench comprehensive
//!
//! # With memory profiling
//! cargo bench --package prover --bench comprehensive --features peak-alloc
//!
//! # With parallelization
//! cargo bench --package prover --bench comprehensive --features parallel
//!
//! # Combined
//! cargo bench --package prover --bench comprehensive --features "parallel,peak-alloc"
//! ```

use divan::counter::ItemsCount;
use prover::e2e::{ensure_guest_built, guest_bin_dir};
use prover::{print_enabled_features, prove_rv32im, verify_rv32im};
use runner::run_with_input;
use stwo::core::fri::FriConfig;
use stwo::core::pcs::PcsConfig;

fn main() {
    divan::main();
}

const FIB_SIZES: &[u32] = &[100_000, 500_000, 1_000_000];

/// Comprehensive benchmark measuring proof generation with detailed metrics.
#[divan::bench(consts = FIB_SIZES, sample_count = 1)]
fn proof_generation<const N: u32>(bencher: divan::Bencher) {
    print_enabled_features();

    // Setup: build guest and load ELF
    ensure_guest_built();
    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");
    let input = N.to_le_bytes();

    // Run once to get cycle count
    let test_run =
        run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
    let cycles = test_run.cycles;

    println!("\n=== Proof Generation Benchmark (N={}) ===", N);
    println!("Cycles: {}", cycles);

    let config = PcsConfig {
        pow_bits: 24,
        fri_config: FriConfig::new(0, 1, 70),
    };

    bencher.counter(ItemsCount::new(cycles as usize)).bench(|| {
        #[cfg(feature = "peak-alloc")]
        prover::PEAK_ALLOC.reset_peak_usage();

        // Run and prove
        let run_result =
            run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
        let proof = prove_rv32im(run_result, config).expect("Failed to generate proof");

        // Measure proof size
        let proof_bytes = postcard::to_allocvec(&proof).expect("Failed to serialize proof");
        println!(
            "Proof size: {} bytes ({:.2} KB)",
            proof_bytes.len(),
            proof_bytes.len() as f64 / 1024.0
        );

        #[cfg(feature = "peak-alloc")]
        {
            let peak_mb = prover::PEAK_ALLOC.peak_usage_as_mb();
            println!("Peak memory: {:.2} MB", peak_mb);
        }

        divan::black_box(proof)
    });
}

/// Benchmark verification time separately from proving.
#[divan::bench(consts = FIB_SIZES, sample_count = 1)]
fn verification<const N: u32>(bencher: divan::Bencher) {
    print_enabled_features();

    // Setup: build guest and generate proof
    ensure_guest_built();
    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");
    let input = N.to_le_bytes();

    let config = PcsConfig {
        pow_bits: 24,
        fri_config: FriConfig::new(0, 1, 70),
    };

    // Generate proof once for verification benchmarking
    println!(
        "\n=== Generating proof for verification benchmark (N={}) ===",
        N
    );
    let run_result =
        run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
    let cycles = run_result.cycles;
    let proof = prove_rv32im(run_result, config).expect("Failed to generate proof");

    println!("=== Verification Benchmark (N={}) ===", N);
    println!("Cycles: {}", cycles);

    bencher.counter(ItemsCount::new(cycles as usize)).bench(|| {
        verify_rv32im(proof.clone(), config).expect("Verification failed");
        divan::black_box(())
    });
}

/// Benchmark proof size scaling across different input sizes.
#[divan::bench(consts = FIB_SIZES, sample_count = 1)]
fn proof_size<const N: u32>(bencher: divan::Bencher) {
    print_enabled_features();

    // Setup
    ensure_guest_built();
    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");
    let input = N.to_le_bytes();

    let config = PcsConfig {
        pow_bits: 24,
        fri_config: FriConfig::new(0, 1, 70),
    };

    bencher.bench(|| {
        let run_result =
            run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
        let cycles = run_result.cycles;
        let proof = prove_rv32im(run_result, config).expect("Failed to generate proof");

        // Serialize and measure
        let proof_bytes = postcard::to_allocvec(&proof).expect("Failed to serialize proof");
        let size_bytes = proof_bytes.len();

        println!(
            "N={}, Cycles={}, Proof size={} bytes ({:.2} KB), Bytes/Cycle={:.4}",
            N,
            cycles,
            size_bytes,
            size_bytes as f64 / 1024.0,
            size_bytes as f64 / cycles as f64
        );

        divan::black_box((proof, proof_bytes.len()))
    });
}

/// Component-level performance breakdown.
/// Measures time spent in different phases of proof generation.
#[divan::bench(consts = FIB_SIZES, sample_count = 1)]
fn component_breakdown<const N: u32>(bencher: divan::Bencher) {
    print_enabled_features();

    // Setup
    ensure_guest_built();
    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");
    let input = N.to_le_bytes();

    println!("\n=== Component Breakdown Benchmark (N={}) ===", N);

    let config = PcsConfig {
        pow_bits: 24,
        fri_config: FriConfig::new(0, 1, 70),
    };

    bencher.bench(|| {
        use std::time::Instant;

        // Measure VM execution
        let vm_start = Instant::now();
        let run_result =
            run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
        let vm_duration = vm_start.elapsed();
        let cycles = run_result.cycles;

        // Measure proving
        let prove_start = Instant::now();
        let proof = prove_rv32im(run_result, config).expect("Failed to generate proof");
        let prove_duration = prove_start.elapsed();

        // Measure verification
        let verify_start = Instant::now();
        verify_rv32im(proof.clone(), config).expect("Verification failed");
        let verify_duration = verify_start.elapsed();

        let total_duration = vm_duration + prove_duration + verify_duration;

        println!("Breakdown for {} cycles:", cycles);
        println!(
            "  VM execution:  {:>8.2}s ({:>5.1}%)",
            vm_duration.as_secs_f64(),
            100.0 * vm_duration.as_secs_f64() / total_duration.as_secs_f64()
        );
        println!(
            "  Proving:       {:>8.2}s ({:>5.1}%)",
            prove_duration.as_secs_f64(),
            100.0 * prove_duration.as_secs_f64() / total_duration.as_secs_f64()
        );
        println!(
            "  Verification:  {:>8.2}s ({:>5.1}%)",
            verify_duration.as_secs_f64(),
            100.0 * verify_duration.as_secs_f64() / total_duration.as_secs_f64()
        );
        println!("  Total:         {:>8.2}s", total_duration.as_secs_f64());
        println!(
            "  Throughput:    {:.2} kHz",
            cycles as f64 / total_duration.as_secs_f64() / 1000.0
        );

        divan::black_box(proof)
    });
}

/// Memory profiling benchmark (requires peak-alloc feature).
#[cfg(feature = "peak-alloc")]
#[divan::bench(consts = FIB_SIZES, sample_count = 1)]
fn memory_profile<const N: u32>(bencher: divan::Bencher) {
    print_enabled_features();

    // Setup
    ensure_guest_built();
    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");
    let input = N.to_le_bytes();

    println!("\n=== Memory Profile Benchmark (N={}) ===", N);

    let config = PcsConfig {
        pow_bits: 24,
        fri_config: FriConfig::new(0, 1, 70),
    };

    bencher.bench(|| {
        prover::PEAK_ALLOC.reset_peak_usage();

        // VM execution
        let vm_before = prover::PEAK_ALLOC.peak_usage_as_mb();
        let run_result =
            run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
        let vm_after = prover::PEAK_ALLOC.peak_usage_as_mb();
        let cycles = run_result.cycles;

        // Proving
        let prove_before = vm_after;
        let proof = prove_rv32im(run_result, config).expect("Failed to generate proof");
        let prove_after = prover::PEAK_ALLOC.peak_usage_as_mb();

        // Serialize for size
        let proof_bytes = postcard::to_allocvec(&proof).expect("Failed to serialize proof");

        println!("Memory usage for {} cycles:", cycles);
        println!("  VM execution:  {:.2} MB", vm_after - vm_before);
        println!("  Proving:       {:.2} MB", prove_after - prove_before);
        println!("  Peak total:    {:.2} MB", prove_after);
        println!(
            "  Proof size:    {:.2} KB",
            proof_bytes.len() as f64 / 1024.0
        );
        println!(
            "  Memory/Cycle:  {:.4} bytes/cycle",
            (prove_after * 1024.0 * 1024.0) / cycles as f64
        );

        divan::black_box(proof)
    });
}

/// End-to-end throughput benchmark with parallelization.
/// This measures aggregate throughput when running multiple proofs in parallel.
#[divan::bench(consts = FIB_SIZES, args = [1, 2, 4, 8], sample_count = 1)]
fn parallel_throughput<const N: u32>(bencher: divan::Bencher, par_count: usize) {
    print_enabled_features();

    // Setup
    ensure_guest_built();
    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");
    let input = N.to_le_bytes();

    // Get cycle count
    let test_run =
        run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
    let cycles = test_run.cycles;
    let total_cycles = cycles * par_count as u64;

    println!("\n=== Parallel Throughput (N={}, par={}) ===", N, par_count);

    let config = PcsConfig {
        pow_bits: 24,
        fri_config: FriConfig::new(0, 1, 70),
    };

    bencher
        .counter(ItemsCount::new(total_cycles as usize))
        .bench(|| {
            use rayon::iter::{IntoParallelIterator, ParallelIterator};

            #[cfg(feature = "peak-alloc")]
            prover::PEAK_ALLOC.reset_peak_usage();

            let proofs: Vec<_> = (0..par_count)
                .into_par_iter()
                .map(|_| {
                    let run_result = run_with_input(&elf_bytes, &input, 100_000_000)
                        .expect("Failed to run fib_input");
                    prove_rv32im(run_result, config).expect("Failed to generate proof")
                })
                .collect();

            #[cfg(feature = "peak-alloc")]
            {
                let peak_mb = prover::PEAK_ALLOC.peak_usage_as_mb();
                println!("Peak memory (parallel): {:.2} MB", peak_mb);
            }

            divan::black_box(proofs)
        });
}
