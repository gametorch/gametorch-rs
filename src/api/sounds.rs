//! Sound generation and sound asset endpoints.

use std::collections::HashMap;

use reqwest::Method;
use serde::Serialize;
use uuid::Uuid;

use crate::api::list_query;
use crate::client::Client;
use crate::error::{Error, Result};
use crate::models::{SoundGeneration, SoundGenerationsResponse};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::{
    ArchiveResponse, AssetMetadataResponse, AssetNameResponse, Download, JobCreated, ListParams,
    Page, Paginator,
};

#[derive(Serialize)]
struct CreateSoundBody<'a> {
    request_id: Uuid,
    prompt: &'a str,
    sound_model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<&'a str>,
}

#[derive(Serialize)]
struct UpdateSoundBody<'a> {
    name: Option<&'a str>,
}

#[derive(Serialize)]
struct MetadataBody<'a> {
    metadata: &'a HashMap<String, String>,
}

impl Client {
    /// Starts building a sound generation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(client: gametorch::Client, project: uuid::Uuid) -> gametorch::Result<()> {
    /// let job = client
    ///     .generate_sound(project)
    ///     .prompt("a sword unsheathing, then a heavy thud")
    ///     .sound_model("bytedance-seed/seed-audio-1-0")
    ///     .send()
    ///     .await?;
    /// println!("sound generation {} is {}", job.id, job.status);
    /// # Ok(())
    /// # }
    /// ```
    pub fn generate_sound(&self, project_id: impl Into<Uuid>) -> SoundGenerationBuilder<'_> {
        SoundGenerationBuilder {
            client: self,
            project_id: project_id.into(),
            prompt: None,
            sound_model: None,
            response_format: None,
            request_id: None,
        }
    }

    /// Lists a project's sound generations.
    ///
    /// `GET /projects/{project_id}/sound-generations`
    pub async fn list_sound_generations(
        &self,
        project_id: impl Into<Uuid>,
        params: &ListParams,
    ) -> Result<SoundGenerationsResponse> {
        let project_id = project_id.into();
        let query = list_query(params);
        self.send_json(
            RateClass::Tier2,
            "GET /projects/{project_id}/sound-generations",
            Concurrency::NONE,
            || {
                self.request(
                    Method::GET,
                    &format!("projects/{project_id}/sound-generations"),
                )
                .query(&query)
            },
        )
        .await
    }

    /// Returns one sound generation and its assets.
    ///
    /// `GET /sound-generations/{id}`
    pub async fn get_sound_generation(
        &self,
        generation_id: impl Into<Uuid>,
        include_archived: bool,
    ) -> Result<SoundGeneration> {
        let generation_id = generation_id.into();
        self.send_json(
            RateClass::Tier2,
            "GET /sound-generations/{id}",
            Concurrency::NONE,
            || {
                let request =
                    self.request(Method::GET, &format!("sound-generations/{generation_id}"));
                if include_archived {
                    request.query(&[("include_archived", "true")])
                } else {
                    request
                }
            },
        )
        .await
    }

    /// Returns the audio bytes of a sound asset.
    ///
    /// `GET /sound-assets/{id}/content`
    pub async fn sound_asset_content(&self, asset_id: impl Into<Uuid>) -> Result<Download> {
        let asset_id = asset_id.into();
        self.send_download(
            RateClass::Tier2,
            "GET /sound-assets/{id}/content",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("sound-assets/{asset_id}/content")),
        )
        .await
    }

    /// Renames a sound asset. Pass `None` to clear the name.
    ///
    /// `PATCH /sound-assets/{id}`
    pub async fn rename_sound_asset(
        &self,
        asset_id: impl Into<Uuid>,
        name: Option<&str>,
    ) -> Result<AssetNameResponse> {
        let asset_id = asset_id.into();
        let name = name.map(str::to_owned);
        self.send_json(
            RateClass::Writes,
            "PATCH /sound-assets/{id}",
            Concurrency::NONE,
            || {
                self.request(Method::PATCH, &format!("sound-assets/{asset_id}"))
                    .json(&UpdateSoundBody {
                        name: name.as_deref(),
                    })
            },
        )
        .await
    }

    /// Replaces a sound asset's metadata.
    ///
    /// `PUT /sound-assets/{id}/metadata`
    pub async fn put_sound_asset_metadata(
        &self,
        asset_id: impl Into<Uuid>,
        metadata: &HashMap<String, String>,
    ) -> Result<AssetMetadataResponse> {
        let asset_id = asset_id.into();
        self.send_json(
            RateClass::Writes,
            "PUT /sound-assets/{id}/metadata",
            Concurrency::NONE,
            || {
                self.request(Method::PUT, &format!("sound-assets/{asset_id}/metadata"))
                    .json(&MetadataBody { metadata })
            },
        )
        .await
    }

    /// Archives a sound asset.
    ///
    /// `POST /sound-assets/{id}/archive`
    pub async fn archive_sound_asset(&self, asset_id: impl Into<Uuid>) -> Result<ArchiveResponse> {
        let asset_id = asset_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /sound-assets/{id}/archive",
            Concurrency::NONE,
            || self.request(Method::POST, &format!("sound-assets/{asset_id}/archive")),
        )
        .await
    }

    /// Restores an archived sound asset.
    ///
    /// `POST /sound-assets/{id}/unarchive`
    pub async fn unarchive_sound_asset(
        &self,
        asset_id: impl Into<Uuid>,
    ) -> Result<ArchiveResponse> {
        let asset_id = asset_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /sound-assets/{id}/unarchive",
            Concurrency::NONE,
            || self.request(Method::POST, &format!("sound-assets/{asset_id}/unarchive")),
        )
        .await
    }

    /// Permanently deletes a sound asset.
    ///
    /// `DELETE /sound-assets/{id}`
    pub async fn delete_sound_asset(&self, asset_id: impl Into<Uuid>) -> Result<()> {
        let asset_id = asset_id.into();
        self.send_ok(
            RateClass::Writes,
            "DELETE /sound-assets/{id}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("sound-assets/{asset_id}")),
        )
        .await
    }

    /// Archives a whole sound generation.
    ///
    /// `POST /sound-generations/{id}/archive`
    pub async fn archive_sound_generation(
        &self,
        generation_id: impl Into<Uuid>,
    ) -> Result<ArchiveResponse> {
        let generation_id = generation_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /sound-generations/{id}/archive",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("sound-generations/{generation_id}/archive"),
                )
            },
        )
        .await
    }

    /// Restores an archived sound generation.
    ///
    /// `POST /sound-generations/{id}/unarchive`
    pub async fn unarchive_sound_generation(
        &self,
        generation_id: impl Into<Uuid>,
    ) -> Result<ArchiveResponse> {
        let generation_id = generation_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /sound-generations/{id}/unarchive",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("sound-generations/{generation_id}/unarchive"),
                )
            },
        )
        .await
    }

    /// Streams a project's sound generations page by page.
    pub fn stream_sound_generations(
        &self,
        project_id: impl Into<Uuid>,
        include_archived: bool,
    ) -> Paginator<'_, SoundGeneration> {
        let project_id = project_id.into();
        Paginator::new(move |cursor| {
            Box::pin(async move {
                let params = ListParams {
                    before: cursor,
                    include_archived,
                    ..Default::default()
                };
                let response = self.list_sound_generations(project_id, &params).await?;
                Ok(Page {
                    items: response.sound_generations,
                    next_cursor: response.next_cursor,
                    total: response.total,
                })
            })
        })
    }
}

/// Builder for a sound generation.
///
/// Created by [`Client::generate_sound`]. `prompt` and `sound_model` are
/// required.
pub struct SoundGenerationBuilder<'a> {
    client: &'a Client,
    project_id: Uuid,
    prompt: Option<String>,
    sound_model: Option<String>,
    response_format: Option<String>,
    request_id: Option<Uuid>,
}

impl<'a> SoundGenerationBuilder<'a> {
    /// Sets the generation prompt (required).
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Sets the sound model id (required). See [`Client::sound_models`].
    pub fn sound_model(mut self, sound_model: impl Into<String>) -> Self {
        self.sound_model = Some(sound_model.into());
        self
    }

    /// Sets the output format, for example `mp3` or `pcm`.
    pub fn response_format(mut self, response_format: impl Into<String>) -> Self {
        self.response_format = Some(response_format.into());
        self
    }

    /// Overrides the idempotency key. Defaults to a fresh UUID v4.
    pub fn request_id(mut self, request_id: impl Into<Uuid>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Sends the sound generation request.
    pub async fn send(self) -> Result<JobCreated> {
        let prompt = self
            .prompt
            .ok_or_else(|| Error::Config("sound generation requires a prompt".into()))?;
        let sound_model = self
            .sound_model
            .ok_or_else(|| Error::Config("sound generation requires a sound_model".into()))?;
        let request_id = self.request_id.unwrap_or_else(Uuid::new_v4);
        let project_id = self.project_id;

        let body = CreateSoundBody {
            request_id,
            prompt: &prompt,
            sound_model: &sound_model,
            response_format: self.response_format.as_deref(),
        };

        self.client
            .send_json(
                RateClass::Tier1,
                "POST /projects/{project_id}/sound-generations",
                Concurrency::HOLD,
                || {
                    self.client
                        .request(
                            Method::POST,
                            &format!("projects/{project_id}/sound-generations"),
                        )
                        .json(&body)
                },
            )
            .await
    }
}
