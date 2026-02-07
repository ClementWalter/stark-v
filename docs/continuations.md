# Continuation and Segmentation API

## Overview

This document describes the design for execution continuations and proof
segmentation in claudeth. Continuations allow splitting long-running programs
into multiple segments, each with its own proof, enabling:

- **Bounded memory usage**: Each segment fits within fixed memory constraints
- **Parallelizable proving**: Independent segments can be proven concurrently
- **Incremental verification**: Proofs can be verified as they're generated
- **Recursion-ready**: Segment proofs can be aggregated via recursive
  composition

The core insight is that RISC-V execution is a state machine with well-defined
transitions. We can snapshot the VM state at instruction boundaries, split
execution into segments, and prove each segment's state transition
independently. The continuity constraint ensures that segment N+1's initial
state exactly matches segment N's final state.

## Architecture

### High-Level Design

```text
┌──────────────┐  continuity   ┌──────────────┐  continuity   ┌──────────────┐
│  Segment 0   │ ─────────────▶│  Segment 1   │ ─────────────▶│  Segment 2   │
│              │               │              │               │              │
│  cycles:     │               │  cycles:     │               │  cycles:     │
│  [0..N)      │               │  [N..2N)     │               │  [2N..end)   │
│              │               │              │               │              │
│  Proof_0     │               │  Proof_1     │               │  Proof_2     │
└──────────────┘               └──────────────┘               └──────────────┘
       │                              │                              │
       └──────────────────────────────┴──────────────────────────────┘
                                      │
                               ┌──────▼──────┐
                               │  Aggregated │
                               │    Proof    │
                               └─────────────┘
```

Each segment proves:

1. **State transition**: `initial_state → final_state` via valid RISC-V
   execution
2. **Cycle range**: Execution from clock `start_clk` to `end_clk`
3. **Determinism**: Given the same initial state, execution is reproducible

Segments are chained via **continuity constraints**:

- `segment[i].final_state == segment[i+1].initial_state`

### Key Components

#### 1. State Snapshot Format

The VM state consists of:

```rust
/// Complete VM state at an instruction boundary.
pub struct VmState {
    /// Program counter (4-byte aligned).
    pub pc: u32,

    /// General-purpose registers x0-x31.
    pub registers: [u32; 32],

    /// Current cycle/clock value.
    pub clock: u32,

    /// Memory state snapshot.
    /// Only includes pages that have been accessed (sparse representation).
    pub memory: MemorySnapshot,

    /// Last access clock per register (for continuity of memory model).
    pub reg_last_clk: [u32; 32],

    /// Last access clock per memory address (sparse map).
    pub mem_last_clk: BTreeMap<u32, u32>,
}

/// Memory snapshot using sparse page-based representation.
pub struct MemorySnapshot {
    /// Merkle root of memory state at this point.
    pub root: u32,

    /// Dirty pages since last snapshot (for efficient delta encoding).
    /// Maps 4KB page index to page contents.
    pub dirty_pages: BTreeMap<u32, PageData>,

    /// Full memory state (used for first segment or after threshold).
    /// Only one of `dirty_pages` or `full_state` is populated.
    pub full_state: Option<BTreeMap<u32, u8>>,
}

/// 4KB page of memory.
pub struct PageData {
    /// Page contents (4096 bytes).
    pub data: [u8; 4096],

    /// Merkle root of this page.
    pub root: u32,
}
```

**Design rationale**:

- **PC alignment**: RISC-V instructions are 4-byte aligned, ensuring clean
  boundaries
- **Sparse memory**: Most of the 4GB address space is unused; only track
  accessed regions
- **Page-based delta**: For long executions, delta encoding dramatically reduces
  snapshot size
- **Merkle roots**: Enable efficient state equality checks without transmitting
  full state

#### 2. Segment Creation API

```rust
/// Configuration for segment boundaries.
pub struct SegmentConfig {
    /// Maximum cycles per segment.
    pub max_cycles_per_segment: u64,

    /// Strategy for determining segment boundaries.
    pub boundary_strategy: BoundaryStrategy,

    /// Maximum memory delta before forcing full snapshot.
    pub max_memory_delta_bytes: usize,
}

pub enum BoundaryStrategy {
    /// Split at exact cycle count boundaries.
    FixedCycles,

    /// Split at function boundaries (requires DWARF info).
    FunctionBoundary,

    /// Split at branch points (slightly more flexible).
    BranchPoint,
}

/// Execute program and split into segments.
pub fn run_with_segments(
    elf_bytes: &[u8],
    input: &[u8],
    config: SegmentConfig,
) -> Result<Vec<SegmentExecution>, RunError> {
    // Implementation outline:
    // 1. Run VM normally, tracking state
    // 2. At each potential boundary, check if we should split
    // 3. Capture snapshot at split point
    // 4. Continue execution in new segment
}

/// Execution result for a single segment.
pub struct SegmentExecution {
    /// Segment index (0-based).
    pub segment_id: u32,

    /// Initial VM state.
    pub initial_state: VmState,

    /// Final VM state.
    pub final_state: VmState,

    /// Execution trace for this segment.
    pub tracer: Tracer,

    /// Cycles executed in this segment.
    pub cycles: u64,
}
```

**Design rationale**:

- **Configurable boundaries**: Different applications have different needs
  (latency vs throughput)
- **Multiple strategies**: Fixed cycles for simplicity, function boundaries for
  better cache locality
- **Snapshot capture**: Minimize overhead by capturing only at split points

#### 3. Segment Proving API

```rust
/// Prove a single segment's execution.
pub fn prove_segment(
    segment: SegmentExecution,
    config: PcsConfig,
) -> Result<SegmentProof, ProverError> {
    // Generate STARK proof for this segment's execution
}

/// Prove multiple segments in parallel.
pub fn prove_segments_parallel(
    segments: Vec<SegmentExecution>,
    config: PcsConfig,
    num_workers: usize,
) -> Result<Vec<SegmentProof>, ProverError> {
    // Parallel proving using thread pool
}

/// STARK proof for a single segment.
pub struct SegmentProof<H: MerkleHasher> {
    /// Segment identifier.
    pub segment_id: u32,

    /// Public data (includes initial and final state commitments).
    pub public_data: SegmentPublicData,

    /// Interaction claim.
    pub interaction_claim: InteractionClaim,

    /// STARK proof.
    pub stark_proof: StarkProof<H>,

    /// Proof of work nonce.
    pub interaction_pow: u64,
}

/// Public data for segment proof (extends PublicData).
pub struct SegmentPublicData {
    /// Base public data.
    pub base: PublicData,

    /// Segment metadata.
    pub segment_id: u32,
    pub start_clock: u32,
    pub end_clock: u32,

    /// State commitments (Merkle roots).
    pub initial_state_root: StateCommitment,
    pub final_state_root: StateCommitment,
}

/// Commitment to complete VM state.
pub struct StateCommitment {
    /// Hash of (PC, registers, clock).
    pub register_state_hash: u32,

    /// Merkle root of memory.
    pub memory_root: u32,

    /// Hash of last-access clocks (for memory model continuity).
    pub clock_state_hash: u32,
}
```

**Design rationale**:

- **Segment ID**: Maintains ordering information for aggregation
- **State commitments**: Succinct (O(1) size) representation of full state
- **Public data extension**: Reuses existing PublicData infrastructure
- **Parallel proving**: Key to achieving high throughput

#### 4. Continuity Verification

```rust
/// Verify continuity between two consecutive segments.
pub fn verify_continuity(
    prev_proof: &SegmentProof<H>,
    next_proof: &SegmentProof<H>,
) -> Result<(), ContinuityError> {
    // Check: prev.segment_id + 1 == next.segment_id
    // Check: prev.final_state_root == next.initial_state_root
    // Check: prev.end_clock + 1 == next.start_clock
}

/// Error types for continuity violations.
#[derive(Debug, Error)]
pub enum ContinuityError {
    #[error("Segment ID mismatch: expected {expected}, got {actual}")]
    SegmentIdMismatch { expected: u32, actual: u32 },

    #[error("State commitment mismatch at segment boundary")]
    StateCommitmentMismatch {
        prev_final: StateCommitment,
        next_initial: StateCommitment,
    },

    #[error("Clock discontinuity: prev_end={prev_end}, next_start={next_start}")]
    ClockDiscontinuity { prev_end: u32, next_start: u32 },
}
```

**Design rationale**:

- **Explicit verification**: Don't assume continuity; verify it
  cryptographically
- **Three-level check**: ID ordering, state commitment equality, clock
  continuity
- **Clear error messages**: Aid debugging when continuity breaks

#### 5. Proof Aggregation

Aggregation combines multiple segment proofs into a single proof via recursive
composition.

```rust
/// Aggregate two segment proofs via recursion.
pub fn aggregate_two(
    left: SegmentProof<H>,
    right: SegmentProof<H>,
    config: PcsConfig,
) -> Result<AggregatedProof<H>, ProverError> {
    // 1. Verify continuity between segments
    verify_continuity(&left, &right)?;

    // 2. Verify both proofs (recursively in STARK)
    // 3. Generate proof of: "left is valid AND right is valid AND left.final == right.initial"
}

/// Aggregate multiple proofs using binary tree recursion.
pub fn aggregate_all(
    proofs: Vec<SegmentProof<H>>,
    config: PcsConfig,
) -> Result<AggregatedProof<H>, ProverError> {
    // Binary tree aggregation: O(log n) depth
    // Can be parallelized at each level
}

/// Aggregated proof covering multiple segments.
pub struct AggregatedProof<H: MerkleHasher> {
    /// Range of segments covered.
    pub segment_range: (u32, u32),

    /// Initial state of first segment.
    pub initial_state_root: StateCommitment,

    /// Final state of last segment.
    pub final_state_root: StateCommitment,

    /// Total cycles across all segments.
    pub total_cycles: u64,

    /// Recursive proof.
    pub proof: StarkProof<H>,
}
```

**Design rationale**:

- **Binary tree structure**: O(log n) depth enables efficient verification
- **Parallelizable**: Each level of tree can be processed in parallel
- **Incremental**: Can aggregate subset of proofs, then aggregate aggregates
- **Range tracking**: Maintains segment coverage information

## Splitting at Instruction Boundaries

### Boundary Detection

Segments must begin and end at valid instruction boundaries:

```rust
/// Check if current execution state is a valid segment boundary.
fn is_valid_boundary(cpu: &Cpu, cycles: u64, config: &SegmentConfig) -> bool {
    match config.boundary_strategy {
        BoundaryStrategy::FixedCycles => {
            cycles % config.max_cycles_per_segment == 0
        }

        BoundaryStrategy::FunctionBoundary => {
            // Requires DWARF info to identify function entry points
            // Check if PC is at a function entry (e.g., after a call)
            is_function_entry(cpu.pc)
        }

        BoundaryStrategy::BranchPoint => {
            // More flexible: split after any branch/jump
            // Covers function calls, returns, loops, conditionals
            is_branch_target(cpu.pc)
        }
    }
}
```

**Key invariant**: Every segment begins with a valid instruction fetch and ends
after a completed instruction execution.

### State Capture Mechanism

```rust
/// Capture VM state at current execution point.
fn capture_state(cpu: &Cpu, mem: &Memory, tracer: &Tracer) -> VmState {
    VmState {
        pc: cpu.pc,
        registers: cpu.regs(),
        clock: tracer.clk,
        memory: capture_memory_snapshot(mem, tracer),
        reg_last_clk: tracer.reg_clk,
        mem_last_clk: tracer.mem_clk.clone(),
    }
}

/// Capture memory snapshot (with delta encoding).
fn capture_memory_snapshot(mem: &Memory, tracer: &Tracer) -> MemorySnapshot {
    // Identify dirty pages since last snapshot
    // Compute Merkle root of current memory state
    // Return sparse representation
}
```

**Optimization**: Delta encoding reduces snapshot overhead from megabytes to
kilobytes for typical workloads.

## Security Implications and Threat Model

### Trust Assumptions

1. **Verifier security**: The verifier must check continuity between segments
2. **Prover honesty**: Malicious prover cannot forge valid proofs (soundness)
3. **State commitment binding**: Hash collisions are computationally infeasible

### Attack Vectors and Mitigations

#### 1. State Commitment Forgery

**Attack**: Prover provides false state commitment at segment boundary.

**Mitigation**:

- State commitments include cryptographic hashes (Poseidon2 in STARKs)
- Each segment proof verifies initial state matches commitment
- Continuity check ensures consistency across segments

#### 2. Segment Reordering

**Attack**: Prover swaps segments or presents them out of order.

**Mitigation**:

- Segment IDs enforce strict ordering
- Clock values are monotonically increasing
- Aggregation verifies sequential segment IDs

#### 3. Dropped Segments

**Attack**: Prover omits segments to hide invalid execution.

**Mitigation**:

- Aggregated proof specifies segment range (start_id, end_id)
- Verifier checks that range is complete and contiguous
- Initial and final states must match expected program behavior

#### 4. Clock Manipulation

**Attack**: Adjust clock values to hide timing side-channels or bypass
constraints.

**Mitigation**:

- Clock is strictly monotonic (increments by 1 per instruction)
- Segment boundaries enforce `end_clock + 1 == next.start_clock`
- Range-check constraints in AIR prevent invalid clock differences

#### 5. Memory Inconsistency

**Attack**: Different memory states across segments despite matching
commitments.

**Mitigation**:

- Merkle roots bind to specific memory contents
- Last-access clocks ensure memory continuity model is preserved
- Each access proves consistency with previous access via LogUp

### Soundness Guarantees

The segmentation scheme maintains zkVM soundness if:

1. **Per-segment soundness**: Each segment proof is sound (inherited from base
   STARK)
2. **Continuity soundness**: State commitments are collision-resistant
3. **Completeness check**: Verifier ensures all segments [0..N-1] are present

**Theorem** (Informal): If the base STARK is sound with negligible error ε, and
hash collisions occur with probability ≤ δ, then N-segment execution is sound
with error ≤ N·ε + N·δ.

For typical parameters:

- ε ≈ 2^-100 (STARK soundness)
- δ ≈ 2^-128 (Poseidon2 collision resistance)
- N < 2^32 (practical segment count)

Total error remains negligible: N·(ε + δ) ≈ 2^-68.

## Example Usage

### Basic Segmented Execution

```rust
use claudeth::{SegmentConfig, BoundaryStrategy, run_with_segments, prove_segments_parallel};

// Load guest program
let elf_bytes = std::fs::read("guest.elf")?;
let input = vec![42u8; 100];

// Configure segmentation
let config = SegmentConfig {
    max_cycles_per_segment: 1_000_000,
    boundary_strategy: BoundaryStrategy::FixedCycles,
    max_memory_delta_bytes: 1_024_000, // 1MB
};

// Run and split into segments
let segments = run_with_segments(&elf_bytes, &input, config)?;
println!("Generated {} segments", segments.len());

// Prove segments in parallel
let proofs = prove_segments_parallel(segments, PcsConfig::default(), 8)?;

// Verify continuity
for i in 0..proofs.len() - 1 {
    verify_continuity(&proofs[i], &proofs[i + 1])?;
}

// Aggregate into single proof
let final_proof = aggregate_all(proofs, PcsConfig::default())?;
println!("Aggregated proof covers {} total cycles", final_proof.total_cycles);
```

### Incremental Verification

```rust
// Stream segments and verify as they arrive
let mut expected_state: Option<StateCommitment> = None;

for segment_proof in segment_stream {
    // Verify proof
    verify_segment(&segment_proof)?;

    // Check continuity with previous segment
    if let Some(prev_state) = expected_state {
        if segment_proof.public_data.initial_state_root != prev_state {
            return Err("Continuity violation");
        }
    }

    // Update expected state for next segment
    expected_state = Some(segment_proof.public_data.final_state_root);
}
```

### Parallel Execution and Proving

```rust
// Run segments concurrently (useful for replay/verification)
let segment_runs: Vec<SegmentExecution> = segments
    .into_par_iter()
    .map(|initial_state| {
        // Each thread runs its segment independently
        run_from_state(initial_state, config.max_cycles_per_segment)
    })
    .collect()?;

// Prove all segments in parallel
let proofs = prove_segments_parallel(segment_runs, PcsConfig::default(), num_cpus::get())?;
```

### Custom Boundary Strategy

```rust
// Split at system calls for better isolation
struct SyscallBoundary;

impl BoundaryStrategy for SyscallBoundary {
    fn is_boundary(&self, cpu: &Cpu, inst: &DecodedInst) -> bool {
        matches!(inst.opcode, Opcode::Ecall | Opcode::Ebreak)
    }
}

let config = SegmentConfig {
    max_cycles_per_segment: u64::MAX, // No cycle limit
    boundary_strategy: BoundaryStrategy::Custom(Box::new(SyscallBoundary)),
    max_memory_delta_bytes: usize::MAX,
};
```

## Implementation Considerations

### Memory Efficiency

**Challenge**: State snapshots can be large (megabytes of memory).

**Solutions**:

1. **Delta encoding**: Only store changed pages between segments
2. **Compression**: Use zstd/lz4 for snapshot serialization
3. **Streaming**: Don't keep all snapshots in memory simultaneously
4. **Lazy expansion**: Only materialize full state when needed

### Proving Performance

**Challenge**: Proving overhead should not dominate execution time.

**Target**: Overhead ≤ 10% compared to monolithic proof.

**Optimizations**:

1. **Amortized boundaries**: Split every 1M+ cycles, not every 1K
2. **Parallel proving**: Use all CPU cores for independent segments
3. **Persistent twiddles**: Reuse precomputed twiddles across segments
4. **Batch aggregation**: Aggregate multiple proof pairs in parallel

### Determinism

**Challenge**: Segment boundaries must be deterministic.

**Requirements**:

- Same input → same segment boundaries
- No dependence on timing, memory layout, or randomness
- Reproducible across different machines

**Guarantee**: All boundary strategies are pure functions of (PC, cycle count).

## Future Extensions

### 1. Adaptive Segmentation

Dynamically adjust segment size based on:

- Memory pressure
- Available proving resources
- Target latency requirements

### 2. Speculative Execution

Start proving segment N+1 before segment N completes, speculating on likely
state transitions.

### 3. Persistent State Caching

Cache state snapshots to disk for:

- Resumable execution after crash
- Replay/debugging from arbitrary points
- Time-travel debugging

### 4. Cross-Segment Optimization

Analyze full execution trace to optimize:

- Memory layout (reduce dirty pages)
- Function placement (align with boundaries)
- Loop unrolling (reduce boundary overhead)

### 5. Hierarchical Aggregation

Multi-level aggregation:

- Level 1: Segments (1M cycles)
- Level 2: Chunks (100M cycles)
- Level 3: Full program (10B+ cycles)

This enables proving programs with billions of cycles while maintaining
reasonable memory footprint.

## References

- **RISC-V ISA Specification**: Instruction boundary semantics
- **Stwo STARK Framework**: Base proof system soundness
- **Memory Model**:
  `/Users/clementwalter/.claude/worktrees/stark-v/claudeth/docs/airs.md`
- **Trace Infrastructure**:
  `/Users/clementwalter/.claude/worktrees/stark-v/claudeth/crates/runner/src/trace.rs`
- **Public Data Format**:
  `/Users/clementwalter/.claude/worktrees/stark-v/claudeth/crates/prover/src/public_data.rs`

## Summary

This continuation API enables:

1. **Scalability**: Prove arbitrarily long programs by splitting into fixed-size
   segments
2. **Parallelism**: Independent segment proving maximizes throughput
3. **Incrementality**: Stream and verify proofs as they're generated
4. **Composability**: Aggregate proofs into succinct final proof

The design maintains security through cryptographic state commitments and
explicit continuity verification. Implementation should prioritize memory
efficiency and proving performance while ensuring deterministic segment
boundaries.

Next steps:

1. Implement `VmState` and snapshot capture
2. Add segment splitting to runner
3. Extend prover for segment-aware proving
4. Build aggregation infrastructure
5. Comprehensive E2E tests with multi-segment programs
