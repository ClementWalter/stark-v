use stwo::core::verifier::VerificationError as StwoVerificationError;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum VerificationError {
    #[error("Invalid logup sum.")]
    InvalidLogupSum,
    #[error("Interaction proof of work failed.")]
    InteractionProofOfWork,
    #[error("Segment boundary mismatch between segments {prev} and {next}: {what}.")]
    SegmentChainMismatch {
        prev: usize,
        next: usize,
        what: &'static str,
    },
    #[error(transparent)]
    Stwo(#[from] StwoVerificationError),
}
