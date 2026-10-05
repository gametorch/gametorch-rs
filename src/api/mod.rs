//! Endpoint implementations, grouped by resource.
//!
//! Each submodule contains `impl Client` blocks for one area of the API. They
//! are re-exported through the crate root, so all methods are available on
//! [`crate::Client`].

mod account;
mod animations;
mod art_styles;
mod catalog;
mod exports;
mod keys;
mod labels;
mod projects;
mod saved_animations;
mod sounds;
mod sprites;
mod usage;

pub use animations::{AnimationEstimateBuilder, AnimationRunBuilder, FrameGenerationBuilder};
pub use saved_animations::SaveAnimationBuilder;
pub use sounds::SoundGenerationBuilder;
pub use sprites::SpriteGenerationBuilder;

use crate::types::ListParams;

/// Builds the shared `before` / `include_archived` query pairs.
pub(crate) fn list_query(params: &ListParams) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    if let Some(before) = &params.before {
        query.push(("before", before.clone()));
    }
    if params.include_archived {
        query.push(("include_archived", "true".to_string()));
    }
    query
}
