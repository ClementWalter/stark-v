use std::path::PathBuf;

use clap::{Parser, Subcommand};
use eyre::Result;

#[derive(Parser)]
#[command(
    name = "stark-v",
    version,
    about = "CLI utilities for the stark-v workspace"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a guest package for the risc0 RISC-V target and emit VmExe summary.
    Build {
        /// Path to the guest package directory containing Cargo.toml.
        #[arg(long)]
        guest_path: PathBuf,
    },
    /// Decode a RISC-V ELF and emit a VmExe summary.
    RunElf {
        /// Path to a compiled RISC-V guest ELF (RV32IM).
        #[arg(long)]
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { guest_path } => {
            let build = builder::build_guest(&guest_path)?;
            println!("Guest built at {}", build.elf_path.display());
            let exe = runner::load_vm_exe_from_elf(&build.elf_path)?;
            println!(
                "VmExe ready: {} instructions, pc_start=0x{pc:08x}, init_bytes={}",
                exe.program.len(),
                exe.init_memory.len(),
                pc = exe.pc_start
            );
        }
        Commands::RunElf { path } => {
            let exe = runner::load_vm_exe_from_elf(&path)?;
            println!(
                "VmExe ready: {} instructions, pc_start=0x{pc:08x}, init_bytes={}",
                exe.program.len(),
                exe.init_memory.len(),
                pc = exe.pc_start
            );
        }
    }
    Ok(())
}
