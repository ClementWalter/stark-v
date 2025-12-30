//! Witness generation for trace tables.

use simd::AlignedVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::poly::circle::CanonicCoset;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;

use crate::relations::Counters;

pub type Trace = Vec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>;

/// Extension trait for converting trace tables to witness columns.
pub trait IntoWitness: Sized {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn into_columns(self) -> Vec<AlignedVec<u32>>;

    /// Convert table to trace columns, padding to power of 2.
    fn into_witness(self, _counters: &mut Counters) -> Trace {
        let len = self.len() as u32;
        let log_size = len.next_power_of_two().ilog2().max(4);
        let padded_len = 1 << log_size;
        let columns = self.into_columns();
        let domain = CanonicCoset::new(log_size).circle_domain();

        columns
            .into_iter()
            .map(|mut col| {
                col.resize(padded_len, 0);
                let base_col: BaseColumn = col.into();
                CircleEvaluation::new(domain, base_col)
            })
            .collect()
    }
}

// Implement for all table types
macro_rules! impl_into_witness {
    ($($table:ident),* $(,)?) => {
        $(
            impl IntoWitness for runner::trace::$table {
                fn len(&self) -> usize { runner::trace::$table::len(self) }
                fn into_columns(self) -> Vec<AlignedVec<u32>> { runner::trace::$table::into_columns(self) }
            }
        )*
    };
}

impl_into_witness!(
    BaseAluRegTable,
    BaseAluImmTable,
    ShiftsRegTable,
    ShiftsImmTable,
    LtRegTable,
    LtImmTable,
    BranchEqTable,
    BranchLtTable,
    LuiTable,
    AuipcTable,
    JalrTable,
    JalTable,
    LoadStoreTable,
    MulTable,
    MulhTable,
    DivTable,
);
