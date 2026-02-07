use stwo::core::verifier::VerificationError as StwoVerificationError;
use stwo::prover::ProvingError as StwoProvingError;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum VerificationError {
    #[error("Invalid logup sum.")]
    InvalidLogupSum,
    #[error("Interaction proof of work failed.")]
    InteractionProofOfWork,
    #[error(transparent)]
    Stwo(#[from] StwoVerificationError),
}

#[derive(Debug, Error)]
pub enum ProverError {
    #[error("Failed to build guest binaries")]
    GuestBuildFailed,
    #[error("Failed to execute cargo build: {0}")]
    GuestBuildCommand(#[source] std::io::Error),
    #[error("Empty trace after generation")]
    EmptyTrace,
    #[error("Cycles overflow u32: {0}")]
    CyclesOverflow(u64),
    #[error("Clock overflow when computing final clock")]
    ClockOverflow,
    #[error("Proof generation failed: {0}")]
    ProofGeneration(#[from] StwoProvingError),
}
