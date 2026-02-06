# Security Documentation

**Version**: 0.1.0 **Date**: 2026-02-06 **Status**: Pre-production (Not audited)

---

## Overview

stark-v is an RV32IM zkVM built on Stwo that generates STARK proofs for RISC-V
program execution. This document outlines the security model, threat analysis,
cryptographic assumptions, known limitations, and responsible disclosure
procedures.

**⚠️ WARNING**: stark-v is a work in progress and **NOT yet ready for production
use**. This system has not undergone a formal security audit. Use in production
environments is strongly discouraged until critical security gaps are addressed.

---

## Table of Contents

1. [Threat Model](#threat-model)
2. [Security Assumptions](#security-assumptions)
3. [Known Limitations](#known-limitations)
4. [Memory Safety](#memory-safety)
5. [Clock Gap-Filling](#clock-gap-filling)
6. [Cryptographic Components](#cryptographic-components)
7. [Audit Status](#audit-status)
8. [Responsible Disclosure](#responsible-disclosure)

---

## Threat Model

### In Scope

stark-v aims to provide security against the following threats:

#### 1. **Malicious Prover**

A prover attempting to generate fraudulent proofs that would pass verification
for:

- Execution traces that do not match the actual program execution
- Invalid state transitions (incorrect ALU operations, memory accesses, control
  flow)
- Fabricated public inputs or outputs
- Constraint violations in the AIR (Algebraic Intermediate Representation)

**Defense**: The STARK proof system ensures soundness — a malicious prover
cannot convince a verifier of a false statement except with negligible
probability.

#### 2. **Malicious Guest Program**

A guest program attempting to:

- Access memory outside its allocated regions
- Forge register or memory state
- Manipulate the execution trace to hide malicious behavior
- Overflow stack or heap memory regions
- Exploit undefined behavior in RISC-V instructions

**Defense**: All memory accesses and register operations are fully constrained
via LogUp lookup relations. The AIR enforces correctness of all 45 RV32IM
instructions.

#### 3. **Constraint Bypass**

An attacker attempting to:

- Find execution traces that satisfy constraints but represent invalid
  computation
- Exploit weaknesses in range checks or lookup tables
- Violate continuity constraints (memory/register state transitions)
- Manipulate clock values to bypass ordering constraints

**Defense**: Comprehensive AIR constraints defined in `docs/airs.md` enforce
computational integrity. LogUp relations ensure all memory and register accesses
are consistent.

### Out of Scope

The following threats are **NOT** addressed by stark-v:

#### 1. **Side-Channel Attacks**

- Timing attacks on the prover or verifier
- Power analysis or electromagnetic emanation attacks
- Cache-based attacks during proof generation

**Rationale**: stark-v focuses on computational integrity, not privacy.
Side-channel resistance is the responsibility of the deployment environment.

#### 2. **Denial of Service (DoS)**

- Resource exhaustion through large programs or excessive proof generation
- Malicious guest programs with infinite loops or extreme memory usage

**Rationale**: The runner enforces a maximum cycle limit (`max_cycles`
parameter), but DoS mitigation is deployment-specific.

#### 3. **Prover Privacy**

- Leakage of private inputs through the proof
- Information disclosure via proof size or generation time

**Rationale**: STARK proofs are zero-knowledge in the cryptographic sense, but
stark-v does not currently implement zero-knowledge protocols. All execution is
public by default.

#### 4. **Verifier Bugs**

- Bugs in the Stwo verifier implementation
- Incorrect verification due to implementation flaws

**Rationale**: stark-v relies on the security of Stwo's verifier. Bugs in Stwo
are out of scope for stark-v itself.

---

## Security Assumptions

### Cryptographic Primitives

stark-v's security relies on the following cryptographic assumptions:

#### 1. **STARK Soundness**

**Assumption**: The Stwo STARK prover is sound, meaning a malicious prover
cannot convince a verifier of a false statement except with negligible
probability (bounded by the security parameter).

**Parameter**: 100 bits of security (default Stwo configuration)

**Cryptographic Basis**: FRI (Fast Reed-Solomon Interactive Oracle Proofs) over
the M31 field with extension to QM31.

#### 2. **Hash Function Security**

**Poseidon2 (Merkle Commitments)**:

- Used for Merkle tree commitments to memory and program state
- Parameters: 16-wide permutation, 8 full rounds, 14 partial rounds
- Security target: 128-bit collision resistance
- Constants: Hardcoded in `crates/prover/src/components/poseidon2.rs`

**Blake2s (Fiat-Shamir)**:

- Used by Stwo for Fiat-Shamir transformation
- Security target: 128-bit collision resistance
- Provided by: Stwo's channel implementation

**Assumption**: No collision attacks, preimage attacks, or second-preimage
attacks exist against these hash functions with complexity below 2^128
operations.

#### 3. **Field Arithmetic**

**M31 Field**:

- Prime: 2^31 - 1
- All computations modulo M31
- PC (program counter) represented as M31 element

**QM31 Extension Field**:

- Used for interaction traces and LogUp relations
- 4-dimensional extension over M31

**Assumption**: Field operations are correctly implemented and no arithmetic
overflow/underflow occurs.

#### 4. **Random Oracle Model**

**Assumption**: Hash functions (Poseidon2, Blake2s) behave as random oracles in
the security proofs of the STARK protocol.

### Completeness

stark-v assumes **perfect completeness**: an honest prover executing a valid
program will always generate a valid proof that passes verification.

**Current Status**: Completeness is validated through extensive testing (177
unit tests, multiple end-to-end integration tests).

### Soundness

stark-v assumes **computational soundness**: a malicious prover cannot generate
a proof of an invalid execution except with negligible probability.

**Current Status**: Soundness relies on:

1. Stwo's STARK implementation
2. Correctness of AIR constraints (17 component families)
3. LogUp lookup relations (memory_access, program_access, bitwise, range checks)

**Critical Gap**: LogUp sum verification is currently **disabled by default**
(see [Known Limitations](#known-limitations)).

---

## Known Limitations

### Critical Security Issues

#### 1. **LogUp Sum Verification Disabled (CRITICAL)**

**Location**: `crates/prover/src/prover.rs:136`

**Issue**: LogUp sum verification is only enabled with the `track-relations`
feature flag, which is **not enabled by default** in production builds.

**Impact**:

- Public inputs and outputs are not fully constrained in production mode
- Memory access ordering may not be properly verified
- A malicious prover could potentially manipulate memory state

**Workaround**: Enable the `track-relations` feature flag:

```bash
cargo build --release --features track-relations
```

**Status**: Marked as P0 (Critical) in `PLAN.md`. Fix scheduled for Phase 1.

**Recommendation**: **Do not use stark-v in production** until this issue is
resolved.

#### 2. **Shift Carry Range Checks Incomplete**

**Locations**:

- `crates/prover/src/components/opcodes/shifts_reg/air.rs:333`
- `crates/prover/src/components/opcodes/shifts_imm/air.rs:284`

**Issue**: Range checks for shift carry values are not fully implemented (marked
with TODO comments).

**Impact**:

- Shift operations (sll, srl, sra, slli, srli, srai) may not be fully
  constrained
- A malicious prover could potentially provide invalid shift results

**Status**: Marked as P1 (High Priority) in `PLAN.md`. Fix scheduled for
Phase 1.

#### 3. **No Formal Security Audit**

**Status**: stark-v has **not undergone a formal security audit** by an external
cryptography firm.

**Impact**:

- AIR constraint completeness and correctness not independently verified
- Potential undiscovered vulnerabilities in constraint logic
- Implementation bugs may exist in the runner or prover

**Recommendation**: Conduct a formal audit before production deployment.

### High Priority Limitations

#### 4. **Missing System Instructions**

**Issue**: RV32I system instructions are not yet implemented:

- `ecall` (environment call)
- `ebreak` (breakpoint)
- `fence`, `fence.i` (memory ordering)
- 6 CSR instructions (`csrrw`, `csrrs`, `csrrc`, `csrrwi`, `csrrsi`, `csrrci`)

**Impact**:

- Cannot execute programs requiring system calls
- Limited OS interaction support
- Reduced compatibility with standard RISC-V toolchains

**Status**: Scheduled for Phase 2 implementation.

#### 5. **No Continuations/Segmentation**

**Issue**: No API exists for splitting large programs across multiple proofs.

**Impact**:

- Maximum program size limited by available memory
- Cannot prove unbounded execution
- Large programs may be impractical to prove

**Status**: Scheduled for Phase 3 implementation.

#### 6. **Error Handling**

**Issue**: Extensive use of `expect()` and `unwrap()` throughout the codebase
(321 occurrences across 109 files).

**Impact**:

- Potential for unexpected panics in production
- Reduced robustness and error recovery
- Difficult to debug failures

**Status**: Cleanup scheduled for Phase 1 (target: reduce by 50%).

### Medium Priority Limitations

#### 7. **No Zero-Knowledge**

**Issue**: stark-v does not implement zero-knowledge protocols.

**Impact**:

- All execution traces are public
- Cannot hide private inputs from verifiers
- Not suitable for privacy-preserving applications

**Status**: Not currently planned. Would require significant Stwo integration
work.

#### 8. **No Proof Aggregation**

**Issue**: No recursive proof composition or aggregation.

**Impact**:

- Cannot combine multiple proofs into a single proof
- Limited scalability for recursive verification scenarios

**Status**: Blocked on Stwo recursion support.

---

## Memory Safety

### Memory Layout

Guest programs use a fixed memory layout defined in `guest/guest-bin/linker.ld`:

```text
Address Range           Region          Size      Access
────────────────────────────────────────────────────────────
0x00000400 - 0x000FFFFF  TEXT (rx)      ~1 MB     Read+Execute
0x00100000 - 0x00100FFF  INPUT          4 KB      Read (prover-supplied)
0x00101000              HALT_FLAG       4 B       Read+Write
0x00101004              OUTPUT_LEN      4 B       Write
0x00101008 - 0x001FFFBF  OUTPUT         ~1 MB     Write
0x001FFFC0 - 0x001FFFFF  STACK          1 KB      Read+Write (grows down)
0x00200000 - 0x002FFFFF  DATA (rw)      1 MB      Read+Write
```

### Stack Overflow Protection

**Issue**: The linker script allocates **only 1 KB** for the stack
(`__stack_size = 0x00000400`).

**Implications**:

- Very small stack, easily exhausted by recursive functions
- No runtime stack overflow detection in the guest environment
- Stack overflow would overwrite the output buffer (silent corruption)

**Mitigations**:

1. **Static Analysis**: Guest programs should be analyzed for stack usage at
   compile time
2. **Conservative Usage**: Avoid deep recursion and large stack allocations
3. **AIR Constraints**: Memory access constraints ensure the prover cannot forge
   memory contents, but do not prevent stack overflow during execution

**Recommendation**: Consider increasing stack size or implementing runtime stack
guards.

### Buffer Overflow Protection

**Issue**: No runtime bounds checking for memory accesses in the guest
environment.

**Mitigations**:

1. **AIR Constraints**: All memory accesses are logged in the execution trace
   and verified via LogUp relations
2. **Rust Safety**: Guest programs written in Rust benefit from compile-time
   bounds checking
3. **Program Validation**: Memory accesses outside defined regions will be
   detected during proof generation (assuming LogUp verification is enabled)

**Limitation**: The runner itself does not enforce memory region boundaries
during execution — invalid accesses are only caught during proof generation.

### Input Buffer Safety

**Input Region**: 4 KB at `0x00100000 - 0x00100FFF`

**Protection**: The runner validates input size before execution:

```rust
let input_capacity = input_end.saturating_sub(input_start) as usize;
if input.len() > input_capacity {
    return Err(RunError::InputTooLarge { len: input.len(), capacity: input_capacity });
}
```

**Implication**: Input buffer overflow is prevented at the runner level.

### Output Buffer Safety

**Output Region**: ~1 MB at `0x00101008 - 0x001FFFBF`

**Protection**: Output length is constrained by the available region size.
Overflowing the output buffer would overwrite the stack (located immediately
after).

**Recommendation**: Guest programs should implement explicit bounds checking for
output writes.

---

## Clock Gap-Filling

### Mechanism

stark-v uses a **clock-based ordering** mechanism to enforce the temporal
sequence of register and memory accesses. Each access records:

- `clk`: Current clock value (monotonically increasing)
- `clk_prev`: Clock value of the previous access to the same address
- `prev`: Value before access
- `next`: Value after access

### Maximum Clock Difference

**Parameter**: `DEFAULT_MAX_CLOCK_DIFF = (1 << 20) - 1 = 1,048,575`

**Location**: `crates/runner/src/trace.rs:12`

**Purpose**: Constrains the maximum time gap between consecutive accesses to the
same register or memory location.

### Gap-Filling Mechanism

When the clock difference between two accesses exceeds `DEFAULT_MAX_CLOCK_DIFF`,
the tracer inserts **intermediate catch-up entries**:

```rust
// Example: Access at clk=0, next access at clk=350, max_diff=100
// Generates intermediates at clk_prev: 0, 100, 200, 300
// Final access has clk_prev=300, clk=350 (diff=50 <= 100)
```

**Components**:

- `reg_clk_update`: Catch-up entries for register accesses
- `mem_clk_update`: Catch-up entries for memory accesses

### Security Implications

#### 1. **Range Check Dependency**

**Critical Assumption**: The maximum clock difference must not exceed the range
check bound.

**Current Configuration**:

- `DEFAULT_MAX_CLOCK_DIFF = 2^20 - 1`
- Range check: `range_check_20` verifies values in `[0, 2^20 - 1]`

**Security Requirement**: `DEFAULT_MAX_CLOCK_DIFF` **must equal**
`range_check_20` maximum to prevent clock manipulation.

**Verification**: Enforced by AIR constraints in:

- `crates/prover/src/components/mem_clock_update.rs:81`
- `crates/prover/src/components/reg_clock_update.rs:81`

#### 2. **Clock Overflow**

**Issue**: If a program exceeds 2^20 cycles, clock gaps may exceed the range
check bound.

**Mitigation**: The runner enforces a maximum cycle limit (`max_cycles`
parameter). For very long executions, continuations (not yet implemented) would
be required.

#### 3. **Clock Manipulation**

**Threat**: A malicious prover might attempt to:

- Skip clock values to bypass ordering constraints
- Use non-monotonic clock values
- Forge intermediate catch-up entries

**Defense**:

1. **LogUp Lookup Relations**: All memory/register accesses are verified via
   `memory_access` and `register_access` LogUp relations
2. **Continuity Constraints**: Each access's `next` value must match the
   subsequent access's `prev` value (for the same address)
3. **Range Checks**: Clock differences are range-checked to prevent overflow

**Status**: Defense is complete assuming LogUp verification is enabled (see
[Known Limitations](#known-limitations)).

---

## Cryptographic Components

### 1. Poseidon2 Hash Function

**Usage**: Merkle tree commitments for memory and program state

**Implementation**: `crates/prover/src/components/poseidon2.rs`

**Parameters**:

- State width: 16 elements (M31)
- Full rounds: 8
- Partial rounds: 14
- Round constants: Hardcoded (generated with specific seed)

**Security Properties**:

- **Collision Resistance**: Designed for 128-bit security
- **Algebraic Structure**: Optimized for STARK-friendly arithmetic
- **Standardization**: Poseidon2 is a well-studied hash function for
  SNARKs/STARKs

**Concerns**:

- Round constants must be generated with a verifiable process (currently
  hardcoded)
- No independent audit of the Poseidon2 AIR implementation

### 2. Blake2s (via Stwo)

**Usage**: Fiat-Shamir transformation in STARK protocol

**Implementation**: Provided by Stwo (`stwo::core::channel::blake2s`)

**Security Properties**:

- **Collision Resistance**: 128-bit security
- **Standardization**: Blake2s is a standardized hash function (RFC 7693)

**Assumption**: Stwo's Blake2s implementation is correct and secure.

### 3. Bitwise Lookup Tables

**Usage**: XOR, OR, AND operations on 8-bit limbs

**Implementation**: Preprocessed lookup table
(`crates/prover/src/preprocessed/bitwise.rs`)

**Security Properties**:

- **Completeness**: All 2^16 possible (a, b) pairs for each operation
- **Soundness**: LogUp relation ensures prover uses only valid table entries

**Verification**: Bitwise operations are constrained via `bitwise` LogUp
relation.

### 4. Range Checks

**Purpose**: Ensure values are within expected bounds (prevent
overflow/underflow)

**Types**:

- `range_check_20`: Values in `[0, 2^20 - 1]` (clock differences)
- `range_check_8_8`: Two 8-bit limbs (memory/register bytes)
- `range_check_8_11`: Mixed limb sizes (immediate values)
- `range_check_8_8_4`: Mixed limb sizes (special cases)
- `range_check_m31`: Values are valid M31 field elements

**Implementation**: Preprocessed tables in `crates/prover/src/preprocessed/`

**Security Properties**:

- **Soundness**: All table entries are precomputed and committed
- **Completeness**: Tables cover all valid values

**Concern**: Incorrect range check bounds could allow constraint bypass (see
shift carry issue).

### 5. Merkle Trees (Memory Commitment)

**Usage**: Commit to initial memory state

**Implementation**: `crates/runner/src/commitment.rs`

**Parameters**:

- Maximum tree height: `MAX_TREE_HEIGHT = 20` (up to 2^20 memory words)
- Hash function: Poseidon2
- Leaf size: 4 bytes (u32)

**Security Properties**:

- **Binding**: Prover cannot change memory contents after commitment
- **Verifiability**: Merkle proofs verify memory reads

**AIR Constraints**: Memory access Merkle proofs are verified via `merkle` LogUp
relation.

---

## Audit Status

### Current Status

> **NO FORMAL AUDIT COMPLETED**

stark-v has **not** undergone a formal security audit by an external
cryptography firm.

### Audit Scope Recommendations

A formal audit should cover:

1. **AIR Constraint Completeness**
   - Verify all 45 RV32IM instructions are correctly constrained
   - Review all 17 AIR component families (`docs/airs.md`)
   - Check for missing constraints or edge cases

2. **LogUp Relation Soundness**
   - Verify lookup relations are complete and correct
   - Ensure all memory/register accesses are properly tracked
   - Validate range check bounds

3. **Cryptographic Primitives**
   - Review Poseidon2 implementation and parameters
   - Verify round constants generation process
   - Check Merkle tree implementation

4. **Runner Security**
   - Validate memory layout and access controls
   - Review input/output handling
   - Check for edge cases in instruction execution

5. **Integration with Stwo**
   - Verify correct usage of Stwo APIs
   - Ensure Fiat-Shamir transformation is properly applied
   - Validate proof generation and verification flow

### Testing Status

**Current Test Coverage**:

- 177 unit tests across 54 files
- 10 guest program examples
- Multiple end-to-end integration tests (Fibonacci, SHA256, ECDSA, Keccak)

**Test Types**:

- Unit tests for each AIR component
- Integration tests for proof generation and verification
- Edge case tests (x0 register, div-by-zero, overflow)

**Limitations**:

- No formal verification of constraint completeness
- No fuzzing or property-based testing
- No adversarial test cases (malicious proofs)

---

## Responsible Disclosure

### Reporting Security Vulnerabilities

If you discover a security vulnerability in stark-v, please follow responsible
disclosure practices:

1. **Do NOT** publicly disclose the vulnerability until it has been addressed
2. Report the issue privately to the maintainers via:
   - GitHub Security Advisories:
     [https://github.com/starkware-libs/stark-v/security/advisories](https://github.com/starkware-libs/stark-v/security/advisories)
   - Email: [security contact to be added]

3. Provide the following information:
   - Description of the vulnerability
   - Steps to reproduce (proof-of-concept if available)
   - Potential impact and severity assessment
   - Suggested fix (if any)

### Response Timeline

The stark-v team commits to:

- **Acknowledge receipt** within 48 hours
- **Provide initial assessment** within 7 days
- **Issue a fix** within 30 days for critical vulnerabilities
- **Publicly disclose** after a fix is released and users have had time to
  update

### Scope

Vulnerabilities in the following areas are in scope:

- AIR constraint bypass or incompleteness
- Proof forgery or soundness violations
- Cryptographic primitive weaknesses
- Memory safety issues in the runner
- Input/output validation bypasses

Vulnerabilities in the following areas are **out of scope**:

- Stwo library bugs (report to StarkWare directly)
- DoS attacks via resource exhaustion
- Side-channel attacks (timing, power, etc.)
- Social engineering or phishing

### Acknowledgments

We will publicly acknowledge security researchers who responsibly disclose
vulnerabilities (with their permission).

---

## Security Checklist for Deployment

Before deploying stark-v in a production environment, ensure the following:

- [ ] **Enable LogUp verification** (use `track-relations` feature flag)
- [ ] **Fix shift carry range checks** (wait for Phase 1 completion)
- [ ] **Conduct formal security audit** (external cryptography firm)
- [ ] **Implement error handling** (reduce `expect()` usage)
- [ ] **Validate guest program stack usage** (avoid stack overflow)
- [ ] **Review memory layout** (ensure adequate stack/heap sizes)
- [ ] **Test with adversarial inputs** (fuzzing, malicious proofs)
- [ ] **Monitor for Stwo security updates** (keep submodule up-to-date)
- [ ] **Implement operational security** (key management, access controls)
- [ ] **Establish incident response plan** (vulnerability handling process)

**Recommendation**: **Do NOT deploy to production** until the critical issues in
[Known Limitations](#known-limitations) are resolved and an external audit is
completed.

---

## Additional Resources

- **AIR Constraints**: `docs/airs.md`
- **Development Plan**: `PLAN.md`
- **Architecture**: `README.md`
- **Stwo Documentation**:
  [https://github.com/starkware-libs/stwo](https://github.com/starkware-libs/stwo)
- **RISC-V ISA Specification**:
  [https://riscv.org/technical/specifications/](https://riscv.org/technical/specifications/)

---

## Changelog

- **2026-02-06**: Initial security documentation created (v0.1.0)

---

**Disclaimer**: This security documentation is provided for informational
purposes only. stark-v is experimental software and comes with NO WARRANTY. Use
at your own risk.
