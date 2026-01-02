//! Debug utilities for converting trace data to Polars DataFrames.
//!
//! Provides a `ToDataFrame` trait for converting various trace types to readable DataFrames.
//!
//! # Display Configuration
//!
//! Use [`set_display_options`] to configure max rows/columns shown:
//! ```ignore
//! debug_utils::set_display_options(100, 20); // 100 rows, 20 columns
//! ```

use polars::prelude::{Column as PolarsColumn, IntoColumn, NamedFrom, Series};
pub use polars::prelude::DataFrame;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::{BaseField, M31, P};
use stwo::prover::backend::Column as StwoColumn; // Used for .to_cpu() method
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::m31::PackedM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;

/// Half of the prime modulus, used for centering M31 values.
const HALF_P: i32 = (P / 2) as i32;

/// Set the maximum rows and columns displayed when printing DataFrames.
///
/// # Arguments
/// * `max_rows` - Maximum number of rows to display (use `None` for unlimited)
/// * `max_cols` - Maximum number of columns to display (use `None` for unlimited)
pub fn set_display_options(max_rows: Option<usize>, max_cols: Option<usize>) {
    // Set table formatting options via environment
    if let Some(rows) = max_rows {
        std::env::set_var("POLARS_FMT_MAX_ROWS", rows.to_string());
    }
    if let Some(cols) = max_cols {
        std::env::set_var("POLARS_FMT_MAX_COLS", cols.to_string());
    }
}

/// Convert an M31 field element to a centered i32 in range (-P/2, P/2).
#[inline]
pub fn m31_to_centered_i32(m: M31) -> i32 {
    let v = m.0 as i32;
    if v > HALF_P { v - P as i32 } else { v }
}

/// Create a DataFrame from parallel slices of u32 with column names.
///
/// This is a convenient helper for building DataFrames from columnar trace data.
///
/// # Example
/// ```ignore
/// let df = slices_to_df(&[
///     ("clk", &clk_data[..]),
///     ("pc", &pc_data[..]),
/// ]);
/// ```
pub fn slices_to_df(columns: &[(&str, &[u32])]) -> DataFrame {
    let cols: Vec<PolarsColumn> = columns
        .iter()
        .map(|(name, data)| Series::new((*name).into(), *data).into_column())
        .collect();
    DataFrame::new(cols).expect("Failed to create DataFrame")
}

/// Trait for converting trace data to Polars DataFrames.
pub trait ToDataFrame {
    /// Convert to a DataFrame with auto-generated column names (col_0, col_1, ...).
    fn to_df(&self) -> DataFrame;

    /// Convert to a DataFrame with custom column names.
    /// If fewer names are provided than columns, remaining columns use auto-generated names.
    fn to_df_named(&self, names: &[&str]) -> DataFrame;
}

// Helper to get column name
fn col_name(names: &[&str], idx: usize) -> String {
    names
        .get(idx)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("col_{idx}"))
}

// =============================================================================
// Vec<Vec<u32>> - Multi-column DataFrame
// =============================================================================

impl ToDataFrame for Vec<Vec<u32>> {
    fn to_df(&self) -> DataFrame {
        self.to_df_named(&[])
    }

    fn to_df_named(&self, names: &[&str]) -> DataFrame {
        let columns: Vec<PolarsColumn> = self
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let name = col_name(names, i);
                Series::new(name.into(), col.as_slice()).into_column()
            })
            .collect();

        DataFrame::new(columns).expect("Failed to create DataFrame")
    }
}

// =============================================================================
// Vec<Vec<M31>> - Multi-column DataFrame with centered i32 values
// =============================================================================

impl ToDataFrame for Vec<Vec<M31>> {
    fn to_df(&self) -> DataFrame {
        self.to_df_named(&[])
    }

    fn to_df_named(&self, names: &[&str]) -> DataFrame {
        let columns: Vec<PolarsColumn> = self
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let name = col_name(names, i);
                let values: Vec<i32> = col.iter().map(|&m| m31_to_centered_i32(m)).collect();
                Series::new(name.into(), values).into_column()
            })
            .collect();

        DataFrame::new(columns).expect("Failed to create DataFrame")
    }
}

// =============================================================================
// Vec<u32> - Single column DataFrame
// =============================================================================

impl ToDataFrame for Vec<u32> {
    fn to_df(&self) -> DataFrame {
        self.to_df_named(&[])
    }

    fn to_df_named(&self, names: &[&str]) -> DataFrame {
        let name = col_name(names, 0);
        let col = Series::new(name.into(), self.as_slice()).into_column();
        DataFrame::new(vec![col]).expect("Failed to create DataFrame")
    }
}

// =============================================================================
// Vec<M31> - Single column DataFrame with centered i32 values
// =============================================================================

impl ToDataFrame for Vec<M31> {
    fn to_df(&self) -> DataFrame {
        self.to_df_named(&[])
    }

    fn to_df_named(&self, names: &[&str]) -> DataFrame {
        let name = col_name(names, 0);
        let values: Vec<i32> = self.iter().map(|&m| m31_to_centered_i32(m)).collect();
        let col = Series::new(name.into(), values).into_column();
        DataFrame::new(vec![col]).expect("Failed to create DataFrame")
    }
}

// =============================================================================
// Vec<PackedM31> - Single column DataFrame (flattened, centered i32)
// =============================================================================

impl ToDataFrame for Vec<PackedM31> {
    fn to_df(&self) -> DataFrame {
        self.to_df_named(&[])
    }

    fn to_df_named(&self, names: &[&str]) -> DataFrame {
        let name = col_name(names, 0);
        let values: Vec<i32> = self
            .iter()
            .flat_map(|packed| packed.to_array())
            .map(m31_to_centered_i32)
            .collect();
        let col = Series::new(name.into(), values).into_column();
        DataFrame::new(vec![col]).expect("Failed to create DataFrame")
    }
}

// =============================================================================
// Vec<Vec<PackedM31>> - Multi-column DataFrame (each inner vec flattened)
// =============================================================================

impl ToDataFrame for Vec<Vec<PackedM31>> {
    fn to_df(&self) -> DataFrame {
        self.to_df_named(&[])
    }

    fn to_df_named(&self, names: &[&str]) -> DataFrame {
        let columns: Vec<PolarsColumn> = self
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let name = col_name(names, i);
                let values: Vec<i32> = col
                    .iter()
                    .flat_map(|packed| packed.to_array())
                    .map(m31_to_centered_i32)
                    .collect();
                Series::new(name.into(), values).into_column()
            })
            .collect();

        DataFrame::new(columns).expect("Failed to create DataFrame")
    }
}

// =============================================================================
// ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>
// =============================================================================

impl ToDataFrame for ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
    fn to_df(&self) -> DataFrame {
        self.to_df_named(&[])
    }

    fn to_df_named(&self, names: &[&str]) -> DataFrame {
        let columns: Vec<PolarsColumn> = self
            .iter()
            .enumerate()
            .map(|(i, eval)| {
                let name = col_name(names, i);
                let values: Vec<i32> = eval
                    .values
                    .to_cpu()
                    .iter()
                    .map(|&m| m31_to_centered_i32(m))
                    .collect();
                Series::new(name.into(), values).into_column()
            })
            .collect();

        DataFrame::new(columns).expect("Failed to create DataFrame")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_u32() {
        let data: Vec<u32> = vec![1, 2, 3, 4, 5];
        let df = data.to_df();
        assert_eq!(df.shape(), (5, 1));
    }

    #[test]
    fn test_vec_vec_u32() {
        let data: Vec<Vec<u32>> = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let df = data.to_df_named(&["a", "b", "c"]);
        assert_eq!(df.shape(), (3, 3));
    }

    #[test]
    fn test_vec_m31_centered() {
        // Test that values > P/2 become negative
        let half_p_plus_1 = M31::from(HALF_P as u32 + 1);
        let data: Vec<M31> = vec![M31::from(0), M31::from(1), half_p_plus_1];
        let df = data.to_df_named(&["val"]);
        // The third value should be negative
        let col = df.column("val").unwrap();
        let vals: Vec<i32> = col.i32().unwrap().into_no_null_iter().collect();
        assert_eq!(vals[0], 0);
        assert_eq!(vals[1], 1);
        assert!(vals[2] < 0);
    }

    #[test]
    fn test_vec_packed_m31() {
        // Create two PackedM31 (each with 16 lanes)
        let packed1 = PackedM31::from_array(std::array::from_fn(|i| M31::from(i as u32)));
        let packed2 = PackedM31::from_array(std::array::from_fn(|i| M31::from((i + 16) as u32)));
        let data: Vec<PackedM31> = vec![packed1, packed2];

        let df = data.to_df_named(&["vals"]);

        // Should have 32 rows (2 packed * 16 lanes)
        assert_eq!(df.shape(), (32, 1));

        // Check values are flattened correctly
        let col = df.column("vals").unwrap();
        let vals: Vec<i32> = col.i32().unwrap().into_no_null_iter().collect();
        assert_eq!(vals[0], 0);
        assert_eq!(vals[15], 15);
        assert_eq!(vals[16], 16);
        assert_eq!(vals[31], 31);
    }

    #[test]
    fn test_vec_vec_packed_m31() {
        // Create two columns, each with 2 PackedM31 (32 elements per column)
        let col1 = vec![
            PackedM31::from_array(std::array::from_fn(|i| M31::from(i as u32))),
            PackedM31::from_array(std::array::from_fn(|i| M31::from((i + 16) as u32))),
        ];
        let col2 = vec![
            PackedM31::from_array(std::array::from_fn(|i| M31::from((i + 100) as u32))),
            PackedM31::from_array(std::array::from_fn(|i| M31::from((i + 116) as u32))),
        ];
        let data: Vec<Vec<PackedM31>> = vec![col1, col2];

        let df = data.to_df_named(&["a", "b"]);

        // Should have 32 rows and 2 columns
        assert_eq!(df.shape(), (32, 2));

        // Check first column values
        let col_a = df.column("a").unwrap();
        let vals_a: Vec<i32> = col_a.i32().unwrap().into_no_null_iter().collect();
        assert_eq!(vals_a[0], 0);
        assert_eq!(vals_a[31], 31);

        // Check second column values
        let col_b = df.column("b").unwrap();
        let vals_b: Vec<i32> = col_b.i32().unwrap().into_no_null_iter().collect();
        assert_eq!(vals_b[0], 100);
        assert_eq!(vals_b[31], 131);
    }

    #[test]
    fn test_column_vec_circle_evaluation() {
        use stwo::core::poly::circle::CanonicCoset;

        // Create a small domain (size 16 = 2^4)
        let log_size = 4u32;
        let domain = CanonicCoset::new(log_size).circle_domain();

        // Create two columns of evaluations
        let col1_values: Vec<M31> = (0..16).map(M31::from).collect();
        let col2_values: Vec<M31> = (100..116).map(M31::from).collect();

        let eval1 = CircleEvaluation::<SimdBackend, BaseField, BitReversedOrder>::new(
            domain,
            col1_values.into_iter().collect(),
        );
        let eval2 = CircleEvaluation::<SimdBackend, BaseField, BitReversedOrder>::new(
            domain,
            col2_values.into_iter().collect(),
        );

        let traces: ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> =
            vec![eval1, eval2];

        let df = traces.to_df_named(&["col_a", "col_b"]);

        // Should have 16 rows and 2 columns
        assert_eq!(df.shape(), (16, 2));

        // Check column names
        assert!(df.column("col_a").is_ok());
        assert!(df.column("col_b").is_ok());
    }
}
