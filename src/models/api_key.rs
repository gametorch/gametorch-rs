//! API key models.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{ApiKeyScope, SpendResetCadence};

/// An API key (the secret itself is never returned after creation).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiKey {
    /// Unique key id.
    pub id: Uuid,
    /// User-assigned name, if any.
    #[serde(default)]
    pub name: Option<String>,
    /// Non-secret prefix of the key, for display.
    pub key_prefix: String,
    /// When the key expires, if it does.
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    /// Maximum spend in the current cycle, if a limit is set.
    #[serde(default, deserialize_with = "crate::serde_helpers::optional_decimal")]
    pub max_spend_limit: Option<Decimal>,
    /// How often the spend counter resets.
    #[serde(default)]
    pub spend_reset_cadence: String,
    /// What this key is allowed to do.
    #[serde(default)]
    pub key_scope: ApiKeyScope,
    /// The bound project for a project-scoped key; `None` for admin keys.
    #[serde(default)]
    pub project_id: Option<Uuid>,
    /// Spend in the current cycle.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub spend: Decimal,
    /// All-time spend.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub lifetime_spend: Decimal,
    /// When the current cycle ends, or `None` for the `never` cadence.
    #[serde(default)]
    pub spend_reset_at: Option<DateTime<Utc>>,
    /// When the key was created.
    pub created_at: DateTime<Utc>,
}

/// Response from `POST /keys`: an API key plus its secret, returned only once.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiKeyWithSecret {
    /// The key metadata.
    #[serde(flatten)]
    pub key: ApiKey,
    /// The full key. Store it securely; it cannot be retrieved again.
    pub key_full: String,
}

/// Response from `GET /keys`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeysResponse {
    /// Keys in the caller's scope.
    #[serde(default)]
    pub keys: Vec<ApiKey>,
}

/// Request body for `POST /keys`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateApiKeyRequest {
    /// Optional key name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional expiry time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Optional spend limit for the current cycle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_spend_limit: Option<Decimal>,
    /// Optional spend reset cadence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spend_reset_cadence: Option<SpendResetCadence>,
    /// Optional key scope. Defaults to [`ApiKeyScope::Admin`] when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_scope: Option<ApiKeyScope>,
    /// The project to bind a project-scoped key to. Required for
    /// [`ApiKeyScope::ProjectWrite`] and [`ApiKeyScope::ProjectRead`], and must
    /// be omitted for [`ApiKeyScope::Admin`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<Uuid>,
}

impl CreateApiKeyRequest {
    /// Creates an empty request (which the server treats as an admin key).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an admin-scoped key request (full account access).
    pub fn admin() -> Self {
        Self {
            key_scope: Some(ApiKeyScope::Admin),
            ..Self::default()
        }
    }

    /// Creates a write-scoped key request bound to `project_id`.
    pub fn project_write(project_id: impl Into<Uuid>) -> Self {
        Self {
            key_scope: Some(ApiKeyScope::ProjectWrite),
            project_id: Some(project_id.into()),
            ..Self::default()
        }
    }

    /// Creates a read-only key request bound to `project_id`.
    pub fn project_read(project_id: impl Into<Uuid>) -> Self {
        Self {
            key_scope: Some(ApiKeyScope::ProjectRead),
            project_id: Some(project_id.into()),
            ..Self::default()
        }
    }

    /// Sets the key name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the expiry time.
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Sets the spend limit for the current cycle.
    pub fn max_spend_limit(mut self, limit: Decimal) -> Self {
        self.max_spend_limit = Some(limit);
        self
    }

    /// Sets the spend reset cadence.
    pub fn spend_reset_cadence(mut self, cadence: SpendResetCadence) -> Self {
        self.spend_reset_cadence = Some(cadence);
        self
    }

    /// Sets the key scope explicitly.
    pub fn key_scope(mut self, scope: ApiKeyScope) -> Self {
        self.key_scope = Some(scope);
        self
    }

    /// Binds the key to a project. Use with [`ApiKeyScope::ProjectWrite`] or
    /// [`ApiKeyScope::ProjectRead`].
    pub fn project_id(mut self, project_id: impl Into<Uuid>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }
}

/// Request body for `PATCH /keys/{id}`. Only the fields that are set are
/// updated.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateApiKeyRequest {
    /// New key name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New expiry time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// New spend limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_spend_limit: Option<Decimal>,
    /// New spend reset cadence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spend_reset_cadence: Option<SpendResetCadence>,
}

impl UpdateApiKeyRequest {
    /// Creates an empty request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the key name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the expiry time.
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Sets the spend limit.
    pub fn max_spend_limit(mut self, limit: Decimal) -> Self {
        self.max_spend_limit = Some(limit);
        self
    }

    /// Sets the spend reset cadence.
    pub fn spend_reset_cadence(mut self, cadence: SpendResetCadence) -> Self {
        self.spend_reset_cadence = Some(cadence);
        self
    }
}
