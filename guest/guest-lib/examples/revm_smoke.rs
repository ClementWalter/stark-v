//! Auto-generated example for revm_smoke.
//! Run with: cargo run --example revm_smoke

fn main() {
    let result = guest_lib::programs::revm_smoke::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
