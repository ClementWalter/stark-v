//! zkVM implementation for stark-v.
//!
//! This module provides the [`StarkV`] struct which implements the
//! [`ere_zkvm_interface::zkVM`] trait.

use crate::DEFAULT_MAX_CYCLES;
use crate::compiler::StarkVProgram;
use ere_zkvm_interface::{
    Input, InputItem, ProgramExecutionReport, ProgramProvingReport, Proof as EreProof, ProofKind,
    PublicValues, zkVM, zkVMError,
};
use prover::PcsConfig;
use std::io::Read;
use std::time::Instant;

/// stark-v zkVM instance.
///
/// Holds a compiled program and configuration, ready to execute and prove.
pub struct StarkV {
    /// Compiled ELF program.
    program: StarkVProgram,
    /// Maximum cycles before aborting.
    max_cycles: u64,
    /// PCS configuration.
    config: PcsConfig,
}

impl StarkV {
    /// Create a new stark-v instance with a compiled program.
    pub fn new(program: StarkVProgram) -> Self {
        Self {
            program,
            max_cycles: DEFAULT_MAX_CYCLES,
            config: PcsConfig::default(),
        }
    }

    /// Set the maximum cycles for execution.
    pub fn with_max_cycles(mut self, max_cycles: u64) -> Self {
        self.max_cycles = max_cycles;
        self
    }

    /// Set the PCS configuration.
    pub fn with_config(mut self, config: PcsConfig) -> Self {
        self.config = config;
        self
    }

    /// Get the ELF bytes.
    pub fn elf_bytes(&self) -> &[u8] {
        &self.program.elf_bytes
    }

    /// Convert input to raw bytes.
    ///
    /// Note: For stark-v, it's recommended to use `Input::write_bytes()` to add
    /// raw bytes directly. Object serialization via `Input::write()` is not
    /// fully supported in this implementation.
    fn input_to_bytes(input: &Input) -> Vec<u8> {
        let mut bytes = Vec::new();
        for item in input.iter() {
            match item {
                InputItem::Object(_) => {
                    // Object serialization requires erased_serde which adds complexity.
                    // For stark-v, users should prefer write_bytes() instead.
                    // Skip objects - they won't be passed to the guest.
                }
                InputItem::SerializedObject(data) => bytes.extend(data),
                InputItem::Bytes(data) => bytes.extend(data),
            }
        }
        bytes
    }
}

impl zkVM for StarkV {
    fn execute(&self, input: &Input) -> Result<(PublicValues, ProgramExecutionReport), zkVMError> {
        let input_bytes = Self::input_to_bytes(input);
        let start = Instant::now();

        let run_result =
            runner::run_with_input(&self.program.elf_bytes, &input_bytes, self.max_cycles)
                .map_err(|e| zkVMError::other(e.to_string()))?;

        let output = run_result.output.clone().unwrap_or_default();
        let cycles = run_result.cycles;
        let duration = start.elapsed();

        let report = ProgramExecutionReport {
            total_num_cycles: cycles,
            execution_duration: duration,
            ..Default::default()
        };

        Ok((output, report))
    }

    fn prove(
        &self,
        input: &Input,
        _proof_kind: ProofKind,
    ) -> Result<(PublicValues, EreProof, ProgramProvingReport), zkVMError> {
        let input_bytes = Self::input_to_bytes(input);
        let start = Instant::now();

        let run_result =
            runner::run_with_input(&self.program.elf_bytes, &input_bytes, self.max_cycles)
                .map_err(|e| zkVMError::other(e.to_string()))?;

        let output = run_result.output.clone().unwrap_or_default();

        // Generate the proof
        let _proof = prover::prove_rv32im(run_result, self.config.clone());
        let duration = start.elapsed();

        // TODO: Serialize the proof properly once proof types support Serialize
        // For now, we return an empty proof as a placeholder
        // Full proof serialization requires adding Serialize derives to:
        // - prover::Proof
        // - prover::components::Claim
        // - prover::InteractionClaim
        // - prover::PublicData
        let ere_proof = EreProof::Compressed(Vec::new());

        let report = ProgramProvingReport {
            proving_time: duration,
        };

        Ok((output, ere_proof, report))
    }

    fn verify(&self, _proof: &EreProof) -> Result<PublicValues, zkVMError> {
        // TODO: Implement proper verification once proof serialization is available
        // Currently, proof bytes are empty so we cannot verify
        Err(zkVMError::other(
            "Proof verification not yet implemented - proof serialization required",
        ))
    }

    fn name(&self) -> &'static str {
        "stark-v"
    }

    fn sdk_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn deserialize_from<R: Read, T: serde::de::DeserializeOwned>(
        &self,
        mut reader: R,
    ) -> Result<T, zkVMError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|e| zkVMError::other(e.to_string()))?;
        postcard::from_bytes(&bytes).map_err(|e| zkVMError::other(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starkv_creation() {
        let program = StarkVProgram {
            elf_bytes: vec![1, 2, 3],
        };
        let vm = StarkV::new(program);
        assert_eq!(vm.elf_bytes(), &[1, 2, 3]);
        assert_eq!(vm.max_cycles, DEFAULT_MAX_CYCLES);
    }
}
