//! Request model for the metadata endpoints.

use std::collections::HashMap;

use serde::Serialize;

/// Request body for `PUT /assets/{id}/metadata`, `PUT /sound-assets/{id}/metadata`
/// and `PUT /saved-animations/{id}/metadata` (OpenAPI `PutMetadataRequest`,
/// `PutSoundMetadataRequest`, `PutSavedAnimationMetadataRequest`).
///
/// Metadata updates are **additive**: keys in [`metadata`](Self::metadata) are
/// added or overwritten and every other key is kept. A call never clears
/// metadata; to delete keys, list them in [`remove`](Self::remove).
///
/// # Examples
///
/// ```
/// use gametorch::PutMetadataRequest;
///
/// let request = PutMetadataRequest::new()
///     .set("variant", "background")
///     .set("facing", "left")
///     .remove("old_key");
/// let json = serde_json::to_value(&request).unwrap();
/// assert_eq!(json["metadata"]["variant"], "background");
/// assert_eq!(json["remove"], serde_json::json!(["old_key"]));
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct PutMetadataRequest {
    /// Keys to add or overwrite. Keys are 1–1024 characters, values 0–1024
    /// characters, and at most 10 pairs are allowed per call.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    /// Keys to delete. At most 10 keys are allowed per call. This is the only
    /// way to remove metadata.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<String>,
}

impl PutMetadataRequest {
    /// Creates an empty (no-op) request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or overwrites one key.
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Merges in a whole map of key/value pairs (in addition to any already set).
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata.extend(metadata);
        self
    }

    /// Removes one key.
    pub fn remove(mut self, key: impl Into<String>) -> Self {
        self.remove.push(key.into());
        self
    }

    /// Removes many keys.
    pub fn remove_keys<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.remove.extend(keys.into_iter().map(Into::into));
        self
    }
}
