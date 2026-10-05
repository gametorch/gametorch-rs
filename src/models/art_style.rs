//! Art style models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A reusable project art style.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArtStyle {
    /// Unique art-style id.
    pub id: Uuid,
    /// Art-style name.
    pub name: String,
    /// When the art style was created.
    pub created_at: DateTime<Utc>,
}

/// Response from `GET /projects/{project_id}/art-styles`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArtStylesResponse {
    /// Art styles in the project.
    #[serde(default)]
    pub art_styles: Vec<ArtStyle>,
}

/// A freshly generated art-style suggestion returned by
/// `POST /projects/{project_id}/art-styles/generate`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArtStyleSuggestion {
    /// The suggested art-style name.
    pub name: String,
}
