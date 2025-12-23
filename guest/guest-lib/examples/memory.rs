//! Auto-generated example for memory.
//! Run with: cargo run --example memory

fn main() {
    let result = guest_lib::programs::memory::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
