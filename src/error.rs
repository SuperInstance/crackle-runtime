use std::fmt;

/// Errors that can occur during crackle runtime operations.
#[derive(Debug)]
pub enum CrackleError {
    /// A task panicked during firing.
    TaskPanicked(String),
    /// The kiln was already cooled and cannot accept new tasks.
    KilnCooled,
    /// Invalid thermal profile configuration.
    InvalidProfile(String),
    /// Pattern detection failed.
    DetectionFailed(String),
}

impl fmt::Display for CrackleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrackleError::TaskPanicked(msg) => write!(f, "task panicked: {msg}"),
            CrackleError::KilnCooled => write!(f, "kiln has already cooled and cannot accept new tasks"),
            CrackleError::InvalidProfile(msg) => write!(f, "invalid thermal profile: {msg}"),
            CrackleError::DetectionFailed(msg) => write!(f, "pattern detection failed: {msg}"),
        }
    }
}

impl std::error::Error for CrackleError {}

/// A specialized result type for crackle runtime operations.
pub type Result<T> = std::result::Result<T, CrackleError>;
