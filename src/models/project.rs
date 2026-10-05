//! Project models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A GameTorch project.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Project {
    /// Unique project id.
    pub id: Uuid,
    /// Human-readable project name.
    pub name: String,
    /// URL-safe project slug, used by the rename/delete endpoints.
    pub slug: String,
    /// When the project was created.
    pub created_at: DateTime<Utc>,
}

/// Response from `GET /projects`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectsResponse {
    /// Projects in the caller's scope.
    #[serde(default)]
    pub projects: Vec<Project>,
}
