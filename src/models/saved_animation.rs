//! Saved animation models.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A named frame range taken from an animation run, reusable as a preset
/// (OpenAPI `SavedAnimation`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SavedAnimation {
    /// Unique saved-animation id.
    pub id: Uuid,
    /// Id of the project this saved animation belongs to.
    pub project_id: Uuid,
    /// The animation run this range was taken from.
    pub generation_id: Uuid,
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
    /// First frame of the saved range (1-based).
    pub start_frame: i64,
    /// Last frame of the saved range (1-based).
    pub end_frame: i64,
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
    pub created_at: DateTime<Utc>,
    /// When the saved animation was archived, if it is.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
}

/// Response from `GET /projects/{project_id}/saved-animations` (OpenAPI
/// `SavedAnimationList`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SavedAnimationsResponse {
    /// Saved animations in this page.
    #[serde(default)]
    pub saved_animations: Vec<SavedAnimation>,
    /// Cursor for the next page, if any.
    #[serde(default)]
    pub next_cursor: Option<String>,
    /// Total number of saved animations.
    #[serde(default)]
    pub total: i64,
}
