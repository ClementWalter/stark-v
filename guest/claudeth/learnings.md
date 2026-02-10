## Do
- Do pin consensus constants with literal-vector tests, not only by comparing computed values to internal constants.
- Do treat Ethereum empty trie root as `0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421` and fail fast on any drift.
- Do validate withdrawals-root computation against real execution-spec fixture vectors (for example `bcExample/shanghaiExample.json`) to catch RLP/key-index encoding mistakes.
- Do run `cargo test -p claudeth --release` and `prek run --all-files` before committing to keep protocol and lint gates aligned.
- Do keep post-merge header checks strict: `difficulty == 0`, `nonce == 0`, and empty ommers hash, while allowing non-zero `mix_hash` (`prev_randao`).

## Don't
- Don't trust "empty trie" logic unless it is anchored to canonical spec bytes.
- Don't consider EELS compatibility claims reliable while parent-hash rewrites exist in the blockchain harness.
- Don't leave blob transaction fixture conversion limited to `0x00/0x01/0x02`; Cancun coverage requires `0x03` support.
- Don't pass empty block-hash history into blockchain fixture execution when tests depend on `BLOCKHASH` semantics.
- Don't treat unimplemented `0x0a` point-evaluation as acceptable for Cancun conformance paths.
