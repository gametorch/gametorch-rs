//! Animation run, frame generation and content endpoints.

use reqwest::Method;
use serde::Serialize;
use uuid::Uuid;

use crate::api::list_query;
use crate::client::Client;
use crate::error::{Error, Result};
use crate::models::{AnimationEstimate, AnimationRun, AnimationsResponse};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::{ArchiveResponse, Download, JobCreated, ListParams, Page, Paginator};

#[derive(Serialize)]
struct EstimateBody<'a> {
    animation_model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_asset_id: Option<Uuid>,
}

#[derive(Serialize)]
struct CreateAnimationBody<'a> {
    request_id: Uuid,
    prompt: &'a str,
    animation_model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_asset_id: Option<Uuid>,
}

#[derive(Serialize)]
struct GenerateFramesBody {
    request_id: Uuid,
    fps: i64,
}

impl Client {
    /// Estimates the credit cost of an animation run without starting one.
    ///
    /// `POST /projects/{project_id}/animation-runs/estimate`
    pub fn estimate_animation(&self, project_id: impl Into<Uuid>) -> AnimationEstimateBuilder<'_> {
        AnimationEstimateBuilder {
            client: self,
            project_id: project_id.into(),
            animation_model: None,
            duration: None,
            base_asset_id: None,
        }
    }

    /// Starts building an animation run.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run(client: gametorch::Client, project: uuid::Uuid, asset: uuid::Uuid) -> gametorch::Result<()> {
    /// let job = client
    ///     .generate_animation(project)
    ///     .prompt("the knight draws her sword and raises it")
    ///     .animation_model("ash")
    ///     .duration(4)
    ///     .base_asset_id(asset)
    ///     .send()
    ///     .await?;
    /// println!("animation {} is {}", job.id, job.status);
    /// # Ok(())
    /// # }
    /// ```
    pub fn generate_animation(&self, project_id: impl Into<Uuid>) -> AnimationRunBuilder<'_> {
        AnimationRunBuilder {
            client: self,
            project_id: project_id.into(),
            prompt: None,
            animation_model: None,
            duration: None,
            base_asset_id: None,
            request_id: None,
        }
    }

    /// Lists a project's animation runs.
    ///
    /// `GET /projects/{project_id}/animation-runs`
    pub async fn list_animation_runs(
        &self,
        project_id: impl Into<Uuid>,
        params: &ListParams,
    ) -> Result<AnimationsResponse> {
        let project_id = project_id.into();
        let mut query = list_query(params);
        if let Some(base_asset_id) = params.base_asset_id {
            query.push(("base_asset_id", base_asset_id.to_string()));
        }
        self.send_json(
            RateClass::Tier2,
            "GET /projects/{project_id}/animation-runs",
            Concurrency::NONE,
            || {
                self.request(
                    Method::GET,
                    &format!("projects/{project_id}/animation-runs"),
                )
                .query(&query)
            },
        )
        .await
    }

    /// Returns one animation run and its frames.
    ///
    /// `GET /animation-runs/{id}`
    pub async fn get_animation_run(&self, run_id: impl Into<Uuid>) -> Result<AnimationRun> {
        let run_id = run_id.into();
        self.send_json(
            RateClass::Tier2,
            "GET /animation-runs/{id}",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("animation-runs/{run_id}")),
        )
        .await
    }

    /// Returns the animation clip bytes.
    ///
    /// `GET /animation-runs/{id}/content`
    pub async fn animation_content(&self, run_id: impl Into<Uuid>) -> Result<Download> {
        let run_id = run_id.into();
        self.send_download(
            RateClass::Tier2,
            "GET /animation-runs/{id}/content",
            Concurrency::NONE,
            || self.request(Method::GET, &format!("animation-runs/{run_id}/content")),
        )
        .await
    }

    /// Archives an animation run.
    ///
    /// `POST /animation-runs/{id}/archive`
    pub async fn archive_animation_run(&self, run_id: impl Into<Uuid>) -> Result<ArchiveResponse> {
        let run_id = run_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /animation-runs/{id}/archive",
            Concurrency::NONE,
            || self.request(Method::POST, &format!("animation-runs/{run_id}/archive")),
        )
        .await
    }

    /// Restores an archived animation run.
    ///
    /// `POST /animation-runs/{id}/unarchive`
    pub async fn unarchive_animation_run(
        &self,
        run_id: impl Into<Uuid>,
    ) -> Result<ArchiveResponse> {
        let run_id = run_id.into();
        self.send_json(
            RateClass::Writes,
            "POST /animation-runs/{id}/unarchive",
            Concurrency::NONE,
            || self.request(Method::POST, &format!("animation-runs/{run_id}/unarchive")),
        )
        .await
    }

    /// Permanently deletes an animation run.
    ///
    /// `DELETE /animation-runs/{id}`
    pub async fn delete_animation_run(&self, run_id: impl Into<Uuid>) -> Result<()> {
        let run_id = run_id.into();
        self.send_ok(
            RateClass::Writes,
            "DELETE /animation-runs/{id}",
            Concurrency::NONE,
            || self.request(Method::DELETE, &format!("animation-runs/{run_id}")),
        )
        .await
    }

    /// Generates individual PNG frames from a finished animation run.
    ///
    /// `POST /projects/{project_id}/animation-runs/{id}/frames`
    pub fn generate_frames(
        &self,
        project_id: impl Into<Uuid>,
        run_id: impl Into<Uuid>,
    ) -> FrameGenerationBuilder<'_> {
        FrameGenerationBuilder {
            client: self,
            project_id: project_id.into(),
            run_id: run_id.into(),
            fps: 12,
            request_id: None,
        }
    }

    /// Returns the PNG bytes of an generated frame by its id.
    ///
    /// `GET /animation-run-frames/{id}/content`
    pub async fn frame_content(&self, frame_id: impl Into<Uuid>) -> Result<Download> {
        let frame_id = frame_id.into();
        self.send_download(
            RateClass::Tier2,
            "GET /animation-run-frames/{id}/content",
            Concurrency::NONE,
            || {
                self.request(
                    Method::GET,
                    &format!("animation-run-frames/{frame_id}/content"),
                )
            },
        )
        .await
    }

    /// Returns the PNG bytes of a frame addressed by its 1-based number, so a
    /// range can be fetched without listing frame ids first.
    ///
    /// `GET /animation-runs/{id}/frames/{number}/content`
    pub async fn frame_content_by_number(
        &self,
        run_id: impl Into<Uuid>,
        frame_number: i64,
    ) -> Result<Download> {
        let run_id = run_id.into();
        self.send_download(
            RateClass::Tier2,
            "GET /animation-runs/{id}/frames/{number}/content",
            Concurrency::NONE,
            || {
                self.request(
                    Method::GET,
                    &format!("animation-runs/{run_id}/frames/{frame_number}/content"),
                )
            },
        )
        .await
    }

    /// Streams a project's animation runs page by page, honoring the same
    /// filters as [`Client::list_animation_runs`] (including `base_asset_id`).
    pub fn stream_animation_runs(
        &self,
        project_id: impl Into<Uuid>,
        params: ListParams,
    ) -> Paginator<'_, AnimationRun> {
        let project_id = project_id.into();
        Paginator::new(move |cursor| {
            let mut page_params = params.clone();
            page_params.before = cursor;
            Box::pin(async move {
                let response = self.list_animation_runs(project_id, &page_params).await?;
                Ok(Page {
                    items: response.animations,
                    next_cursor: response.next_cursor,
                    total: response.total,
                })
            })
        })
    }
}

/// Builder for an animation cost estimate.
pub struct AnimationEstimateBuilder<'a> {
    client: &'a Client,
    project_id: Uuid,
    animation_model: Option<String>,
    duration: Option<i64>,
    base_asset_id: Option<Uuid>,
}

impl<'a> AnimationEstimateBuilder<'a> {
    /// Sets the animation model (`ash`, `birch` or `cedar`); required.
    pub fn animation_model(mut self, animation_model: impl Into<String>) -> Self {
        self.animation_model = Some(animation_model.into());
        self
    }

    /// Sets the desired duration in seconds.
    pub fn duration(mut self, duration: i64) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Bases the estimate on an existing asset.
    pub fn base_asset_id(mut self, base_asset_id: impl Into<Uuid>) -> Self {
        self.base_asset_id = Some(base_asset_id.into());
        self
    }

    /// Sends the estimate request.
    pub async fn send(self) -> Result<AnimationEstimate> {
        let animation_model = self.animation_model.ok_or_else(|| {
            Error::Config("animation estimate requires an animation_model".into())
        })?;
        let project_id = self.project_id;
        let body = EstimateBody {
            animation_model: &animation_model,
            duration: self.duration,
            base_asset_id: self.base_asset_id,
        };
        self.client
            .send_json(
                RateClass::Tier2,
                "POST /projects/{project_id}/animation-runs/estimate",
                Concurrency::NONE,
                || {
                    self.client
                        .request(
                            Method::POST,
                            &format!("projects/{project_id}/animation-runs/estimate"),
                        )
                        .json(&body)
                },
            )
            .await
    }
}

/// Builder for an animation run.
pub struct AnimationRunBuilder<'a> {
    client: &'a Client,
    project_id: Uuid,
    prompt: Option<String>,
    animation_model: Option<String>,
    duration: Option<i64>,
    base_asset_id: Option<Uuid>,
    request_id: Option<Uuid>,
}

impl<'a> AnimationRunBuilder<'a> {
    /// Sets the generation prompt (required).
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Sets the animation model (`ash`, `birch` or `cedar`); required.
    pub fn animation_model(mut self, animation_model: impl Into<String>) -> Self {
        self.animation_model = Some(animation_model.into());
        self
    }

    /// Sets the desired duration in seconds.
    pub fn duration(mut self, duration: i64) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Animates an existing asset.
    pub fn base_asset_id(mut self, base_asset_id: impl Into<Uuid>) -> Self {
        self.base_asset_id = Some(base_asset_id.into());
        self
    }

    /// Overrides the idempotency key. Defaults to a fresh UUID v4.
    pub fn request_id(mut self, request_id: impl Into<Uuid>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Sends the animation run request.
    pub async fn send(self) -> Result<JobCreated> {
        let prompt = self
            .prompt
            .ok_or_else(|| Error::Config("animation run requires a prompt".into()))?;
        let animation_model = self
            .animation_model
            .ok_or_else(|| Error::Config("animation run requires an animation_model".into()))?;
        let request_id = self.request_id.unwrap_or_else(Uuid::new_v4);
        let project_id = self.project_id;

        let body = CreateAnimationBody {
            request_id,
            prompt: &prompt,
            animation_model: &animation_model,
            duration: self.duration,
            base_asset_id: self.base_asset_id,
        };

        self.client
            .send_json(
                RateClass::Tier1,
                "POST /projects/{project_id}/animation-runs",
                Concurrency::HOLD,
                || {
                    self.client
                        .request(
                            Method::POST,
                            &format!("projects/{project_id}/animation-runs"),
                        )
                        .json(&body)
                },
            )
            .await
    }
}

/// Builder for animation frame generation.
pub struct FrameGenerationBuilder<'a> {
    client: &'a Client,
    project_id: Uuid,
    run_id: Uuid,
    fps: i64,
    request_id: Option<Uuid>,
}

impl<'a> FrameGenerationBuilder<'a> {
    /// Sets the sampling rate (1–30 fps). Defaults to 12.
    pub fn fps(mut self, fps: i64) -> Self {
        self.fps = fps;
        self
    }

    /// Overrides the idempotency key. Defaults to a fresh UUID v4.
    pub fn request_id(mut self, request_id: impl Into<Uuid>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Sends the frame-generation request.
    pub async fn send(self) -> Result<JobCreated> {
        let (project_id, run_id) = (self.project_id, self.run_id);
        let body = GenerateFramesBody {
            request_id: self.request_id.unwrap_or_else(Uuid::new_v4),
            fps: self.fps,
        };
        self.client
            .send_json(
                RateClass::Tier1,
                "POST /projects/{project_id}/animation-runs/{id}/frames",
                Concurrency::FRAME_GENERATION,
                || {
                    self.client
                        .request(
                            Method::POST,
                            &format!("projects/{project_id}/animation-runs/{run_id}/frames"),
                        )
                        .json(&body)
                },
            )
            .await
    }
}
