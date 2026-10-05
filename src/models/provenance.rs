//! Provenance metadata shared by generated resources.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Who or what created a resource.
///
/// GameTorch records the Clerk user and, when the request came through an API
/// key, the key that ran the operation. These fields are flattened into the
/// owning resource, so they appear as `user_id`, `source`, `api_key_id` and
/// `key_name` on the resource itself.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Provenance {
    /// Clerk user id (or API-key owner) credited for the result.
    #[serde(default)]
    pub user_id: Option<String>,
    /// Spend/creation source, for example `UI` or `API key`.
    #[serde(default)]
    pub source: Option<String>,
    /// The API key used, when the result came through one.
    #[serde(default)]
    pub api_key_id: Option<Uuid>,
    /// The API key's name, when available.
    #[serde(default)]
    pub key_name: Option<String>,
}
