//! Live integration tests.
//!
//! These run only when `GAMETORCH_API_KEY` is set, so `cargo test` stays
//! offline by default. Point them at a local deployment with
//! `GAMETORCH_BASE_URL=http://localhost:8300/api`.
//!
//! They exercise read-only endpoints (and the read-only export plan), so they
//! never spend credits.

use gametorch::{Client, Error};
use uuid::Uuid;

fn client() -> Option<Client> {
    let api_key = std::env::var("GAMETORCH_API_KEY").ok()?;
    let mut builder = Client::builder().api_key(api_key);
    if let Ok(base_url) = std::env::var("GAMETORCH_BASE_URL") {
        builder = builder.base_url(base_url);
    }
    Some(builder.build().expect("client builds"))
}

macro_rules! require_client {
    () => {
        match client() {
            Some(client) => client,
            None => {
                eprintln!("skipping live test: GAMETORCH_API_KEY is not set");
                return;
            }
        }
    };
}

/// Returns the first project in the caller's scope, if any.
async fn first_project(client: &Client) -> Option<Uuid> {
    client
        .list_projects()
        .await
        .ok()?
        .projects
        .first()
        .map(|project| project.id)
}

#[tokio::test]
async fn health_is_ok() {
    let client = require_client!();
    let status = client.health().await.expect("health request");
    assert!(!status.is_empty());
}

#[tokio::test]
async fn catalogs_decode() {
    let client = require_client!();

    let sprites = client.sprite_models().await.expect("sprite models");
    assert!(!sprites.image_models.is_empty());

    let sounds = client.sound_models().await.expect("sound models");
    assert!(!sounds.formats.is_empty());

    let animations = client.animation_models().await.expect("animation models");
    assert!(!animations.data.is_empty());
}

#[tokio::test]
async fn list_projects_decodes() {
    let client = require_client!();
    let projects = client.list_projects().await.expect("projects");
    for project in &projects.projects {
        assert!(!project.slug.is_empty());
    }
}

#[tokio::test]
async fn usage_decodes() {
    let client = require_client!();
    let usage = client.usage(None, None).await.expect("usage");
    assert!(usage.total >= 0);

    let histogram = client
        .usage_histogram(Some("24h"), None, None)
        .await
        .expect("histogram");
    assert_eq!(histogram.range, "24h");
}

#[tokio::test]
async fn generations_and_assets_decode() {
    let client = require_client!();
    let Some(project_id) = first_project(&client).await else {
        eprintln!("skipping: no projects");
        return;
    };

    let params = gametorch::ListParams::new().include_archived(true);
    let generations = client
        .list_generations(project_id, &params)
        .await
        .expect("generations");

    for generation in generations.generations.iter().take(3) {
        let fetched = client
            .get_generation(generation.id, true)
            .await
            .expect("get generation");
        assert_eq!(fetched.id, generation.id);

        if let Some(asset) = fetched.assets.first() {
            let asset = client.get_asset(asset.id).await.expect("get asset");
            assert_eq!(asset.id, fetched.assets[0].id);
            let content = client.asset_content(asset.id).await.expect("asset content");
            assert!(!content.is_empty());
            if asset.has_original {
                let original = client.asset_original(asset.id).await.expect("original");
                assert!(!original.is_empty());
            }
        }
    }

    let assets = client
        .list_sprite_assets(project_id, None)
        .await
        .expect("sprite assets");
    for asset in assets.assets.iter().take(3) {
        assert!(asset.width >= 0);
    }
}

#[tokio::test]
async fn sounds_decode() {
    let client = require_client!();
    let Some(project_id) = first_project(&client).await else {
        eprintln!("skipping: no projects");
        return;
    };

    let params = gametorch::ListParams::new().include_archived(true);
    let sounds = client
        .list_sound_generations(project_id, &params)
        .await
        .expect("sound generations");

    for generation in sounds.sound_generations.iter().take(3) {
        let fetched = client
            .get_sound_generation(generation.id, true)
            .await
            .expect("get sound generation");
        assert_eq!(fetched.id, generation.id);

        if let Some(asset) = fetched.assets.first() {
            let content = client
                .sound_asset_content(asset.id)
                .await
                .expect("sound content");
            assert!(!content.is_empty());
        }
    }
}

#[tokio::test]
async fn animations_and_exports_decode() {
    let client = require_client!();
    let Some(project_id) = first_project(&client).await else {
        eprintln!("skipping: no projects");
        return;
    };

    let params = gametorch::ListParams::new().include_archived(true);
    let runs = client
        .list_animation_runs(project_id, &params)
        .await
        .expect("animation runs");

    for run in runs.animations.iter().take(2) {
        let fetched = client
            .get_animation_run(run.id)
            .await
            .expect("get animation run");
        assert_eq!(fetched.id, run.id);
        // Provenance is present on every run.
        assert!(
            fetched
                .provenance
                .source
                .as_deref()
                .is_some_and(|s| !s.is_empty()),
            "animation run has no provenance source"
        );

        if !fetched.frames.is_empty() {
            let frame = &fetched.frames[0];
            let content = client.frame_content(frame.id).await.expect("frame content");
            assert!(!content.is_empty());

            let plan = client.export_plan(run.id, 1, 2).await.expect("export plan");
            assert!(plan.frame_count >= 0);
        }
    }

    // Filtering by base_asset_id returns only runs derived from that image.
    let base = runs
        .animations
        .iter()
        .find_map(|run| run.base_asset_id)
        .unwrap_or_else(uuid::Uuid::new_v4);
    let filtered = client
        .list_animation_runs(
            project_id,
            &gametorch::ListParams::new()
                .include_archived(true)
                .base_asset_id(base),
        )
        .await
        .expect("filter animation runs by base asset");
    assert!(filtered
        .animations
        .iter()
        .all(|run| run.base_asset_id == Some(base)));
    if runs
        .animations
        .iter()
        .any(|run| run.base_asset_id == Some(base))
    {
        assert!(!filtered.animations.is_empty());
    }
}

#[tokio::test]
async fn saved_animations_decode() {
    let client = require_client!();
    let Some(project_id) = first_project(&client).await else {
        eprintln!("skipping: no projects");
        return;
    };

    let params = gametorch::ListParams::new().include_archived(true);
    let saved = client
        .list_saved_animations(project_id, &params)
        .await
        .expect("saved animations");

    for animation in saved.saved_animations.iter().take(3) {
        let fetched = client
            .get_saved_animation(animation.id)
            .await
            .expect("get saved animation");
        assert_eq!(fetched.id, animation.id);
    }
}

#[tokio::test]
async fn labels_and_art_styles_decode() {
    let client = require_client!();
    let Some(project_id) = first_project(&client).await else {
        eprintln!("skipping: no projects");
        return;
    };

    let labels = client.list_labels(project_id).await.expect("labels");
    if let Some(label) = labels.labels.first() {
        let items = client.label_items(label.id).await.expect("label items");
        assert_eq!(items.label.id, label.id);
    }

    let styles = client
        .list_art_styles(project_id)
        .await
        .expect("art styles");
    for style in &styles.art_styles {
        assert!(!style.name.is_empty());
    }
}

#[tokio::test]
async fn keys_decode_or_forbidden() {
    let client = require_client!();
    match client.list_keys().await {
        Ok(keys) => {
            for key in &keys.keys {
                assert!(!key.key_prefix.is_empty());
            }
        }
        Err(err) if err.is_forbidden() => {
            // Organization API keys are admin-only; that's expected.
        }
        Err(err) => panic!("unexpected keys error: {err}"),
    }
}

#[tokio::test]
async fn animation_estimate_decodes() {
    let client = require_client!();
    let Some(project_id) = first_project(&client).await else {
        eprintln!("skipping: no projects");
        return;
    };

    let estimate = client
        .estimate_animation(project_id)
        .animation_model("ash")
        .duration(4)
        .send()
        .await
        .expect("estimate");
    assert_eq!(estimate.animation_model, "ash");
    assert_eq!(estimate.duration, 4);
    assert!(!estimate.resolution.is_empty());
}

#[tokio::test]
async fn unauthenticated_requests_are_rejected() {
    let Some(_) = client() else {
        eprintln!("skipping live test: GAMETORCH_API_KEY is not set");
        return;
    };

    let mut builder = Client::builder();
    if let Ok(base_url) = std::env::var("GAMETORCH_BASE_URL") {
        builder = builder.base_url(base_url);
    }
    let anonymous = builder.build().expect("client builds");

    let err = anonymous
        .list_projects()
        .await
        .expect_err("expected unauthorized");
    match err {
        Error::Api { status, .. } => assert_eq!(status.as_u16(), 401),
        other => panic!("expected API error, got {other:?}"),
    }
}
