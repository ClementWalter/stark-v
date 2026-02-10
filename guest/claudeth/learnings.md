## Do
- Do dispatch precompile calls before the empty-code fast path; precompiles have no code but must still execute.
- Do parse precompile inputs with zero-padding semantics to match execution-specs `buffer_read`.
- Do charge precompile gas even when the output is empty due to invalid inputs.
- Do encode SHA-256 padding length in big-endian and RIPEMD-160 padding length in little-endian.
- Do left-pad RIPEMD-160 precompile output to 32 bytes.
- Do compute MODEXP gas from EIP-2565 (`complexity * iterations / 3`, minimum 200) before running the exponentiation.
- Do validate ALT_BN128 (`ECADD`) coordinates against the BN254 field modulus before curve checks.
- Do treat `(0, 0)` as the point at infinity for ALT_BN128 precompiles.
- Do output `64` zero bytes when ALT_BN128 addition result is infinity.
- Do treat precompile out-of-gas as a failed sub-call that consumes forwarded gas, not as a caller-level exceptional halt.
- Do keep failed precompile calls value-neutral; value transfer must not persist when the sub-call fails.

## Don't
- Don't treat precompiles as ordinary empty-code accounts; it silently drops required behavior.
- Don't skip balance-transfer checks for precompile calls with nonzero value.
- Don't accept `v` values outside 27/28 for ECRECOVER precompile input.
- Don't reuse SHA-256 padding rules for RIPEMD-160 length encoding.
- Don't return the raw 20-byte RIPEMD-160 digest from the precompile without padding.
- Don't parse short MODEXP exponent heads as right-padded 32-byte values; they must be interpreted as variable-length big-endian integers.
- Don't start expensive modular arithmetic before confirming the call has enough gas for MODEXP.
- Don't treat malformed ALT_BN128 points as a successful precompile call with empty output; they must fail the sub-call.
- Don't require full 128-byte calldata for `ECADD`; missing bytes are right-padded with zeros per spec input reads.
