//! Sprite generation and asset models.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::provenance::Provenance;

/// A sprite generation and its results (OpenAPI `SpriteGeneration`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Generation {
    /// Unique generation id.
    pub id: Uuid,
    /// Id of the project this generation belongs to.
    pub project_id: Uuid,
    /// The prompt used for the generation.
    pub prompt: String,
    /// Generation mode (`single` or `multiple`).
    pub mode: String,
    /// The image model used.
    pub image_model: String,
    /// The prompt-enhancement text model, if any.
    #[serde(default)]
    pub text_model: Option<String>,
    /// The quality setting, if any.
    #[serde(default)]
    pub quality: Option<String>,
    /// The resolution setting, if any.
    #[serde(default)]
    pub resolution: Option<String>,
    /// The asset this generation edited, if any.
    #[serde(default)]
    pub base_asset_id: Option<Uuid>,
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
    /// True once this run hit a cold start on an internal self-hosted model.
    #[serde(default)]
    pub warming: bool,
    /// Error message if the generation failed.
    #[serde(default)]
    pub error: Option<String>,
    /// Suggested labels for the generated assets.
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::string_list_lenient"
    )]
    pub label_suggestions: Vec<String>,
    /// Suggested art style for this generation, if any.
    #[serde(default)]
    pub art_style_suggestion: Option<String>,
    /// When the generation was created.
    pub created_at: DateTime<Utc>,
    /// When the generation completed, if it has.
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    /// When the generation was archived, if it is.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
    /// The generated assets.
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// Who or what created this generation.
    #[serde(flatten)]
    pub provenance: Provenance,
}

/// Response from `GET /projects/{project_id}/generations` (OpenAPI
/// `GenerationList`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerationsResponse {
    /// Generations in this page.
    #[serde(default)]
    pub generations: Vec<Generation>,
    /// Cursor for the next page, if any.
    #[serde(default)]
    pub next_cursor: Option<String>,
    /// Total number of generations.
    #[serde(default)]
    pub total: i64,
}

/// A sprite asset (OpenAPI `SpriteAsset`). Some fields are absent depending on
/// the endpoint, for example nested assets in a generation omit
/// `generation_id` and `created_at`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Asset {
    /// Unique asset id.
    pub id: Uuid,
    /// User-assigned name, if any.
    #[serde(default)]
    pub name: Option<String>,
    /// Trimmed image width in pixels.
    #[serde(default)]
    pub width: i64,
    /// Trimmed image height in pixels.
    #[serde(default)]
    pub height: i64,
    /// Labels attached to the asset.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Free-form metadata key/value pairs.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Whether an uncropped original is available.
    #[serde(default)]
    pub has_original: bool,
    /// Dismissed label suggestions.
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::string_list_lenient"
    )]
    pub dismissed_suggestions: Vec<String>,
    /// The generation that produced this asset.
    #[serde(default)]
    pub generation_id: Option<Uuid>,
    /// The project this asset belongs to.
    #[serde(default)]
    pub project_id: Option<Uuid>,
    /// The prompt that produced this asset.
    #[serde(default)]
    pub prompt: Option<String>,
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

/// Response from `GET /projects/{project_id}/sprite-assets` (OpenAPI
/// `AssetList`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpriteAssetsResponse {
    /// Matching sprite assets.
    #[serde(default)]
    pub assets: Vec<Asset>,
}
