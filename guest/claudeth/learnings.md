## Do
- Do dispatch precompile calls before the empty-code fast path; precompiles have no code but must still execute.
- Do parse precompile inputs with zero-padding semantics to match execution-specs `buffer_read`.
- Do treat precompile out-of-gas as a failed sub-call that consumes forwarded gas, not as a caller-level exceptional halt.
- Do keep failed precompile calls value-neutral; value transfer must not persist when the sub-call fails.
- Do encode SHA-256 padding length in big-endian and RIPEMD-160 padding length in little-endian.
- Do left-pad RIPEMD-160 precompile output to 32 bytes.
- Do compute MODEXP gas from EIP-2565 (`complexity * iterations / 3`, minimum 200) before running the exponentiation.
- Do validate ALT_BN128 G1 coordinates against the BN254 **field modulus** (`0x...d87cfd47`) rather than the BN254 curve order (`0x...f0000001`).
- Do treat `(0, 0)` as the point at infinity for ALT_BN128 precompiles and encode infinity output as 64 zero bytes.
- Do anchor BN254 precompile tests to canonical EIP-196 vectors (for example `[2]G1` and `P1 + Q1 = R1`) to catch modulus/curve mismatches early.

## Don't
- Don't treat precompiles as ordinary empty-code accounts; it silently drops required behavior.
- Don't skip balance-transfer checks for precompile calls with nonzero value.
- Don't accept `v` values outside 27/28 for ECRECOVER precompile input.
- Don't reuse SHA-256 padding rules for RIPEMD-160 length encoding.
- Don't return the raw 20-byte RIPEMD-160 digest from the precompile without padding.
- Don't parse short MODEXP exponent heads as right-padded 32-byte values; they must be interpreted as variable-length big-endian integers.
- Don't start expensive modular arithmetic before confirming the call has enough gas for MODEXP.
- Don't treat malformed ALT_BN128 points as a successful precompile call with empty output; they must fail the sub-call.
- Don't rely on self-consistent local ECADD outputs as proof of correctness when reference-vector checks are missing.
