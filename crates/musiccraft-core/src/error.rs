use thiserror::Error;

/// Typed domain error for all MusicCraft L1 operations.
#[derive(Debug, Error)]
pub enum MusicCraftError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("project not found or unreadable: {0}")]
    Io(#[from] std::io::Error),

    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid wav format: {0}")]
    InvalidWav(String),

    #[error("track not found: {0}")]
    TrackNotFound(String),

    #[error("region not found: {0}")]
    RegionNotFound(String),

    #[error("unknown command: {0}")]
    UnknownCommand(String),
}

pub type Result<T> = std::result::Result<T, MusicCraftError>;

/// Validate a user-supplied path or identifier string against the Hostile Input Fence.
pub fn validate_non_empty_bounded(label: &str, value: &str, max_len: usize) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MusicCraftError::InvalidInput(format!(
            "{label} must not be empty"
        )));
    }
    if trimmed.len() > max_len {
        return Err(MusicCraftError::InvalidInput(format!(
            "{label} exceeds maximum length of {max_len} bytes"
        )));
    }
    Ok(())
}
