//! Auto-generated example for heap_vec.
//! Run with: cargo run --example heap_vec

fn main() {
    let result = guest_lib::programs::heap_vec::test_call();
    let bytes = postcard::to_allocvec(&result).unwrap();

    // Write raw bytes to stdout for piping/testing
    use std::io::Write;
    std::io::stdout().write_all(&bytes).unwrap();
}
