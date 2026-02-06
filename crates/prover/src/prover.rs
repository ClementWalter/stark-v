//! Main proving function for RV32IM execution traces.

use num_traits::Zero;
use stwo::core::channel::{Blake2sChannel, Channel};
use stwo::core::pcs::PcsConfig;
use stwo::core::poly::circle::CanonicCoset;
use stwo::core::proof_of_work::GrindOps;
use stwo::core::vcs::blake2_merkle::{Blake2sMerkleChannel, Blake2sMerkleHasher};
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::poly::circle::PolyOps;
use stwo::prover::{CommitmentSchemeProver, prove};
use stwo_constraint_framework::TraceLocationAllocator;
use tracing::{Level, info, span};

use crate::components::{Components, gen_interaction_trace, gen_trace};
use crate::public_data::PublicData;
use crate::relations::{INTERACTION_POW_BITS, PreProcessedTrace, Relations};
use crate::{InteractionClaim, Proof};

/// Prove execution of an RV32IM program.
///
/// Takes a `RunResult` from the runner and generates a STARK proof.
///
/// # Errors
///
/// Returns an error if:
/// - Execution cycles exceed u32::MAX
/// - Clock value overflows during computation
/// - Proof generation fails
///
/// # Panics
///
/// Panics if the logup sum is non-zero (indicating unbalanced lookups).
pub fn prove_rv32im(
    run_result: runner::RunResult,
    config: PcsConfig,
) -> Result<Proof<Blake2sMerkleHasher>, crate::ProverError> {
    let public_data = PublicData::new(&run_result)?;

    // 1. Generate traces from execution
    let span = span!(Level::INFO, "Generate traces").entered();
    let tracer = run_result.tracer;
    info!("Tracer total_traces: {}", tracer.total_traces());
    let traces = gen_trace(tracer);
    let log_size = traces.max_log_size();
    info!("Max trace log_size: {log_size}");
    span.exit();

    // 2. Precompute twiddles (need enough for largest domain + blowup)
    let span = span!(Level::INFO, "Precompute twiddles").entered();
    let twiddles = SimdBackend::precompute_twiddles(
        // See https://github.com/starkware-libs/stwo-cairo/blob/main/stwo_cairo_prover/crates/prover/src/prover.rs#L46-L47
        CanonicCoset::new(log_size + 2 + config.fri_config.log_blowup_factor)
            .circle_domain()
            .half_coset,
    );
    span.exit();

    // 3. Setup protocol
    let channel = &mut Blake2sChannel::default();
    let mut commitment_scheme =
        CommitmentSchemeProver::<_, Blake2sMerkleChannel>::new(config, &twiddles);

    // 4. Public data
    public_data.mix_into(channel);

    // 5. Preprocessed trace (constant lookup tables - fixed size, independent of execution)
    let span = span!(Level::INFO, "Preprocessed trace").entered();
    let preprocessed_trace = PreProcessedTrace::new();
    let preprocessed_ids = preprocessed_trace.ids.clone();
    info!("Preprocessed trace ids len: {}", preprocessed_ids.len());

    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(preprocessed_trace.trace);
    tree_builder.commit(channel);
    span.exit();

    // 6. Main execution trace (opcode + multiplicity columns)
    let span = span!(Level::INFO, "Main trace").entered();
    let claim: crate::components::Claim = (&traces).into();
    let columns = traces.columns_cloned();
    info!("Main trace columns committed: {}", columns.len());

    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(columns);
    tree_builder.commit(channel);
    span.exit();

    // 7. Mix claim into channel
    claim.mix_into(channel);

    // 8. Proof of work before drawing lookup elements
    info!("proof of work with {} bits", INTERACTION_POW_BITS);
    let interaction_pow = SimdBackend::grind(channel, INTERACTION_POW_BITS);
    channel.mix_u64(interaction_pow);

    // 9. Draw lookup elements
    let relations = Relations::draw(channel);
    let public_logup_sum = public_data.logup_sum(&relations);

    // 10. Interaction trace (LogUp fractions) - only commit if non-empty
    let span = span!(Level::INFO, "Interaction trace").entered();
    let (interaction_trace, claimed_sum) = gen_interaction_trace(&traces, &relations);
    let interaction_log_sizes = interaction_trace
        .iter()
        .map(|col| col.domain.log_size())
        .collect::<Vec<_>>();
    let interaction_claim = InteractionClaim {
        claimed_sum,
        log_sizes: interaction_log_sizes,
    };
    interaction_claim.mix_into(channel);
    if !interaction_trace.is_empty() {
        let mut tree_builder = commitment_scheme.tree_builder();
        tree_builder.extend_evals(interaction_trace);
        tree_builder.commit(channel);
    }
    span.exit();

    // 11. Create components
    let span = span!(Level::INFO, "Create components").entered();
    let mut location_allocator =
        TraceLocationAllocator::new_with_preprocessed_columns(&preprocessed_ids);
    let components = Components::new(
        &claim,
        &mut location_allocator,
        relations,
        &interaction_claim.claimed_sum,
    );
    span.exit();

    info!(
        "Trace log degree bounds: {:?}",
        components.trace_log_degree_bounds()
    );

    // 12. Verify claimed sum is zero (all lookups balanced)
    {
        let total_sum = interaction_claim.claimed_sum.total() + public_logup_sum;
        info!("Claimed sum: {total_sum:?}");
        if !total_sum.is_zero() {
            let preprocessed_trace = PreProcessedTrace::new();
            info!(
                "Relation summary: {:?}",
                components.track_relations(&preprocessed_trace.trace, &traces)
            );
            panic!("Relation sum must be zero, got {total_sum:?}");
        }
    }

    // 13. Generate proof
    let span = span!(Level::INFO, "Prove").entered();
    let proof = prove(&components.provers(), channel, commitment_scheme)?;
    span.exit();

    Ok(Proof {
        claim,
        interaction_claim,
        public_data,
        stark_proof: proof,
        interaction_pow,
    })
}
