//! Animation run, frame and export models.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::provenance::Provenance;

use crate::types::Download;

/// An animation run and its results (OpenAPI `AnimationGeneration`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationRun {
    /// Unique animation run id.
    pub id: Uuid,
    /// Id of the project this run belongs to.
    pub project_id: Uuid,
    /// The prompt used for the run.
    pub prompt: String,
    /// The animation model used (`ash`, `birch` or `cedar`).
    #[serde(default)]
    pub animation_model: Option<String>,
    /// Requested duration in seconds, if any.
    #[serde(default)]
    pub duration: Option<i64>,
    /// The underlying animation clip, once available.
    #[serde(default)]
    pub animation: Option<AnimationAsset>,
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
    /// The base asset the animation was derived from, if any.
    #[serde(default)]
    pub base_asset_id: Option<Uuid>,
    /// Error message if the run failed.
    #[serde(default)]
    pub error: Option<String>,
    /// When the run was created.
    pub created_at: DateTime<Utc>,
    /// When the run completed, if it has.
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    /// When the run was archived, if it is.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
    /// Frame-generation runs for this animation.
    #[serde(default)]
    pub frame_runs: Vec<FrameRun>,
    /// Generated animation frames.
    #[serde(default)]
    pub frames: Vec<AnimationFrame>,
    /// Who or what created this run.
    #[serde(flatten)]
    pub provenance: Provenance,
}

/// Metadata about the underlying animation clip (OpenAPI `AnimationAsset`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationAsset {
    /// Unique clip id.
    pub id: Uuid,
    /// Clip duration in seconds.
    #[serde(default)]
    pub duration_seconds: i64,
    /// Clip resolution.
    #[serde(default)]
    pub resolution: String,
    /// When the clip was created.
    pub created_at: DateTime<Utc>,
}

/// A frame-generation run that sampled an animation into PNG frames (OpenAPI
/// `AnimationFrameRun`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameRun {
    /// Unique frame-run id.
    pub id: Uuid,
    /// Current status (`queued`, `running`, `succeeded`, `failed`).
    pub status: String,
    /// Frames per second used for sampling.
    #[serde(default)]
    pub fps: i64,
    /// Number of frames produced.
    #[serde(default)]
    pub frame_count: i64,
    /// True once this frame run hit a cold start on an internal self-hosted
    /// model.
    #[serde(default)]
    pub warming: bool,
    /// Credits actually consumed once settled.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub credits_consumed: Decimal,
    /// Credits currently reserved for in-flight work.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub reserved_credits: Decimal,
    /// Error message if the generation failed.
    #[serde(default)]
    pub error: Option<String>,
    /// When the frame run was created.
    pub created_at: DateTime<Utc>,
}

/// An individual generated animation frame (OpenAPI `AnimationFrame`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationFrame {
    /// Unique frame id.
    pub id: Uuid,
    /// 1-based frame number.
    pub frame_number: i64,
    /// The frame-generation run that produced this frame.
    #[serde(default)]
    pub generation_id: Option<Uuid>,
    /// When the frame was created.
    pub created_at: DateTime<Utc>,
}

/// Response from `GET /projects/{project_id}/animation-runs` (OpenAPI
/// `AnimationList`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationsResponse {
    /// Animation runs in this page.
    #[serde(default)]
    pub animations: Vec<AnimationRun>,
    /// Cursor for the next page, if any.
    #[serde(default)]
    pub next_cursor: Option<String>,
    /// Total number of animation runs.
    #[serde(default)]
    pub total: i64,
}

/// Response from `POST /projects/{project_id}/animation-runs/estimate` (OpenAPI
/// `AnimationEstimate`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationEstimate {
    /// The animation model the estimate is for.
    pub animation_model: String,
    /// Estimated duration in seconds.
    #[serde(default)]
    pub duration: i64,
    /// Estimated resolution.
    #[serde(default)]
    pub resolution: String,
    /// Estimated credit cost.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub credits: Decimal,
    /// Estimated cost in US dollars.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub usd: Decimal,
    /// Credits that would be reserved up front.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub reserved_credits: Decimal,
}

/// Response from `POST /animation-runs/{id}/export-plan` (OpenAPI `ExportPlan`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExportPlan {
    /// First frame in the exported range.
    pub start_frame: i32,
    /// Last frame in the exported range.
    pub end_frame: i32,
    /// The reference frame used to size the canvas, if any.
    #[serde(default)]
    pub reference: Option<ExportReference>,
    /// The first frame's layout, if any.
    #[serde(default)]
    pub first_frame: Option<ExportFrame>,
    /// Layout for every frame in the range.
    #[serde(default)]
    pub frames: Vec<ExportFrame>,
    /// Maximum width across frames.
    pub max_width: i64,
    /// Maximum height across frames.
    pub max_height: i64,
    /// Uniform scale applied to the frames.
    pub scale: f64,
    /// Horizontal scale applied to the frames.
    pub scale_width: f64,
    /// Vertical scale applied to the frames.
    pub scale_height: f64,
    /// Canvas width of the packed result.
    pub canvas_width: i64,
    /// Canvas height of the packed result.
    pub canvas_height: i64,
    /// Number of frames in the range.
    pub frame_count: i64,
}

/// The reference frame used to size an export canvas (OpenAPI
/// `ExportReference`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExportReference {
    /// Bounding box `[x, y, width, height]`.
    #[serde(default)]
    pub bounds: Vec<i64>,
    /// Source image width.
    pub image_width: i64,
    /// Source image height.
    pub image_height: i64,
}

/// Layout information for a single exported frame (OpenAPI `ExportFrame` and
/// `ExportFirstFrame`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExportFrame {
    /// 1-based frame number.
    pub frame_number: i32,
    /// Bounding box `[x, y, width, height]` in the source image.
    #[serde(default)]
    pub bounds: Option<Vec<i64>>,
    /// Source image width.
    pub image_width: i64,
    /// Source image height.
    pub image_height: i64,
    /// Horizontal offset in the packed canvas.
    #[serde(default)]
    pub offset_x: Option<i64>,
    /// Vertical offset in the packed canvas.
    #[serde(default)]
    pub offset_y: Option<i64>,
    /// Scaled width in the packed canvas.
    #[serde(default)]
    pub scaled_width: Option<i64>,
    /// Scaled height in the packed canvas.
    #[serde(default)]
    pub scaled_height: Option<i64>,
}

/// The JSON body returned by `POST /animation-runs/{id}/export/texturepacker`
/// (OpenAPI `TexturePackerExport`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TexturePackerExport {
    /// The export plan.
    pub plan: ExportPlan,
    /// The TexturePacker "JSON Hash" document.
    #[serde(default)]
    pub texturepacker: serde_json::Value,
    /// Base64-encoded PNG image.
    pub image_base64: String,
    /// Filename for the PNG image.
    pub image_filename: String,
    /// Filename for the JSON atlas.
    pub json_filename: String,
}

/// The JSON body returned by `POST /animation-runs/{id}/export/godot` (OpenAPI
/// `GodotExport`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GodotExport {
    /// The export plan.
    pub plan: ExportPlan,
    /// The Godot `.tres` resource text.
    pub tres: String,
    /// Base64-encoded PNG image.
    pub image_base64: String,
    /// Filename for the PNG image.
    pub image_filename: String,
    /// Filename for the `.tres` resource.
    pub tres_filename: String,
}

/// The result of [`crate::Client::export`], which varies by format.
#[derive(Debug, Clone)]
pub enum Export {
    /// A TexturePacker JSON atlas export.
    TexturePacker(TexturePackerExport),
    /// A Godot export.
    Godot(GodotExport),
    /// A binary export (PNG or zip).
    Binary(Download),
}

impl Export {
    /// Returns the binary payload, if this is a binary export.
    pub fn as_binary(&self) -> Option<&Download> {
        match self {
            Export::Binary(download) => Some(download),
            _ => None,
        }
    }

    /// Consumes the export and returns the binary payload, if any.
    pub fn into_binary(self) -> Option<Download> {
        match self {
            Export::Binary(download) => Some(download),
            _ => None,
        }
    }

    /// Whether this is a binary export.
    pub fn is_binary(&self) -> bool {
        matches!(self, Export::Binary(_))
    }
}
