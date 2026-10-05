//! Label models.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::provenance::Provenance;

/// A project label (OpenAPI `LabelRow`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    /// Unique label id.
    pub id: Uuid,
    /// Label name.
    pub name: String,
    /// Hex color, for example `#ca8a04`.
    #[serde(default)]
    pub color: Option<String>,
    /// The asset used as the label's cover thumbnail, if any.
    #[serde(default)]
    pub thumbnail_asset_id: Option<Uuid>,
    /// When the label was created.
    pub created_at: DateTime<Utc>,
}

/// Response from `GET /projects/{project_id}/labels` (OpenAPI `LabelList`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LabelsResponse {
    /// Labels in the project.
    #[serde(default)]
    pub labels: Vec<Label>,
}

/// The label-name set returned by a label mutation (OpenAPI
/// `LabelAssociation`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LabelAssociation {
    /// Always `true` when the mutation succeeded.
    pub ok: bool,
    /// The resulting set of label names on the item.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Response from `GET /labels/{label_id}/items` (OpenAPI `LabelItems`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LabelItems {
    /// The label itself.
    pub label: Label,
    /// Sprite assets tagged with the label.
    #[serde(default)]
    pub assets: Vec<LabelAsset>,
    /// Sound assets tagged with the label.
    #[serde(default)]
    pub sounds: Vec<LabelSound>,
    /// Saved animations tagged with the label.
    #[serde(default)]
    pub saved_animations: Vec<LabelSavedAnimation>,
}

/// A sprite asset as returned by the label-items endpoint (OpenAPI
/// `LabelAsset`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LabelAsset {
    /// Unique asset id.
    pub id: Uuid,
    /// Trimmed image width in pixels.
    #[serde(default)]
    pub width: i64,
    /// Trimmed image height in pixels.
    #[serde(default)]
    pub height: i64,
    /// User-assigned name, if any.
    #[serde(default)]
    pub name: Option<String>,
    /// Labels attached to the asset.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Free-form metadata key/value pairs.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Whether an uncropped original is available.
    #[serde(default)]
    pub has_original: bool,
    /// The generation that produced this asset.
    #[serde(default)]
    pub generation_id: Option<Uuid>,
    /// The prompt that produced this asset.
    #[serde(default)]
    pub prompt: Option<String>,
    /// When the asset was created.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// When the asset was archived, if it is.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
    /// Who or what created this item.
    #[serde(flatten)]
    pub provenance: Provenance,
}

/// A sound asset as returned by the label-items endpoint (OpenAPI
/// `LabelSound`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LabelSound {
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
    /// The generation that produced this asset.
    #[serde(default)]
    pub generation_id: Option<Uuid>,
    /// The prompt that produced this asset.
    #[serde(default)]
    pub prompt: Option<String>,
    /// When the asset was created.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// When the asset was archived, if it is.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
    /// Who or what created this item.
    #[serde(flatten)]
    pub provenance: Provenance,
}

/// A saved animation as returned by the label-items endpoint (OpenAPI
/// `LabelSavedAnimation`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LabelSavedAnimation {
    /// Unique saved-animation id.
    pub id: Uuid,
    /// Id of the project this saved animation belongs to.
    pub project_id: Uuid,
    /// The animation run this range was taken from.
    pub generation_id: Uuid,
    /// First frame of the saved range (1-based).
    pub start_frame: i64,
    /// Last frame of the saved range (1-based).
    pub end_frame: i64,
    /// User-assigned name, if any.
    #[serde(default)]
    pub name: Option<String>,
    /// The animation model of the source run.
    #[serde(default)]
    pub animation_model: Option<String>,
    /// The prompt of the source run.
    #[serde(default)]
    pub prompt: Option<String>,
    /// The base asset of the source run, if any.
    #[serde(default)]
    pub base_asset_id: Option<Uuid>,
    /// Number of frames in the saved range.
    #[serde(default)]
    pub frame_count: i64,
    /// Labels attached to the saved animation.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Free-form metadata key/value pairs.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// When the saved animation was created.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// When the saved animation was archived, if it is.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
    /// Who or what created this item.
    #[serde(flatten)]
    pub provenance: Provenance,
}
