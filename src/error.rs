use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Dataset not found: {repo_id}")]
    NotFound { repo_id: String },
 
    #[error("Hub API error: {status} for {url}")]
    HubApi { status: u16, url: String, body: String },
 
    #[error("Rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
 
    #[error("Invalid parameter: {message}")]
    InvalidParam { message: String },

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

// This lets you write Result<T> instead of Result<T, AppError> everywhere
pub type Result<T> = std::result::Result<T, AppError>;