## Do
- Do reserve and dispatch all known precompile addresses even before full arithmetic support is complete.
- Do mirror execution-spec envelope rules first (gas charging, calldata framing, canonical empty-input behavior) to avoid broad semantic drift.
- Do return `U256(1)` for empty-input ALT_BN128 pairing (`0x08`) with base gas `45000`, matching execution-spec behavior.
- Do keep precompile failures as sub-call failures (`success = false`, forwarded gas consumed) instead of halting the parent frame.
- Do assert host-level value transfer behavior separately for successful and failed precompile calls.
- Do keep BLAKE2F strict on length (`213` bytes) and final flag (`0` or `1`) to match EIP-152 exceptional-halt behavior.
- Do left-pad RIPEMD160 output to 32 bytes; raw 20-byte returns are incorrect for EVM precompile output.

## Don't
- Don't treat active precompile addresses as ordinary empty-code accounts.
- Don't return success for malformed precompile calldata lengths.
- Don't transfer value on any failed precompile path.
- Don't assume README compatibility claims are true unless current automated checks enforce them.
- Don't keep EELS harness shortcuts (`InvalidBlocks` skips, `.take(10)`, ignored full run) when claiming execution-spec parity.
- Don't decode MODEXP exponent head as fixed-width right-padded data; preserve variable-length big-endian semantics.
