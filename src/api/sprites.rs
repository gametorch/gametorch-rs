//! Sprite generation and sprite asset endpoints.

use std::collections::HashMap;

use reqwest::Method;
use serde::Serialize;
use uuid::Uuid;

use crate::api::list_query;
use crate::client::Client;
use crate::error::{Error, Result};
use crate::models::{Asset, Generation, GenerationsResponse, SpriteAssetsResponse};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::{
    ArchiveResponse, AssetMetadataResponse, AssetNameResponse, Download, JobCreated, ListParams,
    Page, Paginator, SpriteMode,
};

#[derive(Serialize)]
struct CreateGenerationBody<'a> {
    request_id: Uuid,
    prompt: &'a str,
    mode: SpriteMode,
    image_model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_model: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_asset_id: Option<Uuid>,
}

#[derive(Serialize)]
struct UpdateAssetBody<'a> {
    name: Option<&'a str>,
}

#[derive(Serialize)]
struct MetadataBody<'a> {
    metadata: &'a HashMap<String, String>,
}

impl Client {
    /// Starts building a sprite generation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(client: gametorch::Client, project: uuid::Uuid) -> gametorch::Result<()> {
    /// let job = client
    ///     .generate_sprite(project)
    ///     .prompt("a red fox, side view")
    ///     .mode(gametorch::SpriteMode::Single)
    ///     .image_model("openai/gpt-image-2.5-flare")
    ///     .send()
    ///     .await?;
    /// println!("generation {} is {}", job.id, job.status);
    /// # Ok(())
    /// # }
    /// ```
    pub fn generate_sprite(&self, project_id: impl Into<Uuid>) -> SpriteGenerationBuilder<'_> {
        SpriteGenerationBuilder {
            client: self,
            project_id: project_id.into(),
            prompt: None,
            mode: SpriteMode::Single,
            image_model: None,
            text_model: None,
            quality: None,
            resolution: None,
            base_asset_id: None,
            request_id: None,
        }
    }

    /// Lists a project's generations, including their assets.
    ///
    /// `GET /projects/{project_id}/generations`
    pub async fn list_generations(
        &self,
        project_id: impl Into<Uuid>,
        params: &ListParams,
    ) -> Result<GenerationsResponse> {
        let project_id = project_id.into();
        let query = list_query(params);
        self.send_json(
            RateClass::Tier2,
            "GET /projects/{project_id}/generations",
            Concurrency::NONE,
            || {
                self.request(Method::GET, &format!("projects/{project_id}/generations"))
                    .query(&query)
            },
        )
        .await
    }

    /// Returns one generation and its results.
    ///
    /// `GET /generations/{id}`
    pub async fn get_generation(
        &self,
        generation_id: impl Into<Uuid>,
        include_archived: bool,
    ) -> Result<Generation> {
        let generation_id = generation_id.into();
        self.send_json(
            RateClass::Tier2,
            "GET /generations/{id}",
            Concurrency::NONE,
            || {
                let request = self.request(Method::GET, &format!("generations/{generation_id}"));
                if include_archived {
                    request.query(&[("include_archived", "true")])
                } else {
                    request
                }
            },
        )
        .await
    }

    /// Searches a project's sprite assets by name or label.
    ///
    /// `GET /projects/{project_id}/sprite-assets`
    pub async fn list_sprite_assets(
        &self,
        project_id: impl Into<Uuid>,
        query: Option<&str>,
    ) -> Result<SpriteAssetsResponse> {
        let project_id = project_id.into();
        let query = query.map(str::to_owned);
        self.send_json(
            RateClass::Tier2,
            "GET /projects/{project_id}/sprite-assets",
            Concurrency::NONE,
            || {
                let request =
                    self.request(Method::GET, &format!("projects/{project_id}/sprite-assets"));
                match &query {
                    Some(q) => request.query(&[("q", q.as_str())]),
                    None => request,
                }
            },
        )
        .await
    }

    /// Returns one sprite asset.
    ///
    /// `GET /assets/{id}`
    pub async fn get_asset(&self, asset_id: impl Into<Uuid>) -> Result<Asset> {
        let asset_id = asset_id.into();
        self.send_json(
            RateClass::Tier2,
            "GET /assets/{id}",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("assets/{asset_id}")),
        )
        .await
    }

    /// Returns the trimmed PNG bytes of a sprite asset.
    ///
    /// `GET /assets/{id}/content`
    pub async fn asset_content(&self, asset_id: impl Into<Uuid>) -> Result<Download> {
        let asset_id = asset_id.into();
        self.send_download(
            RateClass::Tier2,
            "GET /assets/{id}/content",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("assets/{asset_id}/content")),
        )
        .await
    }

    /// Returns the uncropped original PNG bytes of a sprite asset.
    ///
    /// `GET /assets/{id}/original`
    pub async fn asset_original(&self, asset_id: impl Into<Uuid>) -> Result<Download> {
        let asset_id = asset_id.into();
        self.send_download(
            RateClass::Tier2,
            "GET /assets/{id}/original",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("assets/{asset_id}/original")),
        )
        .await
    }

    /// Renames a sprite asset. Pass `None` to clear the name.
    ///
    /// `PATCH /assets/{id}`
    pub async fn rename_asset(
        &self,
        asset_id: impl Into<Uuid>,
        name: Option<&str>,
    ) -> Result<AssetNameResponse> {
        let asset_id = asset_id.into();
        let name = name.map(str::to_owned);
        self.send_json(
            RateClass::Writes,
            "PATCH /assets/{id}",
            Concurrency::NONE,
            || {
                self.request(Method::PATCH, &format!("assets/{asset_id}"))
                    .json(&UpdateAssetBody {
                        name: name.as_deref(),
                    })
            },
        )
        .await
    }

    /// Replaces a sprite asset's metadata.
    ///
    /// `PUT /assets/{id}/metadata`
    pub async fn put_asset_metadata(
        &self,
        asset_id: impl Into<Uuid>,
        metadata: &HashMap<String, String>,
    ) -> Result<AssetMetadataResponse> {
        let asset_id = asset_id.into();
        self.send_json(
            RateClass::Writes,
            "PUT /assets/{id}/metadata",
            Concurrency::NONE,
            || {
                self.request(Method::PUT, &format!("assets/{asset_id}/metadata"))
                    .json(&MetadataBody { metadata })
            },
        )
        .await
    }

    /// Archives a sprite asset, hiding it from default lists.
    ///
    /// `POST /assets/{id}/archive`
    pub async fn archive_asset(&self, asset_id: impl Into<Uuid>) -> Result<ArchiveResponse> {
        let asset_id = asset_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /assets/{id}/archive",
            Concurrency::NONE,
            || self.request(Method::POST, &format!("assets/{asset_id}/archive")),
        )
        .await
    }

    /// Restores an archived sprite asset.
    ///
    /// `POST /assets/{id}/unarchive`
    pub async fn unarchive_asset(&self, asset_id: impl Into<Uuid>) -> Result<ArchiveResponse> {
        let asset_id = asset_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /assets/{id}/unarchive",
            Concurrency::NONE,
            || self.request(Method::POST, &format!("assets/{asset_id}/unarchive")),
        )
        .await
    }

    /// Permanently deletes a sprite asset and its original.
    ///
    /// `DELETE /assets/{id}`
    pub async fn delete_asset(&self, asset_id: impl Into<Uuid>) -> Result<()> {
        let asset_id = asset_id.into();
        self.send_ok(
            RateClass::Writes,
            "DELETE /assets/{id}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("assets/{asset_id}")),
        )
        .await
    }

    /// Archives a whole generation.
    ///
    /// `POST /generations/{id}/archive`
    pub async fn archive_generation(
        &self,
        generation_id: impl Into<Uuid>,
    ) -> Result<ArchiveResponse> {
        let generation_id = generation_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /generations/{id}/archive",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("generations/{generation_id}/archive"),
                )
            },
        )
        .await
    }

    /// Restores an archived generation.
    ///
    /// `POST /generations/{id}/unarchive`
    pub async fn unarchive_generation(
        &self,
        generation_id: impl Into<Uuid>,
    ) -> Result<ArchiveResponse> {
        let generation_id = generation_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /generations/{id}/unarchive",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("generations/{generation_id}/unarchive"),
                )
            },
        )
        .await
    }

    /// Permanently deletes a generation.
    ///
    /// `DELETE /generations/{id}`
    pub async fn delete_generation(&self, generation_id: impl Into<Uuid>) -> Result<()> {
        let generation_id = generation_id.into();
        self.send_ok(
            RateClass::Writes,
            "DELETE /generations/{id}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("generations/{generation_id}")),
        )
        .await
    }

    /// Streams a project's generations page by page.
    pub fn stream_generations(
        &self,
        project_id: impl Into<Uuid>,
        include_archived: bool,
    ) -> Paginator<'_, Generation> {
        let project_id = project_id.into();
        Paginator::new(move |cursor| {
            Box::pin(async move {
                let params = ListParams {
                    before: cursor,
                    include_archived,
                    ..Default::default()
                };
                let response = self.list_generations(project_id, &params).await?;
                Ok(Page {
                    items: response.generations,
                    next_cursor: response.next_cursor,
                    total: response.total,
                })
            })
        })
    }
}

/// Builder for a sprite generation.
///
/// Created by [`Client::generate_sprite`]. `prompt` and `image_model` are
/// required; everything else has a sensible default.
pub struct SpriteGenerationBuilder<'a> {
    client: &'a Client,
    project_id: Uuid,
    prompt: Option<String>,
    mode: SpriteMode,
    image_model: Option<String>,
    text_model: Option<String>,
    quality: Option<String>,
    resolution: Option<String>,
    base_asset_id: Option<Uuid>,
    request_id: Option<Uuid>,
}

impl<'a> SpriteGenerationBuilder<'a> {
    /// Sets the generation prompt (required).
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Sets the generation mode. Defaults to [`SpriteMode::Single`].
    pub fn mode(mut self, mode: SpriteMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the image model id (required). See [`Client::sprite_models`].
    pub fn image_model(mut self, image_model: impl Into<String>) -> Self {
        self.image_model = Some(image_model.into());
        self
    }

    /// Sets the prompt-enhancement text model. Use `"none"` to disable
    /// enhancement.
    pub fn text_model(mut self, text_model: impl Into<String>) -> Self {
        self.text_model = Some(text_model.into());
        self
    }

    /// Disables prompt enhancement.
    pub fn no_text_model(mut self) -> Self {
        self.text_model = Some("none".to_string());
        self
    }

    /// Sets the quality setting.
    pub fn quality(mut self, quality: impl Into<String>) -> Self {
        self.quality = Some(quality.into());
        self
    }

    /// Sets the resolution setting.
    pub fn resolution(mut self, resolution: impl Into<String>) -> Self {
        self.resolution = Some(resolution.into());
        self
    }

    /// Edits an existing individual image result.
    pub fn base_asset_id(mut self, base_asset_id: impl Into<Uuid>) -> Self {
        self.base_asset_id = Some(base_asset_id.into());
        self
    }

    /// Overrides the idempotency key. By default a fresh UUID v4 is generated
    /// for every call. Reusing a key with the same body returns the existing
    /// generation; reusing it with a different body returns `409`.
    pub fn request_id(mut self, request_id: impl Into<Uuid>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Sends the generation request.
    pub async fn send(self) -> Result<JobCreated> {
        let prompt = self
            .prompt
            .ok_or_else(|| Error::Config("sprite generation requires a prompt".into()))?;
        let image_model = self
            .image_model
            .ok_or_else(|| Error::Config("sprite generation requires an image_model".into()))?;
        let request_id = self.request_id.unwrap_or_else(Uuid::new_v4);
        let project_id = self.project_id;

        let body = CreateGenerationBody {
            request_id,
            prompt: &prompt,
            mode: self.mode,
            image_model: &image_model,
            text_model: self.text_model.as_deref(),
            quality: self.quality.as_deref(),
            resolution: self.resolution.as_deref(),
            base_asset_id: self.base_asset_id,
        };

        self.client
            .send_json(
                RateClass::Tier1,
                "POST /projects/{project_id}/generations",
                Concurrency::HOLD,
                || {
                    self.client
                        .request(Method::POST, &format!("projects/{project_id}/generations"))
                        .json(&body)
                },
            )
            .await
    }
}
