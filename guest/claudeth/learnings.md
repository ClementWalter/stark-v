## Do
- Do dispatch precompile calls before the empty-code fast path; precompiles have no code but must still execute.
- Do parse precompile inputs with zero-padding semantics to match execution-specs `buffer_read`.
- Do charge precompile gas even when the output is empty due to invalid inputs.

## Don't
- Don't treat precompiles as ordinary empty-code accounts; it silently drops required behavior.
- Don't skip balance-transfer checks for precompile calls with nonzero value.
- Don't accept `v` values outside 27/28 for ECRECOVER precompile input.
