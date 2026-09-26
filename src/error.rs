use thiserror::Error;

/// Errors that can occur when calling the TypeSafe API.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP request failure
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// The `TYPESAFE_API_KEY` environment variable is not set.
    #[error("TYPESAFE_API_KEY environment variable is not set")]
    MissingApiKey,

    /// The API returned an error response.
    #[error("api error (HTTP {status}): {message}")]
    Api {
        /// The HTTP status code of the response.
        status: reqwest::StatusCode,
        /// The response body, if any.
        message: String,
    },
}

impl Error {
    /// Returns `true` for transient errors (`429 Too Many Requests` or
    /// `529 Overloaded`) that are worth retrying after a short delay.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Api { status, .. } if status.as_u16() == 429 || status.as_u16() == 529)
    }
}
