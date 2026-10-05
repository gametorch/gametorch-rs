//! Shared types used across the SDK: pagination, request parameters, downloads
//! and the common acknowledgement/creation envelopes.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// A generic acknowledgement response, used by delete and a few write
/// endpoints (OpenAPI `OkResponse`).
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct OkResponse {
    /// Always `true` when the operation succeeded.
    pub ok: bool,
}

/// Response returned by archive/unarchive endpoints (OpenAPI
/// `ArchiveResponse`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArchiveResponse {
    /// Always `true` when the operation succeeded.
    pub ok: bool,
    /// When the item was archived, or `None` after unarchiving.
    #[serde(default)]
    pub archived_at: Option<DateTime<Utc>>,
}

/// Response returned by asset rename endpoints (OpenAPI `AssetNameResponse`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetNameResponse {
    /// Always `true` when the operation succeeded.
    pub ok: bool,
    /// The item's new name, or `None` if it was cleared.
    #[serde(default)]
    pub name: Option<String>,
}

/// Response returned by metadata endpoints (OpenAPI `AssetMetadataResponse`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetMetadataResponse {
    /// Always `true` when the operation succeeded.
    pub ok: bool,
    /// The item's metadata after the update.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// A job-creation envelope returned by generation and frame-generation
/// endpoints (the OpenAPI `Reservation` schema).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobCreated {
    /// Identifier of the created job.
    pub id: Uuid,
    /// Current status, for example `queued` or `running`.
    pub status: String,
    /// `true` when this call created the job, `false` for an idempotent replay
    /// of an existing `request_id`.
    pub created: bool,
    /// Credits reserved up front for the job.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub reserved_credits: Decimal,
}

/// Raw bytes returned by a content endpoint, along with the response's content
/// type.
#[derive(Debug, Clone)]
pub struct Download {
    /// The `Content-Type` reported by the server, if any.
    pub content_type: Option<String>,
    /// The response body.
    pub data: Bytes,
}

impl Download {
    /// The content type reported by the server, if any.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// The raw response bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Consumes the download and returns the raw bytes.
    pub fn into_bytes(self) -> Bytes {
        self.data
    }

    /// The length of the body in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the body is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// One page of a paginated list response.
#[derive(Debug, Clone)]
pub struct Page<T> {
    /// The items in this page.
    pub items: Vec<T>,
    /// Opaque cursor to pass as `before` to fetch the next page.
    pub next_cursor: Option<String>,
    /// Total number of items available across all pages.
    pub total: i64,
}

/// Options shared by the paginated list endpoints.
#[derive(Debug, Clone, Default)]
pub struct ListParams {
    /// Cursor returned by a previous page.
    pub before: Option<String>,
    /// Include archived items (defaults to `false`).
    pub include_archived: bool,
    /// Only return animation runs whose base image is this sprite asset.
    ///
    /// Only used by [`crate::Client::list_animation_runs`]; ignored by the
    /// other list endpoints.
    pub base_asset_id: Option<Uuid>,
}

impl ListParams {
    /// Creates empty list parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the pagination cursor.
    pub fn before(mut self, cursor: impl Into<String>) -> Self {
        self.before = Some(cursor.into());
        self
    }

    /// Includes or excludes archived items.
    pub fn include_archived(mut self, include: bool) -> Self {
        self.include_archived = include;
        self
    }

    /// Filters animation runs by their base sprite asset.
    pub fn base_asset_id(mut self, base_asset_id: impl Into<Uuid>) -> Self {
        self.base_asset_id = Some(base_asset_id.into());
        self
    }
}

/// A boxed async page fetcher used internally by [`Paginator`].
type PageFetcher<'a, T> = Box<dyn Fn(Option<String>) -> BoxFuture<'a, Result<Page<T>>> + Send + 'a>;

/// An asynchronous cursor over a paginated GameTorch list endpoint.
///
/// Created by the `stream_*` methods on [`crate::Client`].
pub struct Paginator<'a, T> {
    fetch: PageFetcher<'a, T>,
    cursor: Option<String>,
    finished: bool,
    buffer: std::vec::IntoIter<T>,
    total: Option<i64>,
}

impl<'a, T> fmt::Debug for Paginator<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Paginator")
            .field("cursor", &self.cursor)
            .field("finished", &self.finished)
            .field("total", &self.total)
            .finish_non_exhaustive()
    }
}

impl<'a, T> Paginator<'a, T> {
    pub(crate) fn new<F>(fetch: F) -> Self
    where
        F: Fn(Option<String>) -> BoxFuture<'a, Result<Page<T>>> + Send + 'a,
    {
        Paginator {
            fetch: Box::new(fetch),
            cursor: None,
            finished: false,
            buffer: Vec::new().into_iter(),
            total: None,
        }
    }

    /// Fetches the next page, or `None` when the cursor is exhausted.
    pub async fn next_page(&mut self) -> Option<Result<Vec<T>>> {
        if self.finished {
            return None;
        }

        match (self.fetch)(self.cursor.clone()).await {
            Ok(page) => {
                self.total = Some(page.total);
                self.cursor = page.next_cursor;
                if self.cursor.is_none() {
                    self.finished = true;
                }
                if page.items.is_empty() && self.finished {
                    return None;
                }
                Some(Ok(page.items))
            }
            Err(err) => {
                self.finished = true;
                Some(Err(err))
            }
        }
    }

    /// Fetches the next item, transparently loading pages as needed.
    pub async fn next_item(&mut self) -> Option<Result<T>> {
        loop {
            if let Some(item) = self.buffer.next() {
                return Some(Ok(item));
            }
            match self.next_page().await? {
                Ok(items) => self.buffer = items.into_iter(),
                Err(err) => return Some(Err(err)),
            }
        }
    }

    /// The total number of items reported by the API, once the first page has
    /// been fetched.
    pub fn total(&self) -> Option<i64> {
        self.total
    }
}

/// Sprite generation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpriteMode {
    /// Generate a single sprite.
    Single,
    /// Generate four sprites from one 2x2 sheet.
    Multiple,
}

impl fmt::Display for SpriteMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SpriteMode::Single => "single",
            SpriteMode::Multiple => "multiple",
        })
    }
}

impl FromStr for SpriteMode {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "single" => Ok(SpriteMode::Single),
            "multiple" => Ok(SpriteMode::Multiple),
            other => Err(format!("unknown sprite mode: {other}")),
        }
    }
}

/// The supported animation export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// TexturePacker JSON atlas.
    TexturePacker,
    /// TexturePacker JSON atlas plus PNG, zipped.
    TexturePackerZip,
    /// Aseprite `.aseprite` file.
    Aseprite,
    /// Godot `.tres` plus PNG.
    Godot,
    /// Godot `.tres` plus PNG, zipped.
    GodotZip,
    /// A single grid-strip PNG.
    Grid,
    /// A GameMaker strip PNG.
    GameMaker,
    /// A numbered PNG sequence, zipped.
    SequenceZip,
}

impl ExportFormat {
    /// The path segment used by the export endpoint.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportFormat::TexturePacker => "texturepacker",
            ExportFormat::TexturePackerZip => "texturepacker.zip",
            ExportFormat::Aseprite => "aseprite",
            ExportFormat::Godot => "godot",
            ExportFormat::GodotZip => "godot.zip",
            ExportFormat::Grid => "grid",
            ExportFormat::GameMaker => "gamemaker",
            ExportFormat::SequenceZip => "sequence.zip",
        }
    }
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ExportFormat {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "texturepacker" => Ok(ExportFormat::TexturePacker),
            "texturepacker.zip" => Ok(ExportFormat::TexturePackerZip),
            "aseprite" => Ok(ExportFormat::Aseprite),
            "godot" => Ok(ExportFormat::Godot),
            "godot.zip" => Ok(ExportFormat::GodotZip),
            "grid" => Ok(ExportFormat::Grid),
            "gamemaker" => Ok(ExportFormat::GameMaker),
            "sequence.zip" => Ok(ExportFormat::SequenceZip),
            other => Err(format!("unknown export format: {other}")),
        }
    }
}

/// What an API key is allowed to do. The scope is fixed at creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyScope {
    /// The historical key type: full access to the owning account/org, exactly
    /// like an admin session.
    #[default]
    Admin,
    /// Bound to one project: can read it and write its sprites, sounds,
    /// animations, labels, names and metadata, but cannot touch account
    /// surfaces or any other project.
    ProjectWrite,
    /// Bound to one project: read-only.
    ProjectRead,
}

impl fmt::Display for ApiKeyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ApiKeyScope {
    /// The wire representation of the scope.
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyScope::Admin => "admin",
            ApiKeyScope::ProjectWrite => "project_write",
            ApiKeyScope::ProjectRead => "project_read",
        }
    }
}

impl FromStr for ApiKeyScope {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "admin" => Ok(ApiKeyScope::Admin),
            "project_write" => Ok(ApiKeyScope::ProjectWrite),
            "project_read" => Ok(ApiKeyScope::ProjectRead),
            other => Err(format!("unknown API key scope: {other}")),
        }
    }
}

/// How often an API key's spend counter resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpendResetCadence {
    /// Reset daily.
    Daily,
    /// Reset weekly.
    Weekly,
    /// Reset monthly.
    Monthly,
    /// Never reset.
    Never,
}

impl fmt::Display for SpendResetCadence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SpendResetCadence::Daily => "daily",
            SpendResetCadence::Weekly => "weekly",
            SpendResetCadence::Monthly => "monthly",
            SpendResetCadence::Never => "never",
        })
    }
}

impl FromStr for SpendResetCadence {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "daily" => Ok(SpendResetCadence::Daily),
            "weekly" => Ok(SpendResetCadence::Weekly),
            "monthly" => Ok(SpendResetCadence::Monthly),
            "never" => Ok(SpendResetCadence::Never),
            other => Err(format!("unknown spend reset cadence: {other}")),
        }
    }
}

/// Converts GameTorch credits to US dollars (100 credits = $1).
pub fn credits_to_usd(credits: Decimal) -> Decimal {
    credits / Decimal::from(100)
}
