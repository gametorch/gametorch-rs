//! Catalog endpoints.

use reqwest::Method;

use crate::client::Client;
use crate::error::Result;
use crate::models::{AnimationModels, SoundModels, SpriteModels};
use crate::rate_limit::{Concurrency, RateClass};

impl Client {
    /// Returns the image-model catalog, prompt-enhancement text models,
    /// supported modes and reservation information.
    ///
    /// `GET /sprite-models`
    pub async fn sprite_models(&self) -> Result<SpriteModels> {
        self.send_json(
            RateClass::Tier2,
            "GET /sprite-models",
            Concurrency::NONE,
            || self.request(Method::GET, "sprite-models"),
        )
        .await
    }

    /// Returns the sound-model catalog and supported output formats.
    ///
    /// `GET /sound-models`
    pub async fn sound_models(&self) -> Result<SoundModels> {
        self.send_json(
            RateClass::Tier2,
            "GET /sound-models",
            Concurrency::NONE,
            || self.request(Method::GET, "sound-models"),
        )
        .await
    }

    /// Returns the animation models by their public names (`ash`, `birch`,
    /// `cedar`), with their descriptions, durations and resolutions.
    ///
    /// `GET /animation-models`
    pub async fn animation_models(&self) -> Result<AnimationModels> {
        self.send_json(
            RateClass::Tier2,
            "GET /animation-models",
            Concurrency::NONE,
            || self.request(Method::GET, "animation-models"),
        )
        .await
    }
}
