use stwo::core::verifier::VerificationError as StwoVerificationError;
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
