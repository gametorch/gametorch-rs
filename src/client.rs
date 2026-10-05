//! The GameTorch [`Client`] and its builder.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{Error, Result};
use crate::rate_limit::{Concurrency, RateClass, RateLimiter};
use crate::types::{Download, OkResponse};

/// The production GameTorch API base URL.
pub const DEFAULT_BASE_URL: &str = "https://gametorch.app/api";

/// Environment variable read by [`ClientBuilder::from_env`] for the API key or
/// bearer token.
pub const ENV_API_KEY: &str = "GAMETORCH_API_KEY";

/// Environment variable read by [`ClientBuilder::from_env`] for the base URL.
pub const ENV_BASE_URL: &str = "GAMETORCH_BASE_URL";

/// Credentials used to authenticate requests.
#[derive(Clone)]
pub(crate) enum Auth {
    /// A server-to-server API key (`gt2_...`).
    ApiKey(Arc<str>),
    /// A Clerk session/bearer token.
    Bearer(Arc<str>),
}

impl Auth {
    fn token(&self) -> &str {
        match self {
            Auth::ApiKey(token) | Auth::Bearer(token) => token,
        }
    }
}

impl fmt::Debug for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Auth::ApiKey(_) => f.write_str("Auth::ApiKey(redacted)"),
            Auth::Bearer(_) => f.write_str("Auth::Bearer(redacted)"),
        }
    }
}

/// An async client for the GameTorch API.
///
/// The client is cheap to clone; clones share the underlying connection pool
/// and rate limiter, so a single client is enough for the whole application.
///
/// # Examples
///
/// ```no_run
/// # async fn run() -> gametorch::Result<()> {
/// let client = gametorch::Client::builder()
///     .api_key("gt2_your_key")
///     .build()?;
///
/// let models = client.sprite_models().await?;
/// println!("{} image models available", models.image_models.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: Url,
    pub(crate) auth: Option<Auth>,
    pub(crate) limiter: Arc<RateLimiter>,
    pub(crate) max_retries: u32,
    pub(crate) retry_base_delay: Duration,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url.as_str())
            .field("auth", &self.auth)
            .field("max_retries", &self.max_retries)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Starts building a new client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Builds a client from the `GAMETORCH_API_KEY` (and optional
    /// `GAMETORCH_BASE_URL`) environment variables.
    pub fn from_env() -> Result<Client> {
        ClientBuilder::default().from_env().build()
    }

    /// The base URL this client talks to, including the `/api` suffix.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Whether the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.auth.is_some()
    }

    pub(crate) fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .expect("path is a valid relative URL");

        let mut request = self.http.request(method, url);
        if let Some(auth) = &self.auth {
            request = request.bearer_auth(auth.token());
        }
        request
    }

    /// Sends a request with local rate limiting, concurrency caps and retries,
    /// returning the raw response on success.
    pub(crate) async fn send<F>(
        &self,
        class: RateClass,
        route: &'static str,
        concurrency: Concurrency,
        build: F,
    ) -> Result<Response>
    where
        F: Fn() -> RequestBuilder,
    {
        let mut attempt: u32 = 0;

        loop {
            self.limiter.acquire(class, route).await;
            let _permits = self.limiter.acquire_concurrency(concurrency).await;

            tracing::debug!(route, attempt, "sending GameTorch request");
            let outcome = build().send().await;

            match outcome {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) => {
                    let status = response.status();
                    if is_retryable_status(status) && attempt < self.max_retries {
                        let delay = retry_delay(&response, attempt, self.retry_base_delay);
                        attempt += 1;
                        tracing::debug!(
                            route,
                            attempt,
                            status = status.as_u16(),
                            delay_ms = delay.as_millis() as u64,
                            "retrying after retryable response"
                        );
                        drop(_permits);
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    if status == StatusCode::TOO_MANY_REQUESTS {
                        return Err(Error::RateLimited {
                            attempts: attempt + 1,
                        });
                    }
                    return Err(parse_api_error(response).await);
                }
                Err(err) => {
                    if is_retryable_transport(&err) && attempt < self.max_retries {
                        let delay = backoff(attempt, self.retry_base_delay);
                        attempt += 1;
                        tracing::debug!(
                            route,
                            attempt,
                            delay_ms = delay.as_millis() as u64,
                            "retrying after transport error"
                        );
                        drop(_permits);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(err.into());
                }
            }
        }
    }

    /// Sends a request and decodes a JSON response body into `T`.
    pub(crate) async fn send_json<T, F>(
        &self,
        class: RateClass,
        route: &'static str,
        concurrency: Concurrency,
        build: F,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        F: Fn() -> RequestBuilder,
    {
        let response = self.send(class, route, concurrency, build).await?;
        let bytes = response.bytes().await?;
        serde_json::from_slice(&bytes).map_err(Error::Decode)
    }

    /// Sends a request and returns the raw bytes plus content type.
    pub(crate) async fn send_download<F>(
        &self,
        class: RateClass,
        route: &'static str,
        concurrency: Concurrency,
        build: F,
    ) -> Result<Download>
    where
        F: Fn() -> RequestBuilder,
    {
        let response = self.send(class, route, concurrency, build).await?;
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let data = response.bytes().await?;
        Ok(Download { content_type, data })
    }

    /// Sends a request and discards the body, returning `()`.
    pub(crate) async fn send_ok<F>(
        &self,
        class: RateClass,
        route: &'static str,
        concurrency: Concurrency,
        build: F,
    ) -> Result<()>
    where
        F: Fn() -> RequestBuilder,
    {
        let response = self.send(class, route, concurrency, build).await?;
        let _ = response.bytes().await?;
        Ok(())
    }

    /// Convenience wrapper for endpoints that return `{"ok": true}`.
    pub(crate) async fn send_ack<F>(
        &self,
        class: RateClass,
        route: &'static str,
        concurrency: Concurrency,
        build: F,
    ) -> Result<OkResponse>
    where
        F: Fn() -> RequestBuilder,
    {
        self.send_json(class, route, concurrency, build).await
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::REQUEST_TIMEOUT
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn is_retryable_transport(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

fn retry_delay(response: &Response, attempt: u32, base: Duration) -> Duration {
    if let Some(delay) = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_retry_after)
    {
        return delay;
    }
    backoff(attempt, base)
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    if let Ok(seconds) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    // HTTP-date form: not worth pulling in an extra dependency, fall back to
    // the caller's exponential backoff.
    None
}

fn backoff(attempt: u32, base: Duration) -> Duration {
    let exponential = base
        .checked_mul(1u32 << attempt.min(6))
        .unwrap_or(Duration::from_secs(30))
        .min(Duration::from_secs(30));

    // Add up to 250ms of jitter so concurrent clients don't retry in lockstep.
    let jitter = Duration::from_millis(u64::from(uuid::Uuid::new_v4().as_bytes()[0]) % 250);
    exponential + jitter
}

async fn parse_api_error(response: Response) -> Error {
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .or_else(|| response.headers().get("request-id"))
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let body = response.bytes().await.unwrap_or_default();
    let message = serde_json::from_slice::<ApiErrorBody>(&body)
        .map(|body| body.error)
        .unwrap_or_else(|_| {
            let text = String::from_utf8_lossy(&body);
            let text = text.trim();
            if text.is_empty() {
                status
                    .canonical_reason()
                    .unwrap_or("unknown error")
                    .to_string()
            } else {
                text.to_string()
            }
        });

    Error::api(status, message, request_id)
}

#[derive(serde::Deserialize)]
struct ApiErrorBody {
    error: String,
}

/// Configures and builds a [`Client`].
///
/// # Examples
///
/// ```no_run
/// # async fn run() -> gametorch::Result<()> {
/// let client = gametorch::Client::builder()
///     .api_key(std::env::var("GAMETORCH_API_KEY").unwrap())
///     .timeout(std::time::Duration::from_secs(60))
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    base_url: Option<String>,
    auth: Option<Auth>,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    max_retries: u32,
    retry_base_delay: Duration,
    rate_limit: bool,
    user_agent: String,
    default_headers: HeaderMap,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        ClientBuilder {
            base_url: None,
            auth: None,
            timeout: Some(Duration::from_secs(120)),
            connect_timeout: Some(Duration::from_secs(15)),
            max_retries: 3,
            retry_base_delay: Duration::from_millis(500),
            rate_limit: true,
            user_agent: format!("gametorch-rust/{}", env!("CARGO_PKG_VERSION")),
            default_headers: HeaderMap::new(),
        }
    }
}

impl ClientBuilder {
    /// Sets a `gt2_...` API key used for server-to-server authentication.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.auth = Some(Auth::ApiKey(Arc::from(api_key.into())));
        self
    }

    /// Sets a Clerk session/bearer token.
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(Auth::Bearer(Arc::from(token.into())));
        self
    }

    /// Overrides the base URL. Defaults to [`DEFAULT_BASE_URL`]. For local
    /// development use `http://localhost:8300/api`.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Sets the total request timeout (default 120s).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Disables the total request timeout.
    pub fn no_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    /// Sets the connection timeout (default 15s).
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Maximum number of automatic retries for retryable failures (default 3).
    /// Set to `0` to disable retries.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Base delay for exponential backoff between retries (default 500ms).
    pub fn retry_base_delay(mut self, delay: Duration) -> Self {
        self.retry_base_delay = delay;
        self
    }

    /// Enables or disables the built-in client-side rate limiting and
    /// concurrency caps (enabled by default).
    pub fn rate_limit(mut self, enabled: bool) -> Self {
        self.rate_limit = enabled;
        self
    }

    /// Overrides the `User-Agent` header.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Adds a default header sent with every request.
    pub fn default_header(mut self, name: reqwest::header::HeaderName, value: HeaderValue) -> Self {
        self.default_headers.insert(name, value);
        self
    }

    /// Populates the builder from environment variables:
    /// `GAMETORCH_API_KEY` and `GAMETORCH_BASE_URL`.
    pub fn from_env(mut self) -> Self {
        if let Ok(api_key) = std::env::var(ENV_API_KEY) {
            if !api_key.is_empty() {
                self.auth = Some(Auth::ApiKey(Arc::from(api_key)));
            }
        }
        if let Ok(base_url) = std::env::var(ENV_BASE_URL) {
            if !base_url.is_empty() {
                self.base_url = Some(base_url);
            }
        }
        self
    }

    /// Builds the client.
    pub fn build(self) -> Result<Client> {
        let base_url = self
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let mut base_url = Url::parse(&base_url)?;
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path().trim_end_matches('/'));
            base_url.set_path(&path);
        }

        let mut headers = self.default_headers;
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&self.user_agent)
                .map_err(|e| Error::Config(format!("invalid User-Agent: {e}")))?,
        );

        let mut builder = reqwest::Client::builder()
            .default_headers(headers)
            .pool_max_idle_per_host(16);

        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(connect_timeout) = self.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }

        let http = builder.build()?;

        Ok(Client {
            http,
            base_url,
            auth: self.auth,
            limiter: Arc::new(RateLimiter::new(self.rate_limit)),
            max_retries: self.max_retries,
            retry_base_delay: self.retry_base_delay,
        })
    }
}
