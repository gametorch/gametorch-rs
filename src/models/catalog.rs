//! Catalog models: available sprite, sound and animation models.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Response from `GET /sprite-models` (OpenAPI `SpriteCatalog`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpriteModels {
    /// The text model used for prompt enhancement when none is specified.
    pub default_text_model: String,
    /// Path to the empirical provider-cost notes.
    #[serde(default)]
    pub empirical_evidence: Option<String>,
    /// Available image models.
    #[serde(default)]
    pub image_models: Vec<ImageModel>,
    /// Supported generation modes (`single`, `multiple`).
    #[serde(default)]
    pub modes: Vec<String>,
    /// Layout metadata for `multiple` mode.
    pub multiple_layout: MultipleLayout,
    /// Credits reserved per sprite generation, if reported.
    #[serde(default, deserialize_with = "crate::serde_helpers::optional_decimal")]
    pub reservation_credits: Option<Decimal>,
    /// Human-readable note about what `resolution` refers to.
    #[serde(default)]
    pub resolution_note: Option<String>,
    /// Scope of the resolution setting.
    #[serde(default)]
    pub resolution_scope: Option<String>,
    /// Available prompt-enhancement text models.
    #[serde(default)]
    pub text_models: Vec<TextModel>,
    /// When the catalog was last verified.
    #[serde(default)]
    pub verified_at: Option<String>,
}

/// Layout metadata for sprite `multiple` mode.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct MultipleLayout {
    /// Number of columns in the sheet.
    pub columns: i64,
    /// Number of images requested per generation.
    pub images_per_request: i64,
    /// Number of rows in the sheet.
    pub rows: i64,
}

/// An image model available for sprite generation (OpenAPI `ImageModelInfo`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageModel {
    /// Whether the model is currently available.
    pub available: bool,
    /// Short marketing blurb.
    #[serde(default)]
    pub blurb: String,
    /// URL documenting the model's capabilities.
    #[serde(default)]
    pub capabilities_url: Option<String>,
    /// Default canvas size, for example `1024x1024`.
    #[serde(default)]
    pub default_canvas_size: Option<String>,
    /// Default quality setting.
    #[serde(default)]
    pub default_quality: Option<String>,
    /// Default resolution setting.
    #[serde(default)]
    pub default_resolution: Option<String>,
    /// Whether the model supports editing an existing image.
    pub editing: bool,
    /// Model identifier to pass as `image_model`.
    pub id: String,
    /// Human-readable model name.
    pub name: String,
    /// Whether the model produces transparency natively.
    pub native_transparency: bool,
    /// Quality options supported by the model.
    #[serde(default)]
    pub qualities: Vec<String>,
    /// Credits reserved for a generation with this model, if reported.
    #[serde(default, deserialize_with = "crate::serde_helpers::optional_decimal")]
    pub reservation_credits: Option<Decimal>,
    /// Resolution options supported by the model.
    #[serde(default)]
    pub resolutions: Vec<String>,
    /// Transparency verification status.
    #[serde(default)]
    pub transparency_status: Option<String>,
    /// Reason the model is unavailable, if any.
    #[serde(default)]
    pub unavailable_reason: Option<String>,
}

/// A prompt-enhancement text model (OpenAPI `TextModelInfo`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextModel {
    /// Model identifier to pass as `text_model`.
    pub id: String,
    /// Human-readable model name.
    pub name: String,
}

/// Response from `GET /sound-models` (OpenAPI `SoundCatalog`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SoundModels {
    /// The default output format.
    pub default_format: String,
    /// Supported output formats.
    #[serde(default)]
    pub formats: Vec<String>,
    /// The available sound model.
    pub model: SoundModelInfo,
}

/// A sound generation model (OpenAPI `SoundModelInfo`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SoundModelInfo {
    /// Short marketing blurb.
    #[serde(default)]
    pub blurb: String,
    /// Model identifier to pass as `sound_model`.
    pub id: String,
    /// Human-readable model name.
    pub name: String,
}

/// Response from `GET /animation-models` (OpenAPI `AnimationCatalog`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationModels {
    /// The available animation models.
    #[serde(default)]
    pub data: Vec<AnimationModelInfo>,
}

/// An animation model addressed by its public name (`ash`, `birch`, `cedar`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationModelInfo {
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Public model name to pass as `animation_model`.
    pub id: String,
    /// Human-readable model name.
    pub name: String,
    /// Durations (in seconds) supported by the model.
    #[serde(default)]
    pub supported_durations: Vec<i64>,
    /// Resolutions supported by the model.
    #[serde(default)]
    pub supported_resolutions: Vec<String>,
}
