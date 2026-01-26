//! Build script to auto-generate individual e2e test functions for each guest program.

use std::fs;
use std::path::Path;

fn main() {
    let examples_dir = Path::new("../../guest/guest-lib/examples");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("e2e_tests.rs");
    let revm_enabled = std::env::var("CARGO_FEATURE_REVM").is_ok();

    // Tell Cargo to rerun if examples directory changes
    println!("cargo:rerun-if-changed=../../guest/guest-lib/examples");

    // Collect example names
    let mut examples = Vec::new();

    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "rs") {
                let file_name = path.file_stem().unwrap().to_str().unwrap();
                if file_name == "revm_smoke" && !revm_enabled {
                    continue;
                }
                examples.push(file_name.to_string());
            }
        }
    }

    // Sort for consistent output
    examples.sort();

    // Generate individual test functions
    let mut tests = String::new();
    for name in &examples {
        if name == "revm_smoke" {
            tests.push_str(
                r#"
#[test]
fn test_revm_smoke() {
    test_example_with_features("revm_smoke", &["revm"]);
}
"#,
            );
        } else {
            tests.push_str(&format!(
                r#"
#[test]
fn test_{name}() {{
    test_example("{name}");
}}
"#
            ));
        }
    }

    fs::write(&dest_path, tests).expect("Failed to write e2e_tests.rs");
}
