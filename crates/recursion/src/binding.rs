//! Real-proof composition binding: record the full composition check of an
//! actual stark-v proof (docs/recursion.md, M5).
//!
//! `CompositionRecorder` visits every inner component in composition order —
//! through the macro-generated `visit_components`, so the component list has
//! a single source — slicing the proof's sampled mask values exactly as
//! `FrameworkComponent::evaluate_constraint_quotients_at_point` does and
//! recording each `evaluate()` into one shared arena. The accumulated value
//! must equal the composition value the proof claims at the OODS point.

use prover::components::{ComponentVisitor, Components};
use prover::recursion::transcript::CompositionBindingData;
use stwo::core::constraints::coset_vanishing;
use stwo::core::fields::FieldExpOps;
use stwo::core::fields::qm31::{QM31, SecureField};
use stwo::core::poly::circle::CanonicCoset;
use stwo_constraint_framework::{FrameworkComponent, FrameworkEval, PREPROCESSED_TRACE_IDX};

use crate::recorder::Recorder;

/// Visitor recording every component's point evaluation into one arena.
pub struct CompositionRecorder<'a> {
    data: &'a CompositionBindingData,
    denom_inverse: SecureField,
    recorder: Option<Recorder>,
}

impl<'a> CompositionRecorder<'a> {
    pub fn new(data: &'a CompositionBindingData) -> Self {
        let denom_inverse = coset_vanishing(
            CanonicCoset::new(data.max_log_degree_bound).coset,
            data.oods_point,
        )
        .inverse();
        Self {
            data,
            denom_inverse,
            recorder: None,
        }
    }

    /// Record all components and return the finished recorder.
    pub fn record(mut self, components: &Components) -> Recorder {
        components.visit_components(&self.data.claimed_sums, &mut self);
        self.recorder.expect("at least one component")
    }
}

impl ComponentVisitor for CompositionRecorder<'_> {
    fn visit<E: FrameworkEval>(&mut self, component: &FrameworkComponent<E>, claimed_sum: QM31) {
        // Mask slicing mirrors evaluate_constraint_quotients_at_point.
        let mut mask = self
            .data
            .sampled_values
            .sub_tree(component.trace_locations());
        mask[PREPROCESSED_TRACE_IDX] = component
            .preprocessed_column_indices()
            .iter()
            .map(|idx| &self.data.sampled_values[PREPROCESSED_TRACE_IDX][*idx])
            .collect();
        let mask_owned: Vec<Vec<Vec<SecureField>>> = mask
            .iter()
            .map(|tree| tree.iter().map(|values| (*values).clone()).collect())
            .collect();

        let recorder = match self.recorder.take() {
            None => Recorder::new(
                mask_owned,
                self.data.random_coeff,
                self.denom_inverse,
                component.log_size(),
                claimed_sum,
            ),
            Some(mut recorder) => {
                recorder.col_index = vec![0; mask_owned.len()];
                recorder.mask = mask_owned;
                recorder.next_component(self.denom_inverse, component.log_size(), claimed_sum);
                recorder
            }
        };
        self.recorder = Some((**component).evaluate(recorder));
    }
}
