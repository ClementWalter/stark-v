## Do
- Do dispatch precompile calls before the empty-code fast path; precompiles have no code but must still execute.
- Do parse precompile inputs with zero-padding semantics to match execution-specs `buffer_read`.
- Do charge precompile gas even when the output is empty due to invalid inputs.
- Do encode SHA-256 padding length in big-endian and RIPEMD-160 padding length in little-endian.
- Do left-pad RIPEMD-160 precompile output to 32 bytes.

## Don't
- Don't treat precompiles as ordinary empty-code accounts; it silently drops required behavior.
- Don't skip balance-transfer checks for precompile calls with nonzero value.
- Don't accept `v` values outside 27/28 for ECRECOVER precompile input.
- Don't reuse SHA-256 padding rules for RIPEMD-160 length encoding.
- Don't return the raw 20-byte RIPEMD-160 digest from the precompile without padding.
