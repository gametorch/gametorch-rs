//! Usage and spending models.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Response from `GET /usage` (OpenAPI `UsageResponse`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    /// Credit balance, available to admins only.
    #[serde(default, deserialize_with = "crate::serde_helpers::optional_decimal")]
    pub balance_credits: Option<Decimal>,
    /// Credits currently reserved for in-flight work.
    #[serde(default, deserialize_with = "crate::serde_helpers::optional_decimal")]
    pub reserved_credits: Option<Decimal>,
    /// Per-source spend summary.
    #[serde(default)]
    pub summary: Vec<UsageSummary>,
    /// The operation log.
    #[serde(default)]
    pub records: Vec<UsageRecord>,
    /// Cursor for the next page, if any.
    #[serde(default)]
    pub next_cursor: Option<String>,
    /// Total number of records.
    #[serde(default)]
    pub total: i64,
}

/// A per-source spend summary entry (OpenAPI `UsageSummaryEntry`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageSummary {
    /// Spend source, for example `UI` or `API key`.
    pub source: String,
    /// The API key id, when the source is an API key.
    #[serde(default)]
    pub api_key_id: Option<Uuid>,
    /// The user id, when applicable.
    #[serde(default)]
    pub user_id: Option<String>,
    /// The API key name, when available.
    #[serde(default)]
    pub key_name: Option<String>,
    /// Credits consumed by this source.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub credits_consumed: Decimal,
}

/// A single operation-log entry (OpenAPI `UsageRecord`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageRecord {
    /// Unique record id.
    pub id: Uuid,
    /// The associated generation id.
    #[serde(default)]
    pub generation_id: Option<Uuid>,
    /// When the operation happened.
    pub created_at: DateTime<Utc>,
    /// Spend source, for example `UI` or `API key`.
    pub source: String,
    /// The operation name.
    #[serde(default)]
    pub operation: Option<String>,
    /// The API key id, when the source is an API key.
    #[serde(default)]
    pub api_key_id: Option<Uuid>,
    /// The user id, when applicable.
    #[serde(default)]
    pub user_id: Option<String>,
    /// The model used, if any.
    #[serde(default)]
    pub model: Option<String>,
    /// Credits consumed by the operation.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub credits_consumed: Decimal,
    /// Whether the credit debit has been finalized.
    #[serde(default)]
    pub settled: Option<bool>,
    /// The generation status, if any.
    #[serde(default)]
    pub generation_status: Option<String>,
    /// The generation kind, if any.
    #[serde(default)]
    pub kind: Option<String>,
    /// The image model, if any.
    #[serde(default)]
    pub image_model: Option<String>,
    /// The animation model, if any.
    #[serde(default)]
    pub animation_model: Option<String>,
    /// The parent animation model, if any.
    #[serde(default)]
    pub parent_animation_model: Option<String>,
}

/// Response from `GET /usage/histogram` (OpenAPI `UsageHistogram`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageHistogram {
    /// The requested range (`24h`, `7d`, `30d`, `365d`).
    pub range: String,
    /// Width of each bucket in seconds.
    pub width_seconds: i64,
    /// Spend sources present in the buckets.
    #[serde(default)]
    pub sources: Vec<UsageHistogramSource>,
    /// Dense time buckets.
    #[serde(default)]
    pub buckets: Vec<HistogramBucket>,
}

/// A spend source in a usage histogram (OpenAPI `UsageHistogramSource`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageHistogramSource {
    /// Source identifier used as the key in each bucket's `values` map.
    pub id: String,
    /// Human-readable source label.
    #[serde(default)]
    pub label: String,
    /// Source kind.
    #[serde(default)]
    pub kind: String,
}

/// A single histogram bucket (OpenAPI `UsageHistogramBucket`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistogramBucket {
    /// Bucket start time.
    pub start: DateTime<Utc>,
    /// Bucket end time.
    pub end: DateTime<Utc>,
    /// Total credits consumed in the bucket.
    #[serde(deserialize_with = "crate::serde_helpers::decimal")]
    pub total: Decimal,
    /// Credits consumed per source within the bucket.
    #[serde(default, deserialize_with = "crate::serde_helpers::decimal_map")]
    pub values: HashMap<String, Decimal>,
}
