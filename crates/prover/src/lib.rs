#![allow(non_camel_case_types)]
#![feature(
    allocator_api,
    portable_simd,
    iter_array_chunks,
    macro_metavar_expr_concat
)]

// Allocator configuration via features
#[cfg(feature = "smalloc")]
use smalloc::Smalloc;
#[cfg(feature = "smalloc")]
#[global_allocator]
static GLOBAL: Smalloc = Smalloc::new();

#[cfg(feature = "smalloc")]
#[ctor::ctor]
unsafe fn init_smalloc() {
    GLOBAL.init();
}

#[cfg(feature = "peak-alloc")]
use peak_alloc::PeakAlloc;
#[cfg(feature = "peak-alloc")]
#[global_allocator]
pub static PEAK_ALLOC: PeakAlloc = PeakAlloc;

#[cfg(feature = "jemalloc")]
use tikv_jemallocator::Jemalloc;
#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;
#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Print all enabled features for debugging/benchmarking.
pub fn print_enabled_features() {
    use tracing::info;

    let features: Vec<&str> = vec![
        #[cfg(feature = "parallel")]
        "parallel",
        #[cfg(not(feature = "parallel"))]
        "non-parallel",
        #[cfg(feature = "peak-alloc")]
        "peak-alloc",
        #[cfg(feature = "jemalloc")]
        "jemalloc",
        #[cfg(feature = "mimalloc")]
        "mimalloc",
        #[cfg(feature = "smalloc")]
        "smalloc",
    ];

    if features.is_empty() {
        info!("Features: (none)");
    } else {
        info!("Features: {}", features.join(", "));
    }
}

// Import all macros from stwo-macros crate
#[macro_use]
extern crate stwo_macros;
pub mod components;
pub mod errors;
pub mod preprocessed;
pub mod prover;
pub mod public_data;
pub mod relations;
pub mod verifier;

pub use errors::VerificationError;
pub use prover::prove_rv32im;
pub use public_data::PublicData;
pub use verifier::verify_rv32im;

/// Serializable preprocessed data for RV32IM proving.
///
/// Caches the expensive preprocessing computation including:
/// - Extended polynomial evaluations (after interpolation and blowup)
/// - Merkle tree layers (the commitment structure)
/// - Column IDs for trace allocation
///
/// The data can be serialized to disk and reused across multiple proofs,
/// avoiding the need to rebuild the Merkle tree each time.
#[derive(Clone, Serialize, Deserialize)]
pub struct Preprocessing {
    /// Column IDs for trace allocation.
    pub ids: Vec<String>,
    /// Original column log sizes (before extension) — used by the verifier.
    pub log_sizes: Vec<u32>,
    /// Domain log sizes for each extended evaluation column (after blowup).
    pub domain_log_sizes: Vec<u32>,
    /// Extended polynomial evaluations (after blowup) — raw u32 data per column.
    /// Each inner Vec contains the flattened PackedBaseField data as u32 values.
    pub extended_evals: Vec<Vec<u32>>,
    /// Merkle tree layers (hashes) — Vec<Blake2sHash> per layer.
    /// First layer is the root (single hash), last layer is the largest.
    pub merkle_layers: Vec<Vec<stwo::core::vcs::blake2_hash::Blake2sHash>>,
}

impl Preprocessing {
    /// Get the preprocessed column IDs as PreProcessedColumnId objects.
    pub fn column_ids(
        &self,
    ) -> Vec<stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId> {
        self.ids
            .iter()
            .map(
                |id| stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId {
                    id: id.clone(),
                },
            )
            .collect()
    }

    /// Reconstruct the CommitmentTreeProver from the cached data.
    ///
    /// Converts the serialized data back into the stwo types needed for proving.
    /// Uses 64-byte aligned allocation for SIMD compatibility.
    pub fn to_commitment_tree(
        &self,
    ) -> (
        Vec<stwo::prover::Poly<stwo::prover::backend::simd::SimdBackend>>,
        stwo::prover::vcs::prover::MerkleProver<
            stwo::prover::backend::simd::SimdBackend,
            stwo::core::vcs::blake2_merkle::Blake2sMerkleHasher,
        >,
    ) {
        use stwo::core::poly::circle::CanonicCoset;
        use stwo::prover::Poly;
        use stwo::prover::backend::simd::column::BaseColumn;
        use stwo::prover::poly::circle::CircleEvaluation;
        use stwo::prover::vcs::prover::MerkleProver;

        // Reconstruct polynomials from extended evaluations
        let polynomials: Vec<Poly<stwo::prover::backend::simd::SimdBackend>> = self
            .extended_evals
            .iter()
            .zip(self.domain_log_sizes.iter())
            .map(|(data, &domain_log_size)| {
                // Allocate 64-byte aligned memory and copy data
                let aligned_data: Vec<u32> = aligned_vec_from_slice(data);

                // Convert to Vec<PackedBaseField> using bytemuck
                let packed_data: Vec<_> = bytemuck::cast_slice(&aligned_data).to_vec();

                let values = BaseColumn::from_simd(packed_data);
                let domain = CanonicCoset::new(domain_log_size).circle_domain();
                let evals = CircleEvaluation::new(domain, values);

                Poly::new(None, evals)
            })
            .collect();

        // Reconstruct MerkleProver from layers
        let merkle_prover = MerkleProver {
            layers: self.merkle_layers.clone(),
        };

        (polynomials, merkle_prover)
    }
}

/// Creates a Vec<T> with 64-byte alignment from a slice.
///
/// Required for reconstructing SIMD-compatible PackedBaseField data from
/// serialized u32 arrays.
fn aligned_vec_from_slice<T: Clone>(elements: &[T]) -> Vec<T> {
    use std::alloc::{Layout, alloc_zeroed, handle_alloc_error};
    use std::ptr::write;

    let len = elements.len();
    let elem_size = std::mem::size_of::<T>();
    let align = 64.max(std::mem::align_of::<T>());
    let layout = Layout::from_size_align(
        len.checked_mul(elem_size)
            .expect("Overflow in allocation size"),
        align,
    )
    .unwrap();
    unsafe {
        let ptr = alloc_zeroed(layout) as *mut T;
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        for (i, v) in elements.iter().enumerate() {
            write(ptr.add(i), v.clone());
        }
        Vec::from_raw_parts(ptr, len, len)
    }
}

/// Generate preprocessed data for RV32IM proving.
///
/// Performs the full preprocessing pipeline once:
/// 1. Generates constant lookup table columns
/// 2. Computes twiddles for polynomial extension
/// 3. Builds the Merkle tree commitment
/// 4. Extracts committed data (extended evals + Merkle layers)
///
/// The result can be serialized and reused across multiple prove/verify calls.
pub fn preprocess(config: PcsConfig) -> Preprocessing {
    use stwo::core::channel::Blake2sChannel;
    use stwo::core::poly::circle::CanonicCoset;
    use stwo::core::vcs::blake2_merkle::Blake2sMerkleChannel;
    use stwo::prover::CommitmentSchemeProver;
    use stwo::prover::backend::simd::SimdBackend;
    use stwo::prover::poly::circle::PolyOps;
    use tracing::{Level, span};

    // 1. Generate PreProcessedTrace (raw columns + ids)
    let span = span!(Level::INFO, "Preprocess").entered();

    let span_1 = span!(Level::INFO, "Generate trace").entered();
    let preprocessed_trace = relations::PreProcessedTrace::new();
    span_1.exit();

    // 2. Compute twiddles — need enough for largest preprocessed domain + blowup
    let span_2 = span!(Level::INFO, "Precompute twiddles").entered();
    let max_log_size = preprocessed_trace
        .trace
        .iter()
        .map(|c| c.domain.log_size())
        .max()
        .unwrap_or(0);
    let twiddles = SimdBackend::precompute_twiddles(
        CanonicCoset::new(max_log_size + 2 + config.fri_config.log_blowup_factor)
            .circle_domain()
            .half_coset,
    );
    span_2.exit();

    // 3. Capture original column log sizes before trace is consumed
    let log_sizes: Vec<u32> = preprocessed_trace
        .trace
        .iter()
        .map(|c| c.domain.log_size())
        .collect();

    // 4. Create commitment scheme and commit
    let span_3 = span!(Level::INFO, "Commit").entered();
    let channel = &mut Blake2sChannel::default();
    let mut commitment_scheme =
        CommitmentSchemeProver::<_, Blake2sMerkleChannel>::new(config, &twiddles);

    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(preprocessed_trace.trace);
    tree_builder.commit(channel);
    span_3.exit();

    // 5. Extract the committed tree data
    let span_4 = span!(Level::INFO, "Extract data").entered();
    let tree = &commitment_scheme.trees[0];

    // Extract column IDs
    let ids: Vec<String> = preprocessed_trace
        .ids
        .iter()
        .map(|id| id.id.clone())
        .collect();

    // Extract domain log sizes and extended evaluations
    let domain_log_sizes: Vec<u32> = tree
        .polynomials
        .iter()
        .map(|poly| poly.evals.domain.log_size())
        .collect();

    let extended_evals: Vec<Vec<u32>> = tree
        .polynomials
        .iter()
        .map(|poly| {
            // Cast PackedBaseField data to u32 slice (zero-copy)
            let packed_slice: &[u32] = bytemuck::cast_slice(&poly.evals.values.data);
            packed_slice.to_vec()
        })
        .collect();

    // Extract Merkle tree layers
    let merkle_layers = tree.commitment.layers.clone();

    span_4.exit();
    span.exit();

    Preprocessing {
        ids,
        log_sizes,
        domain_log_sizes,
        extended_evals,
        merkle_layers,
    }
}

// Re-export stwo types needed by external consumers
pub use stwo::core::fri::FriConfig;
pub use stwo::core::pcs::PcsConfig;

/// E2E test infrastructure (building and running guest binaries).
#[doc(hidden)]
pub mod e2e;

use serde::{Deserialize, Serialize};
use stwo::core::channel::Channel;
use stwo::core::proof::StarkProof;
use stwo::core::vcs::MerkleHasher;

use crate::components::ClaimedSum;

/// Interaction claim for LogUp (claimed sums + interaction trace log sizes).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InteractionClaim {
    pub claimed_sum: ClaimedSum,
    pub log_sizes: Vec<u32>,
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.claimed_sum.mix_into(channel);
        channel.mix_u64(self.log_sizes.len() as u64);
        for log_size in &self.log_sizes {
            channel.mix_u64(*log_size as u64);
        }
    }
}

/// RV32IM proof bundle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof<H: MerkleHasher> {
    pub claim: components::Claim,
    pub interaction_claim: InteractionClaim,
    pub public_data: PublicData,
    pub stark_proof: StarkProof<H>,
    pub interaction_pow: u64,
}
