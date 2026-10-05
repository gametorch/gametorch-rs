//! Saved animation endpoints.

use std::collections::HashMap;

use reqwest::Method;
use serde::Serialize;
use uuid::Uuid;

use crate::api::list_query;
use crate::client::Client;
use crate::error::{Error, Result};
use crate::models::{SavedAnimation, SavedAnimationsResponse};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::{
    ArchiveResponse, AssetMetadataResponse, AssetNameResponse, ListParams, Page, Paginator,
};

#[derive(Serialize)]
struct CreateSavedAnimationBody<'a> {
    generation_id: Uuid,
    start_frame: i64,
    end_frame: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
}

#[derive(Serialize)]
struct UpdateSavedAnimationBody<'a> {
    name: Option<&'a str>,
}

#[derive(Serialize)]
struct MetadataBody<'a> {
    metadata: &'a HashMap<String, String>,
}

impl Client {
    /// Lists a project's saved animations.
    ///
    /// `GET /projects/{project_id}/saved-animations`
    pub async fn list_saved_animations(
        &self,
        project_id: impl Into<Uuid>,
        params: &ListParams,
    ) -> Result<SavedAnimationsResponse> {
        let project_id = project_id.into();
        let query = list_query(params);
        self.send_json(
            RateClass::Tier2,
            "GET /projects/{project_id}/saved-animations",
            Concurrency::NONE,
            || {
                self.request(
                    Method::GET,
                    &format!("projects/{project_id}/saved-animations"),
                )
                .query(&query)
            },
        )
        .await
    }

    /// Starts building a saved animation from a range of an animation run.
    ///
    /// `POST /projects/{project_id}/saved-animations`
    pub fn save_animation(&self, project_id: impl Into<Uuid>) -> SaveAnimationBuilder<'_> {
        SaveAnimationBuilder {
            client: self,
            project_id: project_id.into(),
            generation_id: None,
            start_frame: None,
            end_frame: None,
            name: None,
        }
    }

    /// Returns one saved animation.
    ///
    /// `GET /saved-animations/{id}`
    pub async fn get_saved_animation(&self, id: impl Into<Uuid>) -> Result<SavedAnimation> {
        let id = id.into();
        self.send_json(
            RateClass::Tier2,
            "GET /saved-animations/{id}",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("saved-animations/{id}")),
        )
        .await
    }

    /// Renames a saved animation. Pass `None` to clear the name.
    ///
    /// `PATCH /saved-animations/{id}`
    pub async fn rename_saved_animation(
        &self,
        id: impl Into<Uuid>,
        name: Option<&str>,
    ) -> Result<AssetNameResponse> {
        let id = id.into();
        let name = name.map(str::to_owned);
        self.send_json(
            RateClass::Writes,
            "PATCH /saved-animations/{id}",
            Concurrency::NONE,
            || {
                self.request(Method::PATCH, &format!("saved-animations/{id}"))
                    .json(&UpdateSavedAnimationBody {
                        name: name.as_deref(),
                    })
            },
        )
        .await
    }

    /// Replaces a saved animation's metadata.
    ///
    /// `PUT /saved-animations/{id}/metadata`
    pub async fn put_saved_animation_metadata(
        &self,
        id: impl Into<Uuid>,
        metadata: &HashMap<String, String>,
    ) -> Result<AssetMetadataResponse> {
        let id = id.into();
        self.send_json(
            RateClass::Writes,
            "PUT /saved-animations/{id}/metadata",
            Concurrency::NONE,
            || {
                self.request(Method::PUT, &format!("saved-animations/{id}/metadata"))
                    .json(&MetadataBody { metadata })
            },
        )
        .await
    }

    /// Archives a saved animation.
    ///
    /// `POST /saved-animations/{id}/archive`
    pub async fn archive_saved_animation(&self, id: impl Into<Uuid>) -> Result<ArchiveResponse> {
        let id = id.into();
        self.send_json(
            RateClass::Writes,
            "POST /saved-animations/{id}/archive",
            Concurrency::NONE,
            || self.request(Method::POST, &format!("saved-animations/{id}/archive")),
        )
        .await
    }

    /// Restores an archived saved animation.
    ///
    /// `POST /saved-animations/{id}/unarchive`
    pub async fn unarchive_saved_animation(&self, id: impl Into<Uuid>) -> Result<ArchiveResponse> {
        let id = id.into();
        self.send_json(
            RateClass::Writes,
            "POST /saved-animations/{id}/unarchive",
            Concurrency::NONE,
            || self.request(Method::POST, &format!("saved-animations/{id}/unarchive")),
        )
        .await
    }

    /// Permanently deletes a saved animation.
    ///
    /// `DELETE /saved-animations/{id}`
    pub async fn delete_saved_animation(&self, id: impl Into<Uuid>) -> Result<()> {
        let id = id.into();
        self.send_ok(
            RateClass::Writes,
            "DELETE /saved-animations/{id}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("saved-animations/{id}")),
        )
        .await
    }

    /// Streams a project's saved animations page by page.
    pub fn stream_saved_animations(
        &self,
        project_id: impl Into<Uuid>,
        include_archived: bool,
    ) -> Paginator<'_, SavedAnimation> {
        let project_id = project_id.into();
        Paginator::new(move |cursor| {
            Box::pin(async move {
                let params = ListParams {
                    before: cursor,
                    include_archived,
                    ..Default::default()
                };
                let response = self.list_saved_animations(project_id, &params).await?;
                Ok(Page {
                    items: response.saved_animations,
                    next_cursor: response.next_cursor,
                    total: response.total,
                })
            })
        })
    }
}

/// Builder for creating a saved animation.
pub struct SaveAnimationBuilder<'a> {
    client: &'a Client,
    project_id: Uuid,
    generation_id: Option<Uuid>,
    start_frame: Option<i64>,
    end_frame: Option<i64>,
    name: Option<String>,
}

impl<'a> SaveAnimationBuilder<'a> {
    /// Sets the source animation run id (required).
    pub fn generation_id(mut self, generation_id: impl Into<Uuid>) -> Self {
        self.generation_id = Some(generation_id.into());
        self
    }

    /// Sets the inclusive frame range (required).
    pub fn range(mut self, start_frame: i64, end_frame: i64) -> Self {
        self.start_frame = Some(start_frame);
        self.end_frame = Some(end_frame);
        self
    }

    /// Sets an optional name for the saved animation.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Creates the saved animation.
    pub async fn send(self) -> Result<SavedAnimation> {
        let generation_id = self
            .generation_id
            .ok_or_else(|| Error::Config("saved animation requires a generation_id".into()))?;
        let start_frame = self
            .start_frame
            .ok_or_else(|| Error::Config("saved animation requires a start_frame".into()))?;
        let end_frame = self
            .end_frame
            .ok_or_else(|| Error::Config("saved animation requires an end_frame".into()))?;
        let project_id = self.project_id;
        let body = CreateSavedAnimationBody {
            generation_id,
            start_frame,
            end_frame,
            name: self.name.as_deref(),
        };

        self.client
            .send_json(
                RateClass::Writes,
                "POST /projects/{project_id}/saved-animations",
                Concurrency::NONE,
                || {
                    self.client
                        .request(
                            Method::POST,
                            &format!("projects/{project_id}/saved-animations"),
                        )
                        .json(&body)
                },
            )
            .await
    }
}
