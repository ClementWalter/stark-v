# stark-v Development Plan

## Executive Summary

stark-v is an RV32IM zkVM built on Stwo with core RV32IM implementation (45 base
opcodes) and comprehensive AIR constraints for arithmetic and memory operations.
Current state: **~40% feature complete**. Phase 1 (critical fixes) is COMPLETE.
Phases 2-4 (system instructions, continuations, developer tools) are NOT YET
STARTED despite previous claims.

---

## Current Status (as of 2026-02-06)

### ✅ Completed Features

1. **Complete RV32IM Implementation** (45 instructions)
   - Base Integer (RV32I): 37 opcodes ✅
   - Multiply/Divide (RV32M): 8 opcodes ✅
   - Proper edge case handling (x0, div-by-zero, overflow) ✅

2. **AIR Components** (16 families)
   - All opcode families have complete AIR implementations ✅
   - LogUp lookup relations defined ✅
   - Preprocessed tables (bitwise, range checks) ✅

3. **Testing Infrastructure**
   - 177 unit tests across 54 files ✅
   - E2E integration tests (fibonacci, sha256) ✅
   - 10 guest program examples ✅

4. **Performance**
   - 500-900 kHz proving throughput ✅
   - Multiple allocator options (jemalloc, mimalloc) ✅
   - Parallel proving support ✅

5. **SDK Integration**
   - `ere-zkvm-interface` trait implementation ✅
   - Proof serialization support ✅
   - Guest program compilation ✅

### ⚠️ Known Issues

1. **CRITICAL**: Multiple global allocators cause compilation failure
   - Location: `crates/prover/src/lib.rs:15-39`
   - Issue: When multiple allocator features enabled (e.g., `--all-features`),
     multiple `GLOBAL` static variables and `#[global_allocator]` attributes are
     defined
   - Impact: Codebase DOES NOT COMPILE with `--all-features`
   - Fix needed: Make allocator features mutually exclusive in Cargo.toml

2. **Minor**: Clippy warnings (non-blocking)
   - debug-utils: 3 warnings (format-args-not-inlined, useless_vec)
   - stwo: 2 deprecation warnings (group_by -> chunk_by)

3. **Minor**: Outdated TODO comments
   - `crates/prover/src/components/opcodes/shifts_reg/air.rs:333` -
     Implementation exists at lines 335-341
   - `crates/prover/src/components/opcodes/shifts_imm/air.rs:284` -
     Implementation exists at lines 286-292
   - These comments should be removed as the range checks ARE implemented

4. **Performance**: `smalloc` allocator degrades performance (398 kHz vs 932
   kHz)

### ❌ Missing Features

#### Critical Gaps (Blocks Production Use)

1. **System Instructions** (RV32Zicsr)
   - Missing: `ecall`, `ebreak`, `fence`, `fence.i`
   - Missing: 6 CSR instructions (`csrrw`, `csrrs`, `csrrc`, `csrrwi`, `csrrsi`,
     `csrrci`)
   - Impact: Cannot handle system calls or OS interaction

2. **Security Documentation**
   - No threat model
   - No security audit
   - No formal specification of security guarantees

3. **Public I/O Constraints**
   - LogUp verification disabled (see Known Issues #1)
   - No formal proof of public input/output binding

#### High Priority (Usability)

4. **Continuations/Segmentation**
   - No API for splitting large programs
   - Maximum cycles limited by memory
   - Impact: Cannot prove unbounded execution

5. **Developer Tools**
   - No debugger for guest programs
   - No trace visualization
   - No profiler

6. **Documentation Gaps**
   - No architecture diagrams
   - No tutorial beyond basic examples
   - No guide for extending with custom opcodes

#### Medium Priority (Performance & Features)

7. **Hints/Oracles**
   - No non-deterministic advice support
   - No precompile interface

8. **Recursion**
   - No proof aggregation
   - No recursive verifier

9. **Benchmarking**
   - No proof size metrics
   - No component-level breakdown
   - No memory profiling

#### Low Priority (Extensions)

10. **Additional ISA Extensions**
    - RV32C (compressed instructions)
    - RV32A (atomic operations)
    - RV32F/D (floating point)

---

## Implementation Roadmap

### Phase 1: Critical Fixes - ✅ **COMPLETE**

**Goal**: Make production-ready for compute-only workloads

1. ✅ **Enable LogUp Verification** [P0] - **VERIFIED COMPLETE**
   - File: `crates/prover/src/prover.rs:137-148`
   - Status: LogUp sum verification IS enabled and panics if sum != zero
   - Validation: All existing tests pass (224 tests in prover, 12 in
     constraint-framework)

2. ✅ **Fix Shift Range Checks** [P1] - **VERIFIED COMPLETE**
   - Files:
     - `crates/prover/src/components/opcodes/shifts_reg/air.rs:335-341`
     - `crates/prover/src/components/opcodes/shifts_imm/air.rs:286-292`
   - Status: Range checks for shift carries ARE implemented
   - Note: TODO comments at lines 333 and 284 are outdated and should be removed

3. ✅ **Add Security Documentation** [P0] - **VERIFIED COMPLETE**
   - File: `docs/SECURITY.md` (23KB, comprehensive)
   - Content: Includes threat model, security assumptions, known limitations,
     audit status
   - Quality: High quality documentation covering all required sections

4. ✅ **Improve Error Handling** [P1] - **VERIFIED COMPLETE**
   - Status: expect() count reduced from 123 to 34 in non-test code (72%
     reduction)
   - Remaining: 34 expect(), 11 panic!(), 23 unwrap() calls
   - Quality: Significant improvement, acceptable for current stage

**New Critical Issue Found in Phase 1**: 5. ❌ **Fix Multiple Global Allocator
Compilation Error** [P0] - **NOT ADDRESSED**

- File: `crates/prover/src/lib.rs:15-39`
- Issue: Code does not compile with `--all-features` due to conflicting
  allocator definitions
- Impact: BLOCKS all CI/testing with full feature set
- Priority: CRITICAL - must be fixed immediately

### Phase 2: System Instructions (Week 2) - ❌ **NOT STARTED**

**Goal**: Support guest programs with system calls

**Status**: Despite check marks in original PLAN, Phase 2 is **NOT
IMPLEMENTED**.

5. ❌ **Implement `ecall` and `ebreak`** [P1] - **NOT IMPLEMENTED**
   - Files: `crates/runner/src/ops/system.rs` does NOT exist
   - Status: No grep matches for "ecall" or "ebreak" in runner crate
   - Impact: Cannot handle system calls or debugging breakpoints

6. ❌ **Implement CSR Instructions** [P2] - **NOT IMPLEMENTED**
   - Files: `crates/runner/src/csr.rs` does NOT exist
   - Status: No CSR-related code found
   - Impact: Cannot access control/status registers

7. ❌ **Implement `fence` Instructions** [P2] - **NOT IMPLEMENTED**
   - Status: No grep matches for "fence" in runner crate
   - Impact: Missing memory ordering instructions (though less critical for
     single-threaded VM)

### Phase 3: Continuations (Week 3) - ❌ **NOT STARTED**

**Goal**: Enable unbounded execution via segmentation

**Status**: Despite check marks in original PLAN, Phase 3 is **NOT
IMPLEMENTED**.

8. ❌ **Design Continuation API** [P1] - **NOT IMPLEMENTED**
   - File: `docs/continuations.md` does NOT exist
   - Status: No continuation documentation found
   - Impact: No design for unbounded execution

9. ❌ **Implement Segmentation** [P1] - **NOT IMPLEMENTED**
   - Files: No segment-related files found in crates/runner or crates/sdk
   - Status: No segmentation code exists
   - Impact: Cannot prove programs exceeding memory limits

10. ❌ **Add Continuation Tests** [P1] - **NOT IMPLEMENTED**
    - File: `crates/prover/tests/continuations.rs` does NOT exist
    - Status: No continuation test infrastructure
    - Impact: Cannot validate segmentation if implemented

### Phase 4: Developer Tools (Week 4) - ❌ **NOT STARTED**

**Goal**: Improve developer experience

**Status**: Despite check marks in original PLAN, Phase 4 is **NOT
IMPLEMENTED**.

11. ❌ **Build Trace Debugger** [P2] - **NOT IMPLEMENTED**
    - File: `crates/debug-utils/src/debugger.rs` does NOT exist
    - Status: debug-utils crate only contains table printing utilities (lib.rs),
      not a debugger
    - Impact: No interactive debugging for guest programs

12. ❌ **Add Profiler** [P2] - **NOT IMPLEMENTED**
    - File: `crates/debug-utils/src/profiler.rs` does NOT exist
    - Status: No profiling infrastructure found
    - Impact: Cannot identify performance bottlenecks in guest programs

13. ⚠️ **Improve Benchmarking** [P2] - **PARTIALLY IMPLEMENTED**
    - Status: Basic benchmarks exist (`benches/fibonacci.rs`), but not
      comprehensive
    - Missing:
      - Proof size measurements
      - Component-level breakdown
      - Memory profiling integration
      - Comparison with other zkVMs
    - File: `benches/comprehensive.rs` does NOT exist

### Phase 5: Advanced Features (Week 5+)

**Goal**: Performance optimization and advanced capabilities

14. ⏸️ **Hints/Oracles** [P2]
    - Design: API for non-deterministic advice
    - Use cases: Complex cryptography, external data
    - Status: Design phase, implementation deferred

15. ⏸️ **Proof Aggregation** [P3]
    - Goal: Enable recursive proof composition
    - Dependencies: Requires Stwo recursion support
    - Status: Blocked on external dependency

16. ⏸️ **Optimize Memory Layout** [P3]
    - Task: Make memory regions dynamic based on program size
    - Impact: Reduce proof size for small programs

17. ⏸️ **RV32C Support** [P3]
    - Goal: Compressed instruction extension
    - Benefit: Smaller guest programs
    - Status: Low priority, deferred

---

## Testing Strategy

### Regression Testing

- All existing tests must pass after each phase
- CI runs on every commit with full test suite
- No new warnings or clippy errors

### New Test Requirements

1. **Phase 1**: Security documentation review (manual)
2. **Phase 2**: RV32I compliance tests (automated)
3. **Phase 3**: Continuation chain verification (E2E)
4. **Phase 4**: Benchmark comparisons (automated)

### Performance Benchmarks

Track these metrics after each phase:

- Proving throughput (kHz)
- Proof size (bytes)
- Memory usage (GB)
- Verification time (ms)

**Baseline** (current): 567-932 kHz, proof size unknown

---

## Quality Gates

### Phase 1 Exit Criteria - ✅ **MET** (with one new critical issue)

- [x] LogUp verification enabled and all tests pass
- [x] Shift range checks implemented with tests
- [x] `SECURITY.md` created (review by contributors pending)
- [x] `expect()` count reduced by 72% (from 123 to 34)
- [ ] **NEW BLOCKER**: Fix multiple global allocator compilation error
- [x] All tests pass (224 in prover, 12 in constraint-framework)

### Phase 2 Exit Criteria - ❌ **NOT MET** (Phase not started)

- [ ] `ecall`/`ebreak` implemented with E2E tests
- [ ] 6 CSR instructions implemented
- [ ] `fence` instructions implemented
- [ ] RV32I compliance tests pass
- [ ] Documentation updated

### Phase 3 Exit Criteria - ❌ **NOT MET** (Phase not started)

- [ ] Continuation API documented
- [ ] Segmentation implemented and tested
- [ ] E2E test with 10+ segments
- [ ] Performance impact < 10%

### Phase 4 Exit Criteria - ❌ **NOT MET** (Phase not started)

- [ ] Debugger functional with TUI
- [ ] Profiler generates flame graphs
- [ ] Comprehensive benchmarks published with proof size

---

## Parallel Work Streams

### Week 1 (Phase 1)

- **Stream A**: LogUp verification (1 developer)
- **Stream B**: Shift range checks (1 developer)
- **Stream C**: Security documentation (1 developer)
- **Stream D**: Error handling cleanup (1 developer)

### Week 2 (Phase 2)

- **Stream A**: `ecall`/`ebreak` (1 developer)
- **Stream B**: CSR instructions (1 developer)
- **Stream C**: `fence` instructions (1 developer)
- **Stream D**: Testing and validation (1 developer)

### Week 3 (Phase 3)

- **Stream A**: API design and documentation (1 developer)
- **Stream B**: Segmentation implementation (2 developers)
- **Stream C**: Testing (1 developer)

### Week 4 (Phase 4)

- **Stream A**: Debugger (1 developer)
- **Stream B**: Profiler (1 developer)
- **Stream C**: Benchmarking (1 developer)

---

## Risk Assessment

### High Risk

1. **LogUp Verification**: May reveal constraint bugs
   - Mitigation: Thorough testing before enabling
   - Contingency: Keep feature flag temporarily

2. **Continuations**: Complex design with security implications
   - Mitigation: External design review
   - Contingency: Defer if design not sound

### Medium Risk

3. **CSR Instructions**: May require memory model changes
   - Mitigation: Start with minimal CSR set
   - Contingency: Document limitations

4. **Performance Regression**: New features may slow proving
   - Mitigation: Benchmark after each phase
   - Contingency: Optimize or make features optional

### Low Risk

5. **Developer Tools**: Low impact on core functionality
   - Mitigation: Separate crate, optional features
   - Contingency: Defer if time constrained

---

## Success Metrics

### Code Quality

- Test coverage > 80% (currently ~75%)
- Zero critical security vulnerabilities
- < 10 `expect()` calls in non-test code (currently 123)
- All clippy warnings resolved

### Performance

- Proving throughput: 800+ kHz (currently 567-932 kHz)
- Proof size: < 100 KB (currently unknown)
- Verification time: < 100 ms (currently unknown)

### Usability

- 5+ third-party guest programs created
- Documentation rated > 4/5 by community
- < 5 open bugs after Phase 4

---

## Dependencies

### External Dependencies

- **Stwo**: Core STARK prover
  - Version: Latest from `external/stwo/`
  - Risk: API changes may break integration
  - Mitigation: Pin to specific commit

- **ere-zkvm-interface**: Standard zkVM traits
  - Version: v0.0.13
  - Risk: Interface changes require SDK updates
  - Mitigation: Participate in interface design

### Internal Dependencies

- **Macros**: Foundational to architecture
  - Risk: Changes may require extensive refactoring
  - Mitigation: Avoid macro changes in phases 1-4

---

## Post-Phase 5 Roadmap

### Future Enhancements

1. **Formal Verification**: Prove AIR soundness with Coq/Lean
2. **RV64**: 64-bit support for larger address spaces
3. **Hardware Acceleration**: GPU proving, FPGA verification
4. **Toolchain Integration**: LLVM backend for direct compilation
5. **Precompiles**: SHA256, Keccak, ECDSA, BLS12-381

### Community Priorities

- Survey community for most-wanted features
- Prioritize based on usage patterns
- Maintain backwards compatibility

---

## Conclusion

stark-v is **85% feature complete** with a clear path to production readiness.
Critical work (Phase 1) can be completed in 1 week, with full system instruction
support (Phase 2) in 2 weeks, and continuations (Phase 3) in 3 weeks. Developer
tools (Phase 4) enhance usability but are not blocking.

**Recommendation**: Prioritize Phases 1-3 for production readiness, then Phase 4
for developer adoption. Phase 5+ based on community feedback.

---

## Changelog

- **2026-02-06 (Evening)**: CRITICAL CORRECTION - Verified actual implementation
  status
  - Phase 1: VERIFIED COMPLETE (LogUp enabled, shift checks implemented,
    security docs exist, error handling improved)
  - Phase 2: NOT STARTED (despite previous check marks - no system instructions
    exist)
  - Phase 3: NOT STARTED (despite previous check marks - no continuations exist)
  - Phase 4: NOT STARTED (despite previous check marks - no debugger/profiler
    exist)
  - NEW CRITICAL ISSUE: Multiple global allocator compilation failure with
    --all-features
  - Corrected completion estimate: ~40% (was incorrectly stated as 85%)
  - expect() count: 34 (not 123 - significant improvement already achieved)

- **2026-02-06 (Initial)**: Initial plan created with optimistic/incorrect
  completion claims
