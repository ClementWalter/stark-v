# stark-v

Lightweight tooling for compiling RISC-V guests and inspecting their
transpiled `VmExe` metadata.

## Build the sample guest

The CLI ships with a single `build` subcommand. It shells out to `cargo` using
the `risc0` toolchain to compile any RV32IM guest crate you point it at.

```bash
cargo run -- build --guest-path guests/playground
```

That command builds the `guests/playground` package and prints a short summary
containing the ELF path plus the number of transpiled instructions and
initialized bytes. Use `--guest-path` to target other guest crates in your
workspace.
