# Claudeth

Claudeth is a dependency free guest program that implements the Ethereum State
Transition Function (STF). It is written in Rust is used to generate proofs of
ethereum mainnet blocks. It compiles in no_std for the riscv32 target.

It also embeds a Partial MPT for deriving the minimal state required from the
state root to apply the STF at any given block. This Partial MPT lib needs also
to be able to update the state root with the new state after the STF is applied.

Claudeth starts from fusaka and doesn't implement previous forks. Claudeth is
fully compliant with the
[Ethereum Execution Layer Specification (EELS)](https://github.com/ethereum/execution-specs/)
and as such, pass 100% of the
[EELS test vector](https://github.com/ethereum/execution-spec-tests).

Claudeth is minimal and more performant that
[revm](https://github.com/bluealloy/revm) and
[levm](https://github.com/lambdaclass/ethrex). This is not a claim, see
benchmarks.
