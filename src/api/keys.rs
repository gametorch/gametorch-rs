//! API key management endpoints.

use reqwest::Method;
use uuid::Uuid;

use crate::client::Client;
use crate::error::Result;
use crate::models::{
    ApiKey, ApiKeyWithSecret, CreateApiKeyRequest, KeysResponse, UpdateApiKeyRequest,
};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::OkResponse;

impl Client {
    /// Lists the API keys in the caller's scope. Admin-only for organizations.
    ///
    /// `GET /keys`
    pub async fn list_keys(&self) -> Result<KeysResponse> {
        self.send_json(RateClass::Tier2, "GET /keys", Concurrency::NONE, || {
            self.request(Method::GET, "keys")
        })
        .await
    }

    /// Creates an API key. The full key is returned exactly once.
    ///
    /// `POST /keys`
    pub async fn create_key(&self, request: &CreateApiKeyRequest) -> Result<ApiKeyWithSecret> {
        self.send_json(RateClass::Writes, "POST /keys", Concurrency::NONE, || {
            self.request(Method::POST, "keys").json(request)
        })
        .await
    }

    /// Updates an API key. Only the fields set on `request` are changed.
    ///
    /// `PATCH /keys/{id}`
    pub async fn update_key(
        &self,
        key_id: impl Into<Uuid>,
        request: &UpdateApiKeyRequest,
    ) -> Result<ApiKey> {
        let key_id = key_id.into();
        self.send_json(
            RateClass::Writes,
            "PATCH /keys/{id}",
            Concurrency::NONE,
            || {
                self.request(Method::PATCH, &format!("keys/{key_id}"))
                    .json(request)
            },
        )
        .await
    }

    /// Revokes an API key.
    ///
    /// `DELETE /keys/{id}`
    pub async fn delete_key(&self, key_id: impl Into<Uuid>) -> Result<OkResponse> {
        let key_id = key_id.into();
        self.send_ack(
            RateClass::Writes,
            "DELETE /keys/{id}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("keys/{key_id}")),
        )
        .await
    }
}
