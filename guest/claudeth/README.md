# Claudeth

Claudeth is a minimal Ethereum State Transition Function (STF) guest program
written in Rust for the stark-v zkVM. It targets `no_std` and
`riscv32im-unknown-none-elf`.

Core features:

- **no dependencies** on external crates, any;
- **fully compatible** with the Ethereum Execution Layer Specification (EELS):
  passes all tests from
  [execution-spec-tests](https://github.com/ethereum/execution-spec-tests);
- **optimal for RISC-V rv32im** architecture in terms of number of cycles (see
  [benchmarks](benchmarks/README.md));
- includes a **Partial MPT** implementation with inclusion/exclusion proof
  verification for state transitions proofs.

## Getting started

Though `claudeth` is written for RV32im, it can be compiled and run on native
architectures for fast development. As such, `no_std` is a discipline, but not
enforced.

```sh
cargo test --release
```

Running the tests against the RV32im target is slower as it requires to use the
[RISC-V runner](../../crates/runner/).

**All tests are run in both native and RV32im targets.**

## Benchmarks

We benchmarked `claudeth` against other reference implementations used in
[ethproofs.org](https://ethproofs.org/) focusing on the number of cycles used.
In all our benchmarks, `claudeth` consumes less cycles than the reference
implementations.

## References

- [Ethereum Execution Layer Specification](https://github.com/ethereum/execution-specs)
- [Ethereum Execution Layer Tests](https://github.com/ethereum/execution-spec-tests)
- [revm](https://github.com/bluealloy/revm)
- [ethproofs.org](https://ethproofs.org/learn)
