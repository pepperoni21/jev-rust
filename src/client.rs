use crate::error::Error;
use crate::response::EvaluateResponse;
use crate::types::EvaluateRequest;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://api.typesafe.ai";
const DEFAULT_MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 500;
const MAX_BACKOFF_MS: u64 = 8_000;

/// Controls how transient `429 Too Many Requests` and `529 Overloaded`
/// responses are retried
///
/// By default, responses are retried with exponential backoff
/// Use [`TypeSafe::retry`] or [`TypeSafe::no_retries`] to change this
#[derive(Debug, Clone, Copy)]
pub enum RetryPolicy {
    /// Never retry. Transient errors are returned immediately
    None,
    /// Retry up to `max_retries` times with exponential backoff
    Exponential { max_retries: u32 },
}

impl RetryPolicy {
    /// A policy that never retries
    pub const fn none() -> Self {
        RetryPolicy::None
    }

    /// A policy that retries up to `max_retries` times.
    pub const fn exponential(max_retries: u32) -> Self {
        RetryPolicy::Exponential { max_retries }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy::exponential(DEFAULT_MAX_RETRIES)
    }
}

/// A client for the TypeSafe evaluation API.
///
/// Create one with [`TypeSafe::new`] and reuse it across requests. The client
/// is cheap to clone (it wraps an [`reqwest::Client`] internally).
#[derive(Debug, Clone)]
pub struct TypeSafe {
    api_key: String,
    base_url: String,
    http: reqwest::Client,
    retry_policy: RetryPolicy,
}

impl TypeSafe {
    /// Create a client with the given API key and default settings.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_client(api_key, reqwest::Client::new())
    }

    /// Create a client with a pre-configured [`reqwest::Client`] (e.g. for
    /// custom timeouts or proxies).
    pub fn with_client(api_key: impl Into<String>, http: reqwest::Client) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            http,
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Create a client using the `TYPESAFE_API_KEY` environment variable.
    pub fn from_env() -> Result<Self, Error> {
        let key = std::env::var("TYPESAFE_API_KEY").map_err(|_| Error::MissingApiKey)?;
        Ok(Self::new(key))
    }

    /// Override the base URL (e.g. for a proxy or self-hosted endpoint).
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set the retry policy for transient `429`/`529` responses.
    ///
    /// The default is [`RetryPolicy::Exponential`] with 3 retries.
    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Disable retries entirely. Equivalent to [`TypeSafe::retry`] with
    /// [`RetryPolicy::None`].
    pub fn no_retries(mut self) -> Self {
        self.retry_policy = RetryPolicy::None;
        self
    }

    /// Evaluate an [`EvaluateRequest`] and return the structured answers.
    ///
    /// Transient `429 Too Many Requests` and `529 Overloaded` responses are
    /// retried with exponential backoff according to the configured
    /// [`RetryPolicy`].
    pub async fn evaluate(&self, request: EvaluateRequest) -> Result<EvaluateResponse, Error> {
        let url = format!("{}/v1/systemone", self.base_url.trim_end_matches('/'));
        let max_retries = match self.retry_policy {
            RetryPolicy::None => 0,
            RetryPolicy::Exponential { max_retries } => max_retries,
        };
        let mut attempts = 0u32;

        loop {
            let response = self
                .http
                .post(&url)
                .bearer_auth(self.api_key.as_str())
                .json(&request)
                .send()
                .await?;

            let status = response.status();

            if status.is_success() {
                return Ok(response.json::<EvaluateResponse>().await?);
            }

            let retryable = status.as_u16() == 429 || status.as_u16() == 529;
            if retryable && attempts < max_retries {
                attempts += 1;
                let backoff = INITIAL_BACKOFF_MS
                    .saturating_mul(2u64.saturating_pow(attempts - 1))
                    .min(MAX_BACKOFF_MS);
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                continue;
            }

            let message = response.text().await.unwrap_or_default();
            return Err(Error::Api { status, message });
        }
    }
}
