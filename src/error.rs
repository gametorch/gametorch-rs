use reqwest::StatusCode;

/// The result type returned by every SDK operation.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the GameTorch SDK.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The HTTP transport failed (DNS, TLS, connection, timeout, body decoding).
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// The API returned a non-success status with a JSON `{"error": ...}` body.
    #[error("GameTorch API error ({status}): {message}")]
    Api {
        /// HTTP status code returned by the API.
        status: StatusCode,
        /// Human readable message from the API's `error` field.
        message: String,
        /// Correlation id echoed by the API, when present.
        request_id: Option<String>,
    },

    /// The API kept returning `429` (or another retryable status) after the
    /// configured number of retries.
    #[error("rate limited by GameTorch after {attempts} attempt(s)")]
    RateLimited {
        /// Number of attempts that were made.
        attempts: u32,
    },

    /// The client was misconfigured, for example no credentials were supplied.
    #[error("invalid client configuration: {0}")]
    Config(String),

    /// The supplied base URL could not be parsed.
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),

    /// A response could not be deserialized, or the body was not valid JSON.
    #[error("failed to decode response: {0}")]
    Decode(#[from] serde_json::Error),
}

impl Error {
    /// Builds an [`Error::Api`] from a status and message.
    pub(crate) fn api(
        status: StatusCode,
        message: impl Into<String>,
        request_id: Option<String>,
    ) -> Self {
        Error::Api {
            status,
            message: message.into(),
            request_id,
        }
    }

    /// The HTTP status code, when this error originated from an API response.
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Error::Api { status, .. } => Some(*status),
            Error::Http(e) => e.status(),
            _ => None,
        }
    }

    /// The API's error message, when this error originated from an API response.
    pub fn api_message(&self) -> Option<&str> {
        match self {
            Error::Api { message, .. } => Some(message),
            _ => None,
        }
    }

    /// Returns `true` for `404 Not Found` responses.
    pub fn is_not_found(&self) -> bool {
        self.status() == Some(StatusCode::NOT_FOUND)
    }

    /// Returns `true` for `401 Unauthorized` responses.
    pub fn is_unauthorized(&self) -> bool {
        self.status() == Some(StatusCode::UNAUTHORIZED)
    }

    /// Returns `true` for `402 Payment Required` responses, which GameTorch uses
    /// for insufficient credits or a spending/entitlement limit.
    pub fn is_payment_required(&self) -> bool {
        self.status() == Some(StatusCode::PAYMENT_REQUIRED)
    }

    /// Returns `true` for `403 Forbidden` responses.
    pub fn is_forbidden(&self) -> bool {
        self.status() == Some(StatusCode::FORBIDDEN)
    }

    /// Returns `true` for `409 Conflict` responses, typically an idempotency
    /// `request_id` reused with a different body.
    pub fn is_conflict(&self) -> bool {
        self.status() == Some(StatusCode::CONFLICT)
    }

    /// Returns `true` for `429 Too Many Requests` responses.
    pub fn is_rate_limited(&self) -> bool {
        self.status() == Some(StatusCode::TOO_MANY_REQUESTS)
    }
}

impl Error {
    /// A short, stable label for the error kind, useful in logs.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Http(_) => "http",
            Error::Api { .. } => "api",
            Error::RateLimited { .. } => "rate_limited",
            Error::Config(_) => "config",
            Error::InvalidBaseUrl(_) => "invalid_base_url",
            Error::Decode(_) => "decode",
        }
    }
}
