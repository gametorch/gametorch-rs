//! Creates a new project, generates a sprite, animates **that exact sprite**
//! by passing its asset id as `base_asset_id`, generates the animation's
//! frames, saves a sub-range as a named preset with metadata and a label, then
//! exports it in every supported format.
//!
//! Animating an existing sprite is done by referencing it: pass the sprite
//! asset id (from `Client::list_sprite_assets`, or `GET
//! /projects/{id}/sprite-assets`) as `base_asset_id`. Omit it to generate an
//! animation from scratch, which will not resemble any particular sprite.
//!
//! **This example spends credits** (one sprite, animation generation and, if
//! needed, frame generation). It creates a fresh project and deletes it again
//! at the end. Set `GAMETORCH_KEEP_PROJECT=1` to keep the project so you can
//! inspect it in the GameTorch UI.
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_... cargo run --example generate_animation
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_KEEP_PROJECT=1 cargo run --example generate_animation
//! ```

use std::collections::HashMap;
use std::time::Duration;

use gametorch::{Client, Export, ExportFormat, Project, SpriteMode};
use uuid::Uuid;

/// A unix-style file path used for the metadata round-trip.
const FILEPATH: &str = "/foo/bar/animations/sword-raise.aseprite";

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

#[tokio::main]
async fn main() -> gametorch::Result<()> {
    let client = Client::from_env()?;

    let project = client
        .create_project(project_name("Animation Example"))
        .await?;
    println!("created project '{}' ({})", project.name, project.slug);

    let result = run(&client, &project).await;

    finish(&client, &project.slug).await?;
    result
}

async fn run(client: &Client, project: &Project) -> gametorch::Result<()> {
    // 1. Generate a sprite and grab its asset id — this is the reference image
    //    the animation will be built from.
    let models = client.sprite_models().await?;
    let image_model = models
        .image_models
        .iter()
        .find(|model| model.available)
        .expect("no available image models")
        .id
        .clone();

    let sprite_job = client
        .generate_sprite(project.id)
        .prompt("a knight in armor, side view, game sprite")
        .mode(SpriteMode::Single)
        .image_model(image_model)
        .send()
        .await?;
    println!(
        "sprite generation {} is {}",
        sprite_job.id, sprite_job.status
    );

    let generation = wait_for_generation(client, sprite_job.id).await?;
    let base_asset_id = generation
        .assets
        .first()
        .expect("sprite generation produced no assets")
        .id;
    println!("using sprite asset {base_asset_id} as the animation reference");

    // Optional: check the cost first. The estimate takes the same reference.
    let estimate = client
        .estimate_animation(project.id)
        .animation_model("ash")
        .duration(4)
        .base_asset_id(base_asset_id)
        .send()
        .await?;
    println!(
        "estimated {} credits (~${}) for a 4s ash animation",
        estimate.credits, estimate.usd
    );

    // 2. Animate that exact sprite by passing its asset id as `base_asset_id`.
    //    Omitting this call would generate an unrelated from-scratch animation.
    let job = client
        .generate_animation(project.id)
        .prompt("the hero draws her sword and raises it overhead")
        .animation_model("ash")
        .duration(4)
        .base_asset_id(base_asset_id)
        .send()
        .await?;
    println!("animation {} is {}", job.id, job.status);

    let mut run = wait_for_animation(client, job.id).await?;
    println!(
        "animation {} finished with {} frame(s) (base_asset_id={:?})",
        run.id,
        run.frames.len(),
        run.base_asset_id
    );

    // Frames usually generate automatically on success; wait for that to
    // settle before saving a range, otherwise only the first frame may exist.
    if run.frames.is_empty() {
        let frames_job = client
            .generate_frames(project.id, run.id)
            .fps(12)
            .send()
            .await?;
        println!(
            "frame generation {} is {}",
            frames_job.id, frames_job.status
        );
    }
    run = wait_for_frames(client, run.id).await?;

    if run.frames.is_empty() {
        println!("no frames available; skipping saved animation and exports");
        return Ok(());
    }

    // Save a sub-range (not the whole clip) as a named preset.
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
        .await?;
    println!(
        "saved animation {} ({} frames, {}-{})",
        saved.id, saved.frame_count, saved.start_frame, saved.end_frame
    );

    // Name it, add/remove metadata, and add/remove a label.
    let renamed = client
        .rename_saved_animation(saved.id, Some("Sword Raise (named)"))
        .await?;
    println!("renamed saved animation: {:?}", renamed.name);

    let mut metadata = HashMap::new();
    metadata.insert("filepath".to_string(), FILEPATH.to_string());
    let tagged = client
        .put_saved_animation_metadata(saved.id, &metadata)
        .await?;
    println!("metadata after add: {:?}", tagged.metadata);
    let cleared = client
        .put_saved_animation_metadata(saved.id, &HashMap::new())
        .await?;
    println!("metadata after remove: {:?}", cleared.metadata);
    // Re-apply so the project shows the metadata when kept.
    client
        .put_saved_animation_metadata(saved.id, &metadata)
        .await?;

    let label_name = format!("anim-{}", &Uuid::new_v4().simple().to_string()[..8]);
    let label = client
        .create_label(project.id, &label_name, Some("#ff9800"))
        .await?;
    let associated = client
        .associate_saved_animation_label(saved.id, &label.name)
        .await?;
    println!("labels after add: {:?}", associated.labels);
    let removed = client
        .remove_saved_animation_label(saved.id, &label.name)
        .await?;
    println!("labels after remove: {:?}", removed.labels);
    client
        .associate_saved_animation_label(saved.id, &label.name)
        .await?;
    println!("kept label {:?}", label.name);

    // Download a single frame by number.
    let frame = client.frame_content_by_number(run.id, start_frame).await?;
    println!("frame {start_frame}: {} bytes", frame.len());

    // Export the saved range in every supported format.
    println!("\nexports ({start_frame}-{end_frame}):");
    for format in ALL_FORMATS {
        let export = client
            .export(run.id, format, start_frame as i32, end_frame as i32)
            .await?;
        match export {
            Export::Binary(download) => {
                println!(
                    "  {format:<18} {} bytes ({})",
                    download.len(),
                    download.content_type().unwrap_or("?")
                );
            }
            Export::TexturePacker(json) => {
                println!(
                    "  {format:<18} json, image {} bytes, atlas {}",
                    json.image_base64.len(),
                    json.json_filename
                );
            }
            Export::Godot(json) => {
                println!(
                    "  {format:<18} json, image {} bytes, resource {}",
                    json.image_base64.len(),
                    json.tres_filename
                );
            }
        }
    }

    Ok(())
}

async fn wait_for_generation(
    client: &Client,
    generation_id: Uuid,
) -> gametorch::Result<gametorch::Generation> {
    loop {
        let generation = client.get_generation(generation_id, true).await?;
        println!("sprite status: {}", generation.status);
        if generation.status != "queued" && generation.status != "running" {
            return Ok(generation);
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn wait_for_animation(
    client: &Client,
    run_id: Uuid,
) -> gametorch::Result<gametorch::AnimationRun> {
    loop {
        let run = client.get_animation_run(run_id).await?;
        println!("status: {}", run.status);
        if run.status != "queued" && run.status != "running" {
            return Ok(run);
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn wait_for_frames(
    client: &Client,
    run_id: Uuid,
) -> gametorch::Result<gametorch::AnimationRun> {
    let mut last_len = 0usize;
    for _ in 0..180 {
        let run = client.get_animation_run(run_id).await?;
        let settled = run
            .frame_runs
            .iter()
            .all(|frame_run| frame_run.status != "queued" && frame_run.status != "running");
        if !run.frames.is_empty() && settled && run.frames.len() == last_len {
            return Ok(run);
        }
        last_len = run.frames.len();
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    client.get_animation_run(run_id).await
}

fn project_name(kind: &str) -> String {
    format!("SDK {kind} {}", &Uuid::new_v4().simple().to_string()[..8])
}

async fn finish(client: &Client, slug: &str) -> gametorch::Result<()> {
    if std::env::var("GAMETORCH_KEEP_PROJECT").ok().as_deref() == Some("1") {
        println!("keeping project '{slug}' (GAMETORCH_KEEP_PROJECT=1)");
    } else {
        client.delete_project(slug).await?;
        println!("deleted project '{slug}'");
    }
    Ok(())
}
