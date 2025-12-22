use clap::Parser;
use runner::run;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};

/// RV32IM interpreter for zkVM guest programs.
#[derive(Parser)]
#[command(name = "runner", version, about)]
struct Cli {
    /// Path to the ELF file to execute.
    elf: PathBuf,

    /// Maximum number of cycles before aborting.
    #[arg(short, long, default_value_t = 10_000_000)]
    max_cycles: u64,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let elf_bytes = match fs::read(&cli.elf) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(path = ?cli.elf, "Failed to read ELF file: {e}");
            std::process::exit(1);
        }
    };

    match run(&elf_bytes, cli.max_cycles) {
        Ok(result) => {
            if let Some(ref output) = result.output {
                info!(
                    cycles = result.cycles,
                    pc = format!("0x{:08x}", result.final_pc),
                    output_len = output.len(),
                    "Halted with output"
                );
            } else {
                info!(
                    cycles = result.cycles,
                    pc = format!("0x{:08x}", result.final_pc),
                    "Halted (no output)"
                );
            }
        }
        Err(e) => {
            error!("{e}");
            std::process::exit(1);
        }
    }
}
