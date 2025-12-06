# stark-v

Utilities and experiments that sit alongside [`openvm`](../openvm). The `run-elf`
subcommand wires in a lightweight version of OpenVM’s ELF-to-`VmExe` pipeline
without depending on the upstream repository.

## Building and transpiling the playground guest

The `build` subcommand shells out to `cargo` with the `risc0` toolchain to
compile an RV32IM guest package. Pass the path to the package directory (the
default workflow uses `guests/playground`):

```bash
cargo run -- build --guest-path guests/playground
```

This invokes the lightweight builder crate (modeled after `openvm`’s
`crates/toolchain/build` flow) which produces
`guests/playground/target/riscv32im-risc0-zkvm-elf/release/playground` and then
immediately feeds that ELF into the local `run-elf` pipeline to emit a `VmExe`
summary.

You can still point `run-elf` at any pre-built guest ELF:

```bash
cargo run -- run-elf \
  --path guests/playground/target/riscv32im-risc0-zkvm-elf/release/playground
```

Behind the scenes the implementation mirrors OpenVM’s
`crates/toolchain/transpiler` logic (`Elf::decode`, RV32IM transpiler helpers,
and the `VmExe` layout) but is self-contained inside this repository.
