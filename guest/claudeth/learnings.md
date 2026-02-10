## Do
- Do treat every fork-reserved precompile address as a precompile dispatch target, even before full implementation is available.
- Do fail reserved-but-unimplemented precompiles as failed sub-calls so they cannot silently take the empty-code execution path.
- Do assert host-level call semantics for precompile failures: call fails, forwarded gas is consumed, and value transfer does not happen.
- Do dispatch precompiles before empty-code fast paths; precompile accounts intentionally have no runtime bytecode.
- Do preserve EVM precompile failure semantics consistently: malformed input and precompile-level exceptional halts should fail only the sub-call.
- Do parse BLAKE2F exactly per EIP-152 (213-byte input, big-endian rounds, little-endian state/message/counters, strict final flag).
- Do keep ALT_BN128 G1 decoding strict on field bounds and curve membership, with `(0,0)` treated as infinity.

## Don't
- Don't let known addresses like `0x08` and `0x0a` fall through as regular empty-code CALL targets.
- Don't move call value on any failed precompile path.
- Don't report execution-spec compatibility from a harness that still truncates fixtures or skips invalid-block coverage.
- Don't decode MODEXP exponent head as fixed 32-byte right-padded data; keep variable-length big-endian semantics.
- Don't return raw 20-byte RIPEMD-160 output; left-pad to 32 bytes.
- Don't coerce invalid BLAKE2F final-block flags into booleans; reject them as precompile failure.
