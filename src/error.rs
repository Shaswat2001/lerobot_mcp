/// Unified error type for lerobot-mcp.
///

use thiserror::Error;

/// Every fallible operation in the crate returns this error type.
/// The `#[from]` attributes generate `impl From<X> for Error`,
/// which is what makes the `?` operator work across error types.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Invalid parameter: {message}")]
    InvalidParam { message: String },

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("Hub API error: HTTP {status} for {url}")]
    HubApi {
        status: u16,
        url: String,
        body: String,
    },

    #[error("Dataset not found: {repo_id}")]
    NotFound { repo_id: String },

    #[error("Rate limited by HF Hub, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
}

/// Convenience alias so every module can write `Result<T>` instead of
/// `std::result::Result<T, crate::error::Error>`.
pub type Result<T> = std::result::Result<T, AppError>;