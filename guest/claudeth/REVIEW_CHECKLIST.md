# Code Review Checklist for secp256k1 Implementation

## Automated Checks (Must Pass)
- [ ] `cargo test --manifest-path guest/claudeth/Cargo.toml --release` - ALL PASS
- [ ] `cargo clippy --manifest-path guest/claudeth/Cargo.toml --tests -- -D warnings` - ZERO WARNINGS
- [ ] `cargo check --manifest-path guest/claudeth/Cargo.toml` - SUCCESS

## Code Structure Review

### File Organization
- [ ] secp256k1.rs exists in src/crypto/
- [ ] Module exported in src/crypto/mod.rs
- [ ] Proper module documentation at top of file
- [ ] Follows same structure as keccak.rs

### no_std Compatibility
- [ ] Has `#[cfg(target_arch = "riscv32")] extern crate alloc;` if needed
- [ ] Uses proper imports (core:: instead of std:: where applicable)
- [ ] No accidental std dependencies
- [ ] Works with no_std configuration

### Dependencies
- [ ] k256 added to Cargo.toml with correct version
- [ ] k256 configured with default-features = false
- [ ] k256 has "ecdsa" feature enabled
- [ ] No unnecessary dependencies added

## Function Implementation Review

### verify_signature()
- [ ] Function signature matches spec: `verify_signature(message_hash: &Hash, signature: &[u8], public_key: &[u8]) -> Result<bool>`
- [ ] Properly validates signature length (64 or 65 bytes)
- [ ] Properly validates public_key length (33 or 65 bytes compressed/uncompressed)
- [ ] Returns Result with proper error types
- [ ] Handles edge cases (empty signature, invalid data)
- [ ] Has doc comment with example
- [ ] Has doc test

### recover_public_key()
- [ ] Function signature matches spec: `recover_public_key(message_hash: &Hash, signature: &[u8], recovery_id: u8) -> Result<[u8; 64]>`
- [ ] Validates recovery_id range (0-3)
- [ ] Validates signature length (64 bytes for r,s)
- [ ] Returns uncompressed public key (64 bytes, without 0x04 prefix)
- [ ] Handles invalid recovery_id gracefully
- [ ] Has doc comment with example
- [ ] Has doc test

### recover_address()
- [ ] Function signature matches spec: `recover_address(message_hash: &Hash, signature: &[u8], recovery_id: u8) -> Result<Address>`
- [ ] Uses recover_public_key() internally
- [ ] Computes address correctly: keccak256(public_key)[12..32]
- [ ] Returns proper Address type
- [ ] Handles all error cases
- [ ] Has doc comment with example
- [ ] Has doc test

## Test Coverage Review

### Minimum Test Count
- [ ] At least 15 unit tests in #[cfg(test)] mod tests
- [ ] At least 3 doc tests (one per main function)
- [ ] Tests cover all public functions
- [ ] Tests cover all error paths

### Test Quality
- [ ] Uses real Ethereum transaction test vectors (not synthetic)
- [ ] Tests empty signature handling
- [ ] Tests invalid signature handling (wrong length, invalid data)
- [ ] Tests valid signatures with known public keys
- [ ] Tests public key recovery with known transactions
- [ ] Tests address recovery with known transactions
- [ ] Tests all recovery_id values (0, 1, 2, 3)
- [ ] Tests edge cases (boundary values, malformed input)
- [ ] Tests determinism (same input = same output)
- [ ] Tests that different inputs produce different outputs

### Test Data
- [ ] Test vectors are real Ethereum data (can verify on Etherscan)
- [ ] Test vectors include transaction hash, signature (r,s,v), expected address
- [ ] Test vectors are well-documented (block number, tx hash, etc.)
- [ ] Test vectors cover different transaction types (legacy, EIP-155, EIP-2930, EIP-1559)

## Integration Review

### Type Compatibility
- [ ] Uses crate::types::Hash correctly
- [ ] Uses crate::types::Address correctly
- [ ] Types work seamlessly with rest of codebase
- [ ] No unnecessary type conversions

### Error Handling
- [ ] Defines proper error types (or uses existing)
- [ ] Errors are descriptive and helpful
- [ ] All Result types properly propagate errors
- [ ] No unwrap() or expect() in production code (tests OK)

### API Design
- [ ] Function names follow Rust conventions (snake_case)
- [ ] Parameters are in logical order
- [ ] Return types are clear and consistent
- [ ] API is similar to keccak256() for consistency

## Documentation Review

### Module Documentation
- [ ] Module has top-level doc comment explaining purpose
- [ ] Links to relevant Ethereum specifications
- [ ] Explains signature format (65 bytes: r, s, v)
- [ ] Explains recovery_id meaning

### Function Documentation
- [ ] Each public function has doc comment
- [ ] Doc comments explain what function does
- [ ] Doc comments explain parameters
- [ ] Doc comments explain return values
- [ ] Doc comments explain error conditions
- [ ] Doc comments include usage examples
- [ ] Examples are tested (doc tests)

## Performance Considerations
- [ ] No unnecessary allocations
- [ ] Efficient use of k256 crate
- [ ] No obvious performance issues
- [ ] Code is reasonably optimized (not premature optimization)

## Security Considerations
- [ ] No unsafe code (unless necessary and documented)
- [ ] Proper validation of all inputs
- [ ] No potential for panics on invalid input
- [ ] No side-channel vulnerabilities (constant-time operations where needed)
- [ ] Uses battle-tested k256 crate correctly

## Code Quality
- [ ] Code is readable and well-organized
- [ ] Variable names are clear and descriptive
- [ ] No commented-out code
- [ ] No TODO or FIXME comments
- [ ] Follows Rust idioms and best practices
- [ ] Consistent formatting (rustfmt)

## Final Validation Commands

```bash
# Run from stark-v root

# 1. Tests
cargo test --manifest-path guest/claudeth/Cargo.toml --release

# 2. Clippy
cargo clippy --manifest-path guest/claudeth/Cargo.toml --tests -- -D warnings

# 3. Check
cargo check --manifest-path guest/claudeth/Cargo.toml

# 4. Doc tests specifically
cargo test --manifest-path guest/claudeth/Cargo.toml --doc

# 5. Specific module tests
cargo test --manifest-path guest/claudeth/Cargo.toml --release secp256k1

# 6. Format check
cargo fmt --manifest-path guest/claudeth/Cargo.toml -- --check
```

## Pass/Fail Criteria

### MUST PASS (Non-negotiable)
- All automated checks pass (tests, clippy, compilation)
- At least 15 unit tests
- 100% of public functions have tests
- Zero clippy warnings
- Zero compilation errors
- Works with real Ethereum test vectors

### SHOULD PASS (Fix if reasonable)
- All doc tests present
- Comprehensive documentation
- Good test coverage of error paths
- Clean, readable code

### NICE TO HAVE (Not blocking)
- Performance optimizations
- Extra test vectors
- Detailed inline comments

## Review Outcome

After completing this checklist:
- **APPROVE**: All MUST PASS criteria met → Mark task complete, proceed to integration tests
- **REQUEST CHANGES**: Some MUST PASS criteria failed → Provide specific feedback, request fixes
- **MAJOR REVISION**: Many criteria failed → Consider reassigning or breaking down task further
