//! Animation export endpoints.

use reqwest::Method;
use serde::Serialize;
use uuid::Uuid;

use crate::client::Client;
use crate::error::Result;
use crate::models::{Export, ExportPlan, GodotExport, TexturePackerExport};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::{Download, ExportFormat};

#[derive(Serialize)]
struct RangeBody {
    start_frame: i32,
    end_frame: i32,
}

impl Client {
    /// Returns the export plan (frame rectangles and metadata) without building
    /// a file.
    ///
    /// `POST /animation-runs/{id}/export-plan`
    pub async fn export_plan(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<ExportPlan> {
        let run_id = run_id.into();
        let body = RangeBody {
            start_frame,
            end_frame,
        };
        self.send_json(
            RateClass::Tier1,
            "POST /animation-runs/{id}/export-plan",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("animation-runs/{run_id}/export-plan"),
                )
                .json(&body)
            },
        )
        .await
    }

    /// Builds and returns an export in the requested format.
    ///
    /// The result is a JSON document for [`ExportFormat::TexturePacker`] and
    /// [`ExportFormat::Godot`], and raw bytes for every other format. See
    /// [`Export`].
    ///
    /// `POST /animation-runs/{id}/export/{format}`
    pub async fn export(
        &self,
        run_id: impl Into<Uuid>,
        format: ExportFormat,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<Export> {
        match format {
            ExportFormat::TexturePacker => self
                .export_texturepacker(run_id, start_frame, end_frame)
                .await
                .map(Export::TexturePacker),
            ExportFormat::Godot => self
                .export_godot(run_id, start_frame, end_frame)
                .await
                .map(Export::Godot),
            other => self
                .export_binary(run_id, other, start_frame, end_frame)
                .await
                .map(Export::Binary),
        }
    }

    async fn export_binary(
        &self,
        run_id: impl Into<Uuid>,
        format: ExportFormat,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<Download> {
        let run_id = run_id.into();
        let format_path = format.as_str();
        let body = RangeBody {
            start_frame,
            end_frame,
        };
        self.send_download(
            RateClass::Tier1,
            "POST /animation-runs/{id}/export/{format}",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("animation-runs/{run_id}/export/{format_path}"),
                )
                .json(&body)
            },
        )
        .await
    }

    /// Exports a TexturePacker JSON atlas (returned as structured JSON).
    ///
    /// `POST /animation-runs/{id}/export/texturepacker`
    pub async fn export_texturepacker(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<TexturePackerExport> {
        let run_id = run_id.into();
        let body = RangeBody {
            start_frame,
            end_frame,
        };
        self.send_json(
            RateClass::Tier1,
            "POST /animation-runs/{id}/export/texturepacker",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("animation-runs/{run_id}/export/texturepacker"),
                )
                .json(&body)
            },
        )
        .await
    }

    /// Exports a Godot `.tres` resource and PNG (returned as structured JSON).
    ///
    /// `POST /animation-runs/{id}/export/godot`
    pub async fn export_godot(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<GodotExport> {
        let run_id = run_id.into();
        let body = RangeBody {
            start_frame,
            end_frame,
        };
        self.send_json(
            RateClass::Tier1,
            "POST /animation-runs/{id}/export/godot",
            Concurrency::NONE,
            || {
                self.request(
                    Method::POST,
                    &format!("animation-runs/{run_id}/export/godot"),
                )
                .json(&body)
            },
        )
        .await
    }

    /// Exports a zipped TexturePacker JSON atlas.
    pub async fn export_texturepacker_zip(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<Download> {
        self.export_binary(
            run_id,
            ExportFormat::TexturePackerZip,
            start_frame,
            end_frame,
        )
        .await
    }

    /// Exports an Aseprite file.
    pub async fn export_aseprite(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<Download> {
        self.export_binary(run_id, ExportFormat::Aseprite, start_frame, end_frame)
            .await
    }

    /// Exports a zipped Godot `.tres` and PNG.
    pub async fn export_godot_zip(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<Download> {
        self.export_binary(run_id, ExportFormat::GodotZip, start_frame, end_frame)
            .await
    }

    /// Exports a single grid-strip PNG.
    pub async fn export_grid(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<Download> {
        self.export_binary(run_id, ExportFormat::Grid, start_frame, end_frame)
            .await
    }

    /// Exports a GameMaker strip PNG.
    pub async fn export_gamemaker(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<Download> {
        self.export_binary(run_id, ExportFormat::GameMaker, start_frame, end_frame)
            .await
    }

    /// Exports a zipped numbered PNG sequence.
    pub async fn export_sequence_zip(
        &self,
        run_id: impl Into<Uuid>,
        start_frame: i32,
        end_frame: i32,
    ) -> Result<Download> {
        self.export_binary(run_id, ExportFormat::SequenceZip, start_frame, end_frame)
            .await
    }
}
