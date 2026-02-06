# stark-v Development Plan

## Executive Summary

stark-v is a production-quality RV32IM zkVM built on Stwo with complete
instruction set implementation (45 opcodes) and comprehensive AIR constraints.
Current state: **85% feature complete**, with identified gaps in system
instructions, continuations, and security documentation.

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

1. **CRITICAL**: LogUp sum verification disabled by default
   - Location: `crates/prover/src/prover.rs:136`
   - Issue: `track-relations` feature flag required
   - Impact: Public I/O not fully constrained in production mode

2. **TODO Comments** (2 instances)
   - `crates/prover/src/components/opcodes/shifts_reg/air.rs:333`
   - `crates/prover/src/components/opcodes/shifts_imm/air.rs:284`
   - Issue: Range check shift carries not implemented

3. **Performance**: `smalloc` allocator degrades performance (398 kHz vs 932
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

### Phase 1: Critical Fixes (Week 1)

**Goal**: Make production-ready for compute-only workloads

1. ✅ **Enable LogUp Verification** [P0]
   - File: `crates/prover/src/prover.rs`
   - Task: Remove `track-relations` feature flag requirement
   - Validation: All existing tests pass, public I/O verified

2. ✅ **Fix Shift Range Checks** [P1]
   - Files:
     - `crates/prover/src/components/opcodes/shifts_reg/air.rs`
     - `crates/prover/src/components/opcodes/shifts_imm/air.rs`
   - Task: Implement range check for shift carries
   - Validation: Add tests for all shift amounts (0-31)

3. ✅ **Add Security Documentation** [P0]
   - File: `docs/SECURITY.md` (new)
   - Content:
     - Threat model
     - Security assumptions
     - Known limitations
     - Audit status

4. ✅ **Improve Error Handling** [P1]
   - Task: Replace `expect()` with proper error propagation
   - Files: Scan for all uses in `crates/runner/` and `crates/prover/`

### Phase 2: System Instructions (Week 2)

**Goal**: Support guest programs with system calls

5. ✅ **Implement `ecall` and `ebreak`** [P1]
   - Files:
     - `crates/runner/src/ops/system.rs` (new)
     - `crates/prover/src/components/opcodes/system/` (new)
   - Design:
     - `ecall`: Environment call (halt with return code)
     - `ebreak`: Breakpoint (for debugging)
   - Validation: Add E2E test with system calls

6. ✅ **Implement CSR Instructions** [P2]
   - Files:
     - `crates/runner/src/csr.rs` (new)
     - `crates/prover/src/components/opcodes/csr/` (new)
   - Design:
     - Support minimal CSR set (mstatus, mtvec, mcause, etc.)
     - Read-only cycle counters
   - Validation: Add CSR access tests

7. ✅ **Implement `fence` Instructions** [P2]
   - Task: Add no-op implementations (single-threaded VM)
   - Validation: Ensure RV32I compliance tests pass

### Phase 3: Continuations (Week 3)

**Goal**: Enable unbounded execution via segmentation

8. ✅ **Design Continuation API** [P1]
   - File: `docs/continuations.md` (new)
   - Content:
     - Segmentation strategy
     - State commitment format
     - Public input/output format
   - Review: Seek community feedback

9. ✅ **Implement Segmentation** [P1]
   - Files:
     - `crates/runner/src/segments.rs` (new)
     - `crates/sdk/src/segments.rs` (new)
   - Features:
     - Configurable segment size
     - State serialization between segments
     - Merkle tree commitment to state
   - Validation: Split large fibonacci across segments

10. ✅ **Add Continuation Tests** [P1]
    - File: `crates/prover/tests/continuations.rs` (new)
    - Tests:
      - Single segment (baseline)
      - Multiple segments with state transfer
      - Verification of segment chain

### Phase 4: Developer Tools (Week 4)

**Goal**: Improve developer experience

11. ✅ **Build Trace Debugger** [P2]
    - File: `crates/debug-utils/src/debugger.rs`
    - Features:
      - Step-through execution
      - Register/memory inspection
      - Breakpoint support
    - UI: CLI-based with TUI (ratatui)

12. ✅ **Add Profiler** [P2]
    - File: `crates/debug-utils/src/profiler.rs`
    - Features:
      - Cycle count per instruction
      - Hot spot identification
      - Flame graph generation
    - Integration: Add `--profile` flag to SDK

13. ✅ **Improve Benchmarking** [P2]
    - Tasks:
      - Add proof size measurements
      - Add component-level breakdown
      - Add memory profiling (with `peak-alloc`)
      - Compare with other zkVMs (SP1, RISC Zero)
    - File: `benches/comprehensive.rs` (new)

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

### Phase 1 Exit Criteria

- [ ] LogUp verification enabled and all tests pass
- [ ] Shift range checks implemented with tests
- [ ] `SECURITY.md` reviewed by 2+ contributors
- [ ] `expect()` count reduced by 50%
- [ ] All CI checks pass

### Phase 2 Exit Criteria

- [ ] `ecall`/`ebreak` implemented with E2E tests
- [ ] 6 CSR instructions implemented
- [ ] RV32I compliance tests pass
- [ ] Documentation updated

### Phase 3 Exit Criteria

- [ ] Continuation API documented
- [ ] Segmentation implemented and tested
- [ ] E2E test with 10+ segments
- [ ] Performance impact < 10%

### Phase 4 Exit Criteria

- [ ] Debugger functional with TUI
- [ ] Profiler generates flame graphs
- [ ] Benchmarks published with proof size

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

- **2026-02-06**: Initial plan created based on comprehensive codebase analysis
