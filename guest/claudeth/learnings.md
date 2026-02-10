## Do
- Do keep README claims limited to behaviors proven by runnable checks in this repository.
- Do run `cargo test -p claudeth --release` before updating project status.
- Do keep precompile error handling as failed sub-calls that consume forwarded gas and preserve caller-frame execution.
- Do enforce strict calldata framing and gas rules for precompiles before adding arithmetic internals.
- Do treat EELS harness shortcuts as temporary diagnostics, not compatibility evidence.
- Do scope each milestone to one spec gap at a time (pairing arithmetic, then point evaluation), because both depend on easy-to-miss cryptographic edge cases.

## Don't
- Don't claim full execution-spec compatibility while `tests/eels_blockchain_tests.rs` still skips coverage and assertions.
- Don't treat reserved precompile addresses as equivalent to empty-code accounts.
- Don't transfer value on any failed precompile path.
- Don't mix fork-specific precompile behavior without explicit fork gating.
- Don't start `POINT_EVALUATION` implementation without a concrete KZG verification strategy aligned with execution-spec semantics.
