//! Build script to auto-generate:
//! 1. The test dispatcher for e2e tests
//! 2. Cargo examples with main() that can run on host
//! 3. I/O memory layout constants from linker.ld

use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    generate_io_constants();
    generate_dispatcher_and_examples();
}

/// Parse linker.ld and generate src/io.rs with memory layout constants.
fn generate_io_constants() {
    let linker_path = Path::new("../guest-bin/linker.ld");
    let io_path = Path::new("src/io.rs");

    println!("cargo:rerun-if-changed=../guest-bin/linker.ld");

    let linker_content = match fs::read_to_string(linker_path) {
        Ok(content) => content,
        Err(_) => {
            // linker.ld may not exist yet, skip generation
            return;
        }
    };

    // Parse symbol definitions from linker script
    let symbols = parse_linker_symbols(&linker_content);

    // Generate io.rs content
    let io_content = generate_io_rs(&symbols);

    // Only write if content changed
    let should_write = match fs::read_to_string(io_path) {
        Ok(existing) => existing != io_content,
        Err(_) => true,
    };

    if should_write {
        fs::write(io_path, &io_content).expect("Failed to write src/io.rs");
    }
}

/// Parse linker script symbol definitions like `__name = value;` or `__name = expr;`
fn parse_linker_symbols(content: &str) -> HashMap<String, u32> {
    let mut symbols: HashMap<String, u32> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines, comments, and non-assignment lines
        if line.is_empty() || !line.contains('=') {
            continue;
        }

        // Remove inline comments
        let line = line.split("/*").next().unwrap_or(line).trim();

        // Parse: __name = expr;
        if let Some((name, expr)) = line.split_once('=') {
            let name = name.trim().trim_start_matches('_');
            let expr = expr.trim().trim_end_matches(';').trim();

            // Skip non-symbol definitions (like ORIGIN, LENGTH in MEMORY block)
            if name.is_empty() || expr.contains(':') || expr.contains('(') {
                continue;
            }

            if let Some(value) = eval_expr(expr, &symbols) {
                symbols.insert(name.to_string(), value);
            }
        }
    }

    symbols
}

/// Evaluate simple linker expressions: hex literals, symbols, +, -
fn eval_expr(expr: &str, symbols: &HashMap<String, u32>) -> Option<u32> {
    let expr = expr.trim();

    // Try hex literal: 0x...
    if let Some(hex) = expr.strip_prefix("0x") {
        return u32::from_str_radix(hex, 16).ok();
    }

    // Try decimal literal
    if let Ok(val) = expr.parse::<u32>() {
        return Some(val);
    }

    // Try addition: a + b
    if let Some((left, right)) = expr.split_once('+') {
        let left = eval_expr(left.trim(), symbols)?;
        let right = eval_expr(right.trim(), symbols)?;
        return Some(left.wrapping_add(right));
    }

    // Try subtraction: a - b
    if let Some((left, right)) = expr.split_once('-') {
        let left = eval_expr(left.trim(), symbols)?;
        let right = eval_expr(right.trim(), symbols)?;
        return Some(left.wrapping_sub(right));
    }

    // Try symbol reference (strip leading underscores)
    let name = expr.trim_start_matches('_');
    symbols.get(name).copied()
}

/// Generate the io.rs file content from parsed symbols.
fn generate_io_rs(symbols: &HashMap<String, u32>) -> String {
    let get = |name: &str| symbols.get(name).copied().unwrap_or(0);

    let input_start = get("input_start");
    let input_end = get("input_end");
    let input_size = input_end.saturating_sub(input_start);
    let halt_flag = get("halt_flag");
    let output_len = get("output_len");
    let output_data = get("output_data");
    let output_end = get("output_end");
    let output_max_size = output_end.saturating_sub(output_data);
    let stack_top = get("stack_top");
    let stack_size = get("stack_size");
    let stack_bottom = get("stack_bottom");

    format!(
        r#"//! I/O memory layout constants - AUTO-GENERATED from guest-bin/linker.ld
//!
//! Do not edit manually. Run `cargo build -p guest-lib` to regenerate.

/// Start address of the input buffer.
pub const INPUT_START: u32 = {input_start:#010x};

/// End address of the input buffer (exclusive).
pub const INPUT_END: u32 = {input_end:#010x};

/// Input buffer size in bytes.
pub const INPUT_SIZE: usize = {input_size:#x};

/// Address of the halt flag (set to non-zero to halt execution).
pub const HALT_FLAG: u32 = {halt_flag:#010x};

/// Address of the output length word.
pub const OUTPUT_LEN: u32 = {output_len:#010x};

/// Start address of the output data buffer.
pub const OUTPUT_DATA: u32 = {output_data:#010x};

/// End address of the output data buffer (exclusive).
pub const OUTPUT_END: u32 = {output_end:#010x};

/// Maximum output size in bytes.
pub const OUTPUT_MAX_SIZE: usize = {output_max_size:#x};

/// Stack top address.
pub const STACK_TOP: u32 = {stack_top:#010x};

/// Stack size in bytes.
pub const STACK_SIZE: usize = {stack_size:#x};

/// Stack bottom address.
pub const STACK_BOTTOM: u32 = {stack_bottom:#010x};

/// Read input bytes from the input buffer.
///
/// # Safety
/// This function reads from raw memory addresses. Only call from within
/// a zkVM guest program with the correct memory layout.
#[cfg(target_arch = "riscv32")]
pub unsafe fn read_input_bytes(buf: &mut [u8]) -> usize {{
    unsafe {{
        let len = buf.len().min(INPUT_SIZE);
        for (i, byte) in buf.iter_mut().take(len).enumerate() {{
            let addr = INPUT_START + i as u32;
            *byte = core::ptr::read_volatile(addr as *const u8);
        }}
        len
    }}
}}

/// Write output bytes to the output buffer and set the length.
///
/// # Safety
/// This function writes to raw memory addresses. Only call from within
/// a zkVM guest program with the correct memory layout.
#[cfg(target_arch = "riscv32")]
pub unsafe fn write_output_bytes(data: &[u8]) {{
    unsafe {{
        let len = data.len().min(OUTPUT_MAX_SIZE);
        // Write length
        core::ptr::write_volatile(OUTPUT_LEN as *mut u32, len as u32);
        // Write data
        for (i, byte) in data.iter().take(len).enumerate() {{
            let addr = OUTPUT_DATA + i as u32;
            core::ptr::write_volatile(addr as *mut u8, *byte);
        }}
    }}
}}

/// Signal halt to the zkVM runtime.
///
/// # Safety
/// This function writes to raw memory addresses. Only call from within
/// a zkVM guest program with the correct memory layout.
#[cfg(target_arch = "riscv32")]
pub unsafe fn halt() {{
    unsafe {{
        core::ptr::write_volatile(HALT_FLAG as *mut u32, 1);
    }}
}}
"#
    )
}

fn generate_dispatcher_and_examples() {
    let programs_dir = Path::new("src/programs");
    let examples_dir = Path::new("examples");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dispatcher_path = Path::new(&out_dir).join("dispatcher.rs");

    // Tell Cargo to rerun if programs directory changes
    println!("cargo:rerun-if-changed=src/programs");

    // Ensure examples directory exists
    fs::create_dir_all(examples_dir).expect("Failed to create examples directory");

    // Collect program names
    let mut programs = Vec::new();

    let entries = fs::read_dir(programs_dir).expect("Failed to read programs directory");
    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "rs") {
            let file_name = path.file_stem().unwrap().to_str().unwrap();

            // Skip mod.rs
            if file_name == "mod" {
                continue;
            }

            programs.push(file_name.to_string());
        }
    }

    // Sort for consistent output
    programs.sort();

    // Generate dispatcher code
    let mut match_arms = String::new();
    for name in &programs {
        match_arms.push_str(&format!(
            r#"        "{name}" => postcard::to_allocvec(&crate::programs::{name}::test_call()).ok()?,
"#
        ));
    }

    let dispatcher = format!(
        r#"/// Get the serialized test output for a given program name.
/// This is auto-generated by build.rs from the programs directory.
pub fn get_test_bytes(name: &str) -> Option<Vec<u8>> {{
    let bytes = match name {{
{match_arms}        _ => return None,
    }};
    Some(bytes)
}}

/// Returns a list of all available program names.
/// This is auto-generated by build.rs from the programs directory.
pub fn list_programs() -> &'static [&'static str] {{
    &[{program_list}]
}}
"#,
        match_arms = match_arms,
        program_list = programs
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );

    fs::write(&dispatcher_path, dispatcher).expect("Failed to write dispatcher.rs");

    // Generate Cargo examples with main()
    for name in &programs {
        let example_content = format!(
            r#"//! Auto-generated example for {name}.
//! Run with: cargo run --example {name}

fn main() {{
    let result = guest_lib::programs::{name}::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}}
"#,
        );

        let example_path = examples_dir.join(format!("{name}.rs"));

        // Only write if content changed to avoid unnecessary rebuilds
        let should_write = match fs::read_to_string(&example_path) {
            Ok(existing) => existing != example_content,
            Err(_) => true,
        };

        if should_write {
            fs::write(&example_path, &example_content)
                .unwrap_or_else(|e| panic!("Failed to write {}: {}", example_path.display(), e));
        }
    }
}
