//! Account and health endpoints.

use reqwest::Method;

use crate::client::Client;
use crate::error::Result;
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::OkResponse;

impl Client {
    /// Ensures a mirrored user row exists for the caller.
    ///
    /// `POST /users/me`
    pub async fn ensure_user(&self) -> Result<OkResponse> {
        self.send_ack(
            RateClass::Writes,
            "POST /users/me",
            Concurrency::NONE,
            || self.request(Method::POST, "users/me"),
        )
        .await
    }

    /// Returns the API health status (`"ok"` when healthy).
    ///
    /// `GET /health`
    pub async fn health(&self) -> Result<String> {
        let download = self
            .send_download(
                RateClass::Unlimited,
                "GET /health",
                Concurrency::NONE,
                || self.request(Method::GET, "health"),
            )
            .await?;
        Ok(String::from_utf8_lossy(download.as_bytes())
            .trim()
            .to_string())
    }
}
