//! Build script to auto-generate binary wrappers for guest-lib programs.
//!
//! This scans the guest-lib/examples/ directory and generates a binary
//! wrapper for each example.

use std::fs;
use std::path::Path;

fn main() {
    let examples_dir = Path::new("../guest-lib/examples");
    let bin_dir = Path::new("src/bin");

    // Ensure bin directory exists
    fs::create_dir_all(bin_dir).expect("Failed to create src/bin directory");

    // Tell Cargo to rerun if examples directory changes
    println!("cargo:rerun-if-changed=../guest-lib/examples");

    // Scan for example files
    let entries = match fs::read_dir(examples_dir) {
        Ok(entries) => entries,
        Err(_) => {
            // Examples may not exist yet on first build
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip non-rust files
        if path.extension().is_some_and(|ext| ext == "rs") {
            let file_name = path.file_stem().unwrap().to_str().unwrap();

            // Generate binary wrapper that calls test_call from the programs module
            let bin_content = format!(
                r#"#![no_std]
#![no_main]

guest_bin::guest_main!(guest_lib::programs::{module}::test_call());
"#,
                module = file_name
            );

            let bin_path = bin_dir.join(format!("{}.rs", file_name));

            // Only write if content changed to avoid unnecessary rebuilds
            let should_write = match fs::read_to_string(&bin_path) {
                Ok(existing) => existing != bin_content,
                Err(_) => true,
            };

            if should_write {
                fs::write(&bin_path, &bin_content)
                    .unwrap_or_else(|e| panic!("Failed to write {}: {}", bin_path.display(), e));
            }
        }
    }
}
