## Do
- Do dispatch precompiles before the empty-code fast path; precompile addresses have no bytecode but must still execute precompile logic.
- Do preserve EELS-style precompile failure semantics: malformed inputs and OOG must fail the sub-call (caller continues), not halt the caller frame.
- Do keep precompile value transfer atomic with success: move value only on successful precompile execution.
- Do decode ALT_BN128 inputs with zero-padding semantics and strict field/curve validation; `(0, 0)` is infinity and must encode back to exactly 64 zero bytes.
- Do anchor ALT_BN128 arithmetic tests to canonical EIP-196 vectors (`P1`, `Q1`, `[2]G1`) to detect subtle arithmetic regressions.
- Do parse BLAKE2F input exactly as EIP-152 defines: 213 bytes total, rounds as big-endian `u32`, and `h/m/t` words as little-endian `u64`.
- Do validate BLAKE2F final-block flag strictly (`0` or `1`) and verify outputs against official execution-spec vectors (`rounds=0`, `rounds=12`).

## Don't
- Don't allow unknown/missing precompile dispatch to fall through silently to regular empty-code CALL behavior.
- Don't parse MODEXP exponent head as a fixed right-padded 32-byte integer; keep variable-length big-endian semantics.
- Don't return raw 20-byte RIPEMD-160 output; left-pad to 32 bytes.
- Don't start expensive arithmetic paths before checking fixed precompile gas requirements.
- Don't treat malformed ALT_BN128 points as successful calls with empty output; they must fail the sub-call.
- Don't decode BLAKE2F `h/m/t` words with big-endian byte order; this breaks all canonical vectors.
- Don't coerce invalid BLAKE2F flag values (`f > 1`) into boolean values; reject them as precompile failure.
