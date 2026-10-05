//! Sound generation and asset models.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::provenance::Provenance;

/// A sound generation and its results (OpenAPI `SoundGeneration`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SoundGeneration {
    /// Unique sound generation id.
    pub id: Uuid,
    /// Id of the project this generation belongs to.
    pub project_id: Uuid,
    /// The prompt used for the generation.
    pub prompt: String,
    /// The sound model used.
    pub sound_model: String,
    /// Output format requested, if any.
    #[serde(default)]
    pub response_format: Option<String>,
    /// Current status (`queued`, `running`, `succeeded`, `failed`).
    pub status: String,
    /// Credits actually consumed once settled.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub credits_consumed: Decimal,
    /// Credits currently reserved for in-flight work.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub reserved_credits: Decimal,
    /// Number of assets delivered.
    #[serde(default, deserialize_with = "crate::serde_helpers::bool_or_int")]
    pub assets_delivered: i64,
    /// Number of archived assets.
    #[serde(default)]
    pub archived_assets: i64,
    /// Error message if the generation failed.
    #[serde(default)]
    pub error: Option<String>,
    /// Suggested labels for the generated audio.
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::string_list_lenient"
    )]
    pub label_suggestions: Vec<String>,
    /// When the generation was created.
    pub created_at: DateTime<Utc>,
    /// When the generation completed, if it has.
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    /// When the generation was archived, if it is.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
    /// The generated audio assets.
    #[serde(default)]
    pub assets: Vec<SoundAsset>,
    /// Who or what created this generation.
    #[serde(flatten)]
    pub provenance: Provenance,
}

/// Response from `GET /projects/{project_id}/sound-generations` (OpenAPI
/// `SoundGenerationList`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SoundGenerationsResponse {
    /// Sound generations in this page.
    #[serde(default)]
    pub sound_generations: Vec<SoundGeneration>,
    /// Cursor for the next page, if any.
    #[serde(default)]
    pub next_cursor: Option<String>,
    /// Total number of sound generations.
    #[serde(default)]
    pub total: i64,
}

/// A sound asset (OpenAPI `SoundAsset`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SoundAsset {
    /// Unique asset id.
    pub id: Uuid,
    /// Audio format, for example `mp3`.
    #[serde(default)]
    pub format: String,
    /// User-assigned name, if any.
    #[serde(default)]
    pub name: Option<String>,
    /// Labels attached to the asset.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Free-form metadata key/value pairs.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Dismissed label suggestions.
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::string_list_lenient"
    )]
    pub dismissed_suggestions: Vec<String>,
    /// When the asset was created.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// When the asset was archived, if it is.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
    /// Who or what created this asset.
    #[serde(flatten)]
    pub provenance: Provenance,
}
