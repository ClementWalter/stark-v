//! Auto-generated example for load_merge.
//! Run with: cargo run --example load_merge

fn main() {
    let result = guest_lib::programs::load_merge::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
