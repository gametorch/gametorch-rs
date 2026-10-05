//! Usage and spending endpoints.

use reqwest::Method;

use crate::client::Client;
use crate::error::Result;
use crate::models::{Usage, UsageHistogram, UsageRecord};
use crate::rate_limit::{Concurrency, RateClass};
use crate::types::{Page, Paginator};

impl Client {
    /// Returns the credit balance (admins), a per-source summary and the
    /// operation log.
    ///
    /// * `before` — pagination cursor.
    /// * `split` — pass `Some("user")` to break usage out per member.
    ///
    /// `GET /usage`
    pub async fn usage(&self, before: Option<&str>, split: Option<&str>) -> Result<Usage> {
        let query = usage_query(before, split);
        self.send_json(RateClass::Tier2, "GET /usage", Concurrency::NONE, || {
            self.request(Method::GET, "usage").query(&query)
        })
        .await
    }

    /// Returns dense time buckets of spend split by source.
    ///
    /// * `range` — `24h`, `7d`, `30d` or `365d`.
    /// * `source` — filter to a single spend source.
    /// * `split` — pass `Some("user")` to break usage out per member.
    ///
    /// `GET /usage/histogram`
    pub async fn usage_histogram(
        &self,
        range: Option<&str>,
        source: Option<&str>,
        split: Option<&str>,
    ) -> Result<UsageHistogram> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(range) = range {
            query.push(("range", range.to_string()));
        }
        if let Some(source) = source {
            query.push(("source", source.to_string()));
        }
        if let Some(split) = split {
            query.push(("split", split.to_string()));
        }
        self.send_json(
            RateClass::Tier2,
            "GET /usage/histogram",
            Concurrency::NONE,
            || self.request(Method::GET, "usage/histogram").query(&query),
        )
        .await
    }

    /// Streams the operation log page by page.
    pub fn stream_usage(&self, split: Option<&str>) -> Paginator<'_, UsageRecord> {
        let split = split.map(str::to_owned);
        Paginator::new(move |cursor| {
            let split = split.clone();
            Box::pin(async move {
                let response = self.usage(cursor.as_deref(), split.as_deref()).await?;
                Ok(Page {
                    items: response.records,
                    next_cursor: response.next_cursor,
                    total: response.total,
                })
            })
        })
    }
}

fn usage_query(before: Option<&str>, split: Option<&str>) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    if let Some(before) = before {
        query.push(("before", before.to_string()));
    }
    if let Some(split) = split {
        query.push(("split", split.to_string()));
    }
    query
}
