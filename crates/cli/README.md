# stark-v-cli

Command-line interface for the stark-v workspace.

## Overview

The `stark-v-cli` crate provides the `stark-v` binary, a command-line tool for
building and proving RISC-V guest programs.

## Installation

```bash
cargo install --path crates/cli
```

Or run directly from the workspace:

```bash
cargo run -- <command>
```

## Commands

### build

Build a guest package for the risc0 RISC-V target and emit a `VmExe` summary.

```bash
stark-v build --guest-path <PATH>
```

**Arguments:**

- `--guest-path <PATH>` - Path to the guest package directory containing
  `Cargo.toml`

**Example:**

```bash
stark-v build --guest-path guests/playground
```
