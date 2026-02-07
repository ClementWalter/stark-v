//! Example demonstrating the performance profiler.
//!
//! This example shows how to use the profiler to analyze zkVM execution:
//! - Profile a trace to collect statistics
//! - Generate and display a performance report
//! - Export data for flame graph visualization
//!
//! # Usage
//!
//! ```bash
//! cargo run --package runner --example profile_execution
//! ```

use runner::{Profiler, run_with_input};
use std::fs;
use std::path::PathBuf;

fn main() {
    // Path to a guest program ELF (adjust as needed)
    let elf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../guest/target/riscv32im-unknown-none-elf/release/fib_input");

    if !elf_path.exists() {
        eprintln!("Guest program not found at {:?}", elf_path);
        eprintln!("Please build the guest programs first:");
        eprintln!("  cd guest && cargo build --release");
        return;
    }

    // Load ELF
    let elf_bytes = fs::read(&elf_path).expect("Failed to read ELF");

    // Input: compute Fibonacci(10000)
    let input = 10000u32.to_le_bytes();

    println!("Running guest program...");
    let run_result =
        run_with_input(&elf_bytes, &input, 10_000_000).expect("Failed to run guest program");

    println!("Execution completed with {} cycles", run_result.cycles);
    println!();

    // Profile the execution
    println!("Generating performance profile...");
    let profile = Profiler::profile_and_report(&run_result.tracer);

    // Display the report
    println!("{}", profile.report());

    // Export flame graph data
    println!("\nExporting flame graph data...");
    let folded = profile.to_flame_graph_folded();

    let output_path = PathBuf::from("/tmp/profile_folded.txt");
    fs::write(&output_path, folded.join("\n")).expect("Failed to write flame graph data");

    println!("Flame graph data written to: {:?}", output_path);
    println!();
    println!("To generate a flame graph SVG, use:");
    println!("  # Install flamegraph tool:");
    println!("  cargo install inferno");
    println!("  # Generate SVG:");
    println!(
        "  cat {} | inferno-flamegraph > /tmp/flamegraph.svg",
        output_path.display()
    );
    println!("  # View in browser:");
    println!("  open /tmp/flamegraph.svg");
}
