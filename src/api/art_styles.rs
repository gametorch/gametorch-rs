//! Art style endpoints.

use reqwest::Method;
use serde::Serialize;
use uuid::Uuid;

use crate::client::Client;
use crate::error::Result;
use crate::models::{ArtStyle, ArtStyleSuggestion, ArtStylesResponse};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::OkResponse;

#[derive(Serialize)]
struct CreateArtStyleBody<'a> {
    name: &'a str,
}

impl Client {
    /// Lists a project's reusable art styles.
    ///
    /// `GET /projects/{project_id}/art-styles`
    pub async fn list_art_styles(&self, project_id: impl Into<Uuid>) -> Result<ArtStylesResponse> {
        let project_id = project_id.into();
        self.send_json(
            RateClass::Tier2,
            "GET /projects/{project_id}/art-styles",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("projects/{project_id}/art-styles")),
        )
        .await
    }

    /// Creates an art style.
    ///
    /// `POST /projects/{project_id}/art-styles`
    pub async fn create_art_style(
        &self,
        project_id: impl Into<Uuid>,
        name: impl AsRef<str>,
    ) -> Result<ArtStyle> {
        let project_id = project_id.into();
        let name = name.as_ref().to_string();
        self.send_json(
            RateClass::Writes,
            "POST /projects/{project_id}/art-styles",
            Concurrency::NONE,
            || {
                self.request(Method::POST, &format!("projects/{project_id}/art-styles"))
                    .json(&CreateArtStyleBody { name: &name })
            },
        )
        .await
    }

    /// Returns a fresh suggested art style for the user to review.
    ///
    /// `POST /projects/{project_id}/art-styles/generate`
    pub async fn generate_art_style(
        &self,
        project_id: impl Into<Uuid>,
    ) -> Result<ArtStyleSuggestion> {
        let project_id = project_id.into();
        self.send_json(
            RateClass::Tier2,
            "POST /projects/{project_id}/art-styles/generate",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("projects/{project_id}/art-styles/generate"),
                )
            },
        )
        .await
    }

    /// Deletes an art style.
    ///
    /// `DELETE /art-styles/{art_style_id}`
    pub async fn delete_art_style(&self, art_style_id: impl Into<Uuid>) -> Result<OkResponse> {
        let art_style_id = art_style_id.into();
        self.send_ack(
            RateClass::Writes,
            "DELETE /art-styles/{art_style_id}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("art-styles/{art_style_id}")),
        )
        .await
    }
}
