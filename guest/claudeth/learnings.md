## Do
- Do dispatch precompiles before the empty-code fast path; precompile addresses have no code but still must execute precompile logic.
- Do parse precompile calldata with zero-padding semantics (`buffer_read` behavior), especially for short inputs.
- Do treat precompile out-of-gas and malformed precompile inputs as failed sub-calls (caller continues), not caller-level exceptional halts.
- Do keep precompile value transfer atomic with success: apply transfer only when the precompile call succeeds.
- Do validate ALT_BN128 field elements against the BN254 field modulus (`0x...d87cfd47`), not curve order.
- Do treat `(0, 0)` as point at infinity for ALT_BN128 G1 and encode infinity outputs as exactly 64 zero bytes.
- Do charge fixed gas for `ECADD`/`ECMUL` (`150`/`6000`) before expensive arithmetic and fail fast when insufficient.
- Do anchor BN254 tests to canonical EIP-196 vectors (`G1`, `[2]G1`, `P1+Q1=R1`) to detect subtle curve/modulus regressions.

## Don't
- Don't let unknown/missing precompile dispatch silently fall through to regular empty-code CALL behavior.
- Don't parse short MODEXP exponent heads as right-padded fixed 32-byte values; keep variable-length big-endian semantics.
- Don't return raw 20-byte RIPEMD-160 output; left-pad to 32 bytes.
- Don't reuse SHA-256 padding length encoding rules for RIPEMD-160 (different endianness requirement in this implementation).
- Don't start heavy modular arithmetic before confirming gas sufficiency for the precompile path.
- Don't treat malformed ALT_BN128 points as successful calls with empty output; they must fail the sub-call.
- Don't assume passing local self-consistent vectors is enough; cross-check with execution-spec/EIP vectors.
