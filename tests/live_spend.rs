//! Opt-in live tests that **spend credits**.
//!
//! These generate real sprites, sounds and animations. Each test creates its own
//! project and deletes it again at the end, unless `GAMETORCH_KEEP_PROJECT=1` is
//! set (then the project is left behind so you can browse the generations,
//! names, labels, metadata and exports in the UI).
//!
//! They only run when `GAMETORCH_LIVE_SPEND=1` **and** `GAMETORCH_API_KEY` are
//! set.
//!
//! Approximate cost per full run: a single sprite (~5 credits), a sound effect
//! (~4 credits) and a 4-second `ash` animation (~73 credits) plus frame
//! generation (~3 credits). 100 credits = $1.
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_LIVE_SPEND=1 \
//! GAMETORCH_BASE_URL=http://localhost:8300/api \
//!   cargo test --test live_spend -- --test-threads=1 --nocapture
//!
//! # Keep the created projects for inspection:
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_LIVE_SPEND=1 GAMETORCH_KEEP_PROJECT=1 \
//!   cargo test --test live_spend -- --test-threads=1 --nocapture
//! ```

use std::collections::HashMap;
use std::time::Duration;

use gametorch::models::{AnimationRun, Generation, SoundGeneration};
use gametorch::{Client, Export, ExportFormat, Project, SpriteMode};
use uuid::Uuid;

/// Unix-style file paths used for the metadata round-trips.
const SPRITE_FILEPATH: &str = "/foo/bar/sprites/hero.png";
const SOUND_FILEPATH: &str = "/foo/bar/audio/sword-unsheath.mp3";
const ANIMATION_FILEPATH: &str = "/foo/bar/animations/sword-raise.aseprite";

const ALL_FORMATS: [ExportFormat; 8] = [
    ExportFormat::TexturePacker,
    ExportFormat::TexturePackerZip,
    ExportFormat::Aseprite,
    ExportFormat::Godot,
    ExportFormat::GodotZip,
    ExportFormat::Grid,
    ExportFormat::GameMaker,
    ExportFormat::SequenceZip,
];

fn client() -> Option<Client> {
    if std::env::var("GAMETORCH_LIVE_SPEND").ok().as_deref() != Some("1") {
        return None;
    }
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
                eprintln!("skipping spend test: set GAMETORCH_LIVE_SPEND=1 and GAMETORCH_API_KEY");
                return;
            }
        }
    };
}

fn short_name(prefix: &str) -> String {
    format!("{prefix}-{}", &Uuid::new_v4().simple().to_string()[..8])
}

fn project_name(kind: &str) -> String {
    format!("SDK {kind} {}", &Uuid::new_v4().simple().to_string()[..8])
}

fn keep_project() -> bool {
    std::env::var("GAMETORCH_KEEP_PROJECT").ok().as_deref() == Some("1")
}

async fn create_project(client: &Client, kind: &str) -> Option<Project> {
    match client.create_project(project_name(kind)).await {
        Ok(project) => {
            println!("created project '{}' ({})", project.name, project.slug);
            Some(project)
        }
        Err(err) if err.is_forbidden() => {
            eprintln!("skipping: this key cannot create projects");
            None
        }
        Err(err) => panic!("create project: {err}"),
    }
}

async fn finish_project(client: &Client, slug: &str) {
    if keep_project() {
        println!("keeping project '{slug}' (GAMETORCH_KEEP_PROJECT=1)");
    } else {
        client.delete_project(slug).await.expect("delete project");
        println!("deleted project '{slug}'");
    }
}

async fn wait_generation(client: &Client, id: Uuid) -> Generation {
    for _ in 0..120 {
        let generation = client
            .get_generation(id, true)
            .await
            .expect("get generation");
        if generation.status != "queued" && generation.status != "running" {
            return generation;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    panic!("timed out waiting for sprite generation {id}");
}

async fn wait_sound(client: &Client, id: Uuid) -> SoundGeneration {
    for _ in 0..120 {
        let generation = client
            .get_sound_generation(id, true)
            .await
            .expect("get sound generation");
        if generation.status != "queued" && generation.status != "running" {
            return generation;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    panic!("timed out waiting for sound generation {id}");
}

async fn wait_animation(client: &Client, id: Uuid) -> AnimationRun {
    for _ in 0..180 {
        let run = client
            .get_animation_run(id)
            .await
            .expect("get animation run");
        if run.status != "queued" && run.status != "running" {
            return run;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    panic!("timed out waiting for animation run {id}");
}

/// Waits until frame generation has settled (all frame runs terminal and the
/// frame count has stopped growing), so the full frame range is available.
async fn wait_for_frames(client: &Client, id: Uuid) -> AnimationRun {
    let mut last_len = 0usize;
    for _ in 0..180 {
        let run = client
            .get_animation_run(id)
            .await
            .expect("get animation run");
        let settled = run
            .frame_runs
            .iter()
            .all(|frame_run| frame_run.status != "queued" && frame_run.status != "running");
        if !run.frames.is_empty() && settled && run.frames.len() == last_len {
            return run;
        }
        last_len = run.frames.len();
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    client
        .get_animation_run(id)
        .await
        .expect("get animation run")
}

/// Creates a project, generates a sprite, names it, and round-trips metadata,
/// labels and archive state.
///
/// **Spends credits.**
#[tokio::test]
async fn sprite_generation_full_flow() {
    let client = require_client!();
    let Some(project) = create_project(&client, "Sprite Spend Test").await else {
        return;
    };

    let models = client.sprite_models().await.expect("sprite models");
    let image_model = models
        .image_models
        .iter()
        .find(|model| model.available)
        .expect("no available image models")
        .id
        .clone();

    let job = client
        .generate_sprite(project.id)
        .prompt("a friendly red fox, side view, game sprite")
        .mode(SpriteMode::Single)
        .image_model(image_model)
        .send()
        .await
        .expect("generate sprite");

    let generation = wait_generation(&client, job.id).await;
    assert_eq!(
        generation.status, "succeeded",
        "generation failed: {:?}",
        generation.error
    );
    let asset = generation
        .assets
        .first()
        .expect("at least one asset")
        .clone();

    // Naming.
    let named = client
        .rename_asset(asset.id, Some("Hero Fox"))
        .await
        .expect("rename asset");
    assert_eq!(named.name.as_deref(), Some("Hero Fox"));

    // Metadata add then remove.
    let mut metadata = HashMap::new();
    metadata.insert("filepath".to_string(), SPRITE_FILEPATH.to_string());
    let updated = client
        .put_asset_metadata(asset.id, &metadata)
        .await
        .expect("add metadata");
    assert_eq!(
        updated.metadata.get("filepath").map(String::as_str),
        Some(SPRITE_FILEPATH)
    );
    let cleared = client
        .put_asset_metadata(asset.id, &HashMap::new())
        .await
        .expect("remove metadata");
    assert!(cleared.metadata.is_empty());
    // Re-apply so the kept project shows the metadata in the UI.
    client
        .put_asset_metadata(asset.id, &metadata)
        .await
        .expect("re-add metadata");

    // Labels add then remove.
    let label = client
        .create_label(project.id, short_name("sprite"), None)
        .await
        .expect("create label");
    let associated = client
        .associate_asset_label(asset.id, &label.name)
        .await
        .expect("associate label");
    assert!(associated.labels.iter().any(|name| name == &label.name));
    let removed = client
        .remove_asset_label(asset.id, &label.name)
        .await
        .expect("remove label");
    assert!(!removed.labels.iter().any(|name| name == &label.name));
    // Re-apply and keep the label so the kept project shows it in the UI.
    client
        .associate_asset_label(asset.id, &label.name)
        .await
        .expect("re-associate label");

    // Archive / unarchive.
    let archived = client.archive_asset(asset.id).await.expect("archive");
    assert!(archived.archived_at.is_some());
    let unarchived = client.unarchive_asset(asset.id).await.expect("unarchive");
    assert!(unarchived.archived_at.is_none());

    // Content is downloadable.
    let content = client.asset_content(asset.id).await.expect("asset content");
    assert!(!content.is_empty());

    finish_project(&client, &project.slug).await;
}

/// Creates a project, generates a sound effect, names it, and round-trips
/// metadata, labels and archive state.
///
/// **Spends credits.**
#[tokio::test]
async fn sound_generation_full_flow() {
    let client = require_client!();
    let Some(project) = create_project(&client, "Sound Spend Test").await else {
        return;
    };

    let models = client.sound_models().await.expect("sound models");
    let job = client
        .generate_sound(project.id)
        .prompt("a single sword unsheathing, then a heavy metal thud")
        .sound_model(&models.model.id)
        .response_format(&models.default_format)
        .send()
        .await
        .expect("generate sound");

    let generation = wait_sound(&client, job.id).await;
    assert_eq!(
        generation.status, "succeeded",
        "generation failed: {:?}",
        generation.error
    );
    let asset = generation
        .assets
        .first()
        .expect("at least one asset")
        .clone();

    let named = client
        .rename_sound_asset(asset.id, Some("Sword Unsheath"))
        .await
        .expect("rename sound");
    assert_eq!(named.name.as_deref(), Some("Sword Unsheath"));

    let mut metadata = HashMap::new();
    metadata.insert("filepath".to_string(), SOUND_FILEPATH.to_string());
    let updated = client
        .put_sound_asset_metadata(asset.id, &metadata)
        .await
        .expect("add metadata");
    assert_eq!(
        updated.metadata.get("filepath").map(String::as_str),
        Some(SOUND_FILEPATH)
    );
    let cleared = client
        .put_sound_asset_metadata(asset.id, &HashMap::new())
        .await
        .expect("remove metadata");
    assert!(cleared.metadata.is_empty());
    // Re-apply so the kept project shows the metadata in the UI.
    client
        .put_sound_asset_metadata(asset.id, &metadata)
        .await
        .expect("re-add metadata");

    let label = client
        .create_label(project.id, short_name("sound"), None)
        .await
        .expect("create label");
    let associated = client
        .associate_sound_label(asset.id, &label.name)
        .await
        .expect("associate label");
    assert!(associated.labels.iter().any(|name| name == &label.name));
    let removed = client
        .remove_sound_label(asset.id, &label.name)
        .await
        .expect("remove label");
    assert!(!removed.labels.iter().any(|name| name == &label.name));
    // Re-apply and keep the label so the kept project shows it in the UI.
    client
        .associate_sound_label(asset.id, &label.name)
        .await
        .expect("re-associate label");

    let archived = client
        .archive_sound_asset(asset.id)
        .await
        .expect("archive sound asset");
    assert!(archived.archived_at.is_some());
    let unarchived = client
        .unarchive_sound_asset(asset.id)
        .await
        .expect("unarchive sound asset");
    assert!(unarchived.archived_at.is_none());

    let content = client
        .sound_asset_content(asset.id)
        .await
        .expect("sound content");
    assert!(!content.is_empty());

    finish_project(&client, &project.slug).await;
}

/// Creates a project, generates an animation and its frames, saves a sub-range
/// preset, names it, round-trips metadata/labels/archive state, and exports
/// every format.
///
/// **Spends credits** (the most expensive test).
#[tokio::test]
async fn animation_generation_exports_and_saved_animation() {
    let client = require_client!();
    let Some(project) = create_project(&client, "Animation Spend Test").await else {
        return;
    };

    let job = client
        .generate_animation(project.id)
        .prompt("the hero draws her sword and raises it overhead")
        .animation_model("ash")
        .duration(4)
        .send()
        .await
        .expect("generate animation");

    let mut run = wait_animation(&client, job.id).await;
    assert_eq!(run.status, "succeeded", "animation failed: {:?}", run.error);

    // Frames usually generate automatically once the run succeeds; wait for
    // that to settle before saving a range, otherwise only the first frame may
    // be present.
    if run.frames.is_empty() {
        client
            .generate_frames(project.id, run.id)
            .fps(12)
            .send()
            .await
            .expect("generate frames");
    }
    run = wait_for_frames(&client, run.id).await;
    assert!(!run.frames.is_empty(), "animation produced no frames");

    // Save a sub-range (not the whole clip).
    let last = run
        .frames
        .last()
        .map(|frame| frame.frame_number)
        .unwrap_or(1);
    let start_frame = 2.min(last);
    let end_frame = (last - 1).max(start_frame);
    let saved = client
        .save_animation(project.id)
        .generation_id(run.id)
        .range(start_frame, end_frame)
        .name("Sword Raise")
        .send()
        .await
        .expect("save animation");
    assert_eq!(saved.generation_id, run.id);
    assert_eq!(saved.start_frame, start_frame);
    assert_eq!(saved.end_frame, end_frame);

    // Naming.
    let renamed = client
        .rename_saved_animation(saved.id, Some("Sword Raise (named)"))
        .await
        .expect("rename saved animation");
    assert_eq!(renamed.name.as_deref(), Some("Sword Raise (named)"));

    // Metadata add then remove.
    let mut metadata = HashMap::new();
    metadata.insert("filepath".to_string(), ANIMATION_FILEPATH.to_string());
    let tagged = client
        .put_saved_animation_metadata(saved.id, &metadata)
        .await
        .expect("add metadata");
    assert_eq!(
        tagged.metadata.get("filepath").map(String::as_str),
        Some(ANIMATION_FILEPATH)
    );
    let cleared = client
        .put_saved_animation_metadata(saved.id, &HashMap::new())
        .await
        .expect("remove metadata");
    assert!(cleared.metadata.is_empty());
    // Re-apply so the kept project shows the metadata in the UI.
    client
        .put_saved_animation_metadata(saved.id, &metadata)
        .await
        .expect("re-add metadata");

    // Labels add then remove.
    let label = client
        .create_label(project.id, short_name("anim"), None)
        .await
        .expect("create label");
    let associated = client
        .associate_saved_animation_label(saved.id, &label.name)
        .await
        .expect("associate label");
    assert!(associated.labels.iter().any(|name| name == &label.name));
    let removed = client
        .remove_saved_animation_label(saved.id, &label.name)
        .await
        .expect("remove label");
    assert!(!removed.labels.iter().any(|name| name == &label.name));
    // Re-apply and keep the label so the kept project shows it in the UI.
    client
        .associate_saved_animation_label(saved.id, &label.name)
        .await
        .expect("re-associate label");

    // Archive / unarchive.
    let archived = client
        .archive_saved_animation(saved.id)
        .await
        .expect("archive saved animation");
    assert!(archived.archived_at.is_some());
    let unarchived = client
        .unarchive_saved_animation(saved.id)
        .await
        .expect("unarchive saved animation");
    assert!(unarchived.archived_at.is_none());

    // A single frame is downloadable by number.
    let frame = client
        .frame_content_by_number(run.id, start_frame)
        .await
        .expect("frame content");
    assert!(!frame.is_empty());

    // Export every supported format.
    for format in ALL_FORMATS {
        let export = client
            .export(run.id, format, start_frame as i32, end_frame as i32)
            .await
            .unwrap_or_else(|err| panic!("export {format}: {err}"));
        match export {
            Export::Binary(download) => assert!(!download.is_empty(), "{format} was empty"),
            Export::TexturePacker(json) => {
                assert!(
                    !json.image_base64.is_empty(),
                    "texturepacker image was empty"
                );
                assert!(!json.json_filename.is_empty());
            }
            Export::Godot(json) => {
                assert!(!json.tres.is_empty(), "godot resource was empty");
                assert!(!json.image_base64.is_empty(), "godot image was empty");
            }
        }
    }

    finish_project(&client, &project.slug).await;
}
