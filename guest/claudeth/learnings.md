## Do
- Do resolve blockchain fixture parents by `block_header.parent_hash` against a hash-indexed executed-header map; multi-chain fixtures are not linear.
- Do build `BLOCKHASH` input as a bounded ancestry window ordered by increasing block number (oldest -> newest), with direct parent last.
- Do keep invalid blocks from mutating canonical execution context (state/header index); they are expected failures, not chain progression.
- Do add fixture-backed regression tests for branch-switch scenarios (`chainname` A/B) so parent selection regressions are caught immediately.
- Do validate host-level `BLOCKHASH` lookups with explicit window-order tests, not just end-to-end fixture execution.

## Don't
- Don't reuse “previous loop header” as parent in EELS harnesses; that breaks as soon as a fixture includes forks.
- Don't pass empty recent hash arrays to block processing when validating conformance; it guarantees wrong `BLOCKHASH` semantics.
- Don't assume fixture order implies canonical ancestry; treat header hash linkage as the source of truth.
- Don't insert headers from expected-invalid blocks into ancestry indexes; that pollutes later parent/hash resolution.
