//! # GameTorch Rust SDK
//!
//! The official async Rust SDK for the [GameTorch] API. GameTorch generates
//! game-ready sprites, sound effects and animations from text prompts and
//! organizes them into projects.
//!
//! [GameTorch]: https://gametorch.app
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! gametorch = "0.1"
//! tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
//! ```
//!
//! ## Quickstart
//!
//! ```no_run
//! use gametorch::{Client, SpriteMode};
//!
//! # async fn run() -> gametorch::Result<()> {
//! // Reads GAMETORCH_API_KEY, or pass .api_key("gt2_...") explicitly.
//! let client = Client::from_env()?;
//!
//! let models = client.sprite_models().await?;
//! let project = client.create_project("My Game").await?;
//!
//! let job = client
//!     .generate_sprite(project.id)
//!     .prompt("a red fox, side view")
//!     .mode(SpriteMode::Single)
//!     .image_model(&models.image_models[0].id)
//!     .send()
//!     .await?;
//!
//! println!("generation {} is {}", job.id, job.status);
//! # Ok(())
//! # }
//! ```
//!
//! ## Authentication
//!
//! Every request is authenticated with `Authorization: Bearer <token>`, where
//! the token is either a server-to-server API key (`gt2_...`) or a Clerk
//! session token. Use [`ClientBuilder::api_key`] or
//! [`ClientBuilder::bearer_token`], or set `GAMETORCH_API_KEY` and call
//! [`Client::from_env`].
//!
//! ## Base URL
//!
//! The client defaults to [`DEFAULT_BASE_URL`]
//! (`https://gametorch.app/api`). Point it at a local deployment with
//! [`ClientBuilder::base_url`]:
//!
//! ```no_run
//! # fn run() -> gametorch::Result<()> {
//! let client = gametorch::Client::builder()
//!     .api_key("gt2_local_dev_key")
//!     .base_url("http://localhost:8300/api")
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Rate limits
//!
//! GameTorch rate limits each account per route. The SDK is polite by default:
//! it throttles requests locally to match GameTorch's published limits, caps
//! concurrent hold-creating requests and frame generations, and automatically
//! retries `429`/`5xx` responses with exponential backoff (honoring
//! `Retry-After`). This is all configurable on [`ClientBuilder`]; disable it
//! with [`ClientBuilder::rate_limit`] if you manage limits yourself.
//!
//! ## Pagination
//!
//! List endpoints take [`ListParams`] and return a page with a `next_cursor`.
//! The `stream_*` methods return a [`Paginator`] that fetches pages on demand:
//!
//! ```no_run
//! # async fn run(client: gametorch::Client, project: uuid::Uuid) -> gametorch::Result<()> {
//! let mut generations = client.stream_generations(project, false);
//! while let Some(generation) = generations.next_item().await {
//!     let generation = generation?;
//!     println!("{}", generation.id);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Errors
//!
//! All fallible operations return [`Result`]. The [`Error`] enum exposes the
//! HTTP status and API message, plus helpers such as [`Error::is_not_found`]
//! and [`Error::is_rate_limited`].
//!
//! ## Documentation
//!
//! * API reference: <https://gametorch.app/api/docs>
//! * Agent / LLM guide: <https://gametorch.app/llms.txt>
//! * Privacy policy: <https://gametorch.app/privacy>
//! * Terms and conditions: <https://gametorch.app/terms>

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod api;
mod client;
mod error;
mod rate_limit;
mod serde_helpers;

pub mod models;
pub mod types;

pub use api::{
    AnimationEstimateBuilder, AnimationRunBuilder, FrameGenerationBuilder, SaveAnimationBuilder,
    SoundGenerationBuilder, SpriteGenerationBuilder,
};
pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL, ENV_API_KEY, ENV_BASE_URL};
pub use error::{Error, Result};
pub use models::*;
pub use types::*;
