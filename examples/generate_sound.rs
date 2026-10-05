//! Creates a new project, generates a sound effect in it, then names the sound,
//! tags it with metadata and a label, and cleans those annotations up again.
//!
//! **This example spends credits.** It creates a fresh project and deletes it
//! again at the end. Set `GAMETORCH_KEEP_PROJECT=1` to keep the project so you
//! can inspect it in the GameTorch UI.
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_... cargo run --example generate_sound
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_KEEP_PROJECT=1 cargo run --example generate_sound
//! ```

use std::collections::HashMap;
use std::time::Duration;

use gametorch::{Client, Project};
use uuid::Uuid;

/// A unix-style file path used for the metadata round-trip.
const FILEPATH: &str = "/foo/bar/audio/sword-unsheath.mp3";

#[tokio::main]
async fn main() -> gametorch::Result<()> {
    let client = Client::from_env()?;

    let project = client.create_project(project_name("Sound Example")).await?;
    println!("created project '{}' ({})", project.name, project.slug);

    let result = run(&client, &project).await;

    finish(&client, &project.slug).await?;
    result
}

async fn run(client: &Client, project: &Project) -> gametorch::Result<()> {
    let models = client.sound_models().await?;

    let job = client
        .generate_sound(project.id)
        .prompt("a single sword unsheathing, then a heavy metal thud")
        .sound_model(&models.model.id)
        .response_format(&models.default_format)
        .send()
        .await?;

    println!("sound generation {} is {}", job.id, job.status);

    let generation = loop {
        let generation = client.get_sound_generation(job.id, true).await?;
        println!("status: {}", generation.status);
        if generation.status != "queued" && generation.status != "running" {
            break generation;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    let Some(asset) = generation.assets.first() else {
        println!(
            "generation produced no assets (status: {})",
            generation.status
        );
        return Ok(());
    };
    println!("asset {} ({})", asset.id, asset.format);

    // Name the sound.
    let named = client
        .rename_sound_asset(asset.id, Some("Sword Unsheath"))
        .await?;
    println!("named sound: {:?}", named.name);

    // Add metadata, then remove it again.
    let mut metadata = HashMap::new();
    metadata.insert("filepath".to_string(), FILEPATH.to_string());
    let updated = client.put_sound_asset_metadata(asset.id, &metadata).await?;
    println!("metadata after add: {:?}", updated.metadata);
    let cleared = client
        .put_sound_asset_metadata(asset.id, &HashMap::new())
        .await?;
    println!("metadata after remove: {:?}", cleared.metadata);
    // Re-apply so the project shows the metadata when kept.
    client.put_sound_asset_metadata(asset.id, &metadata).await?;

    // Add a label, remove it (to prove the round-trip), then re-apply it.
    let label_name = format!("sound-{}", &Uuid::new_v4().simple().to_string()[..8]);
    let label = client
        .create_label(project.id, &label_name, Some("#2196f3"))
        .await?;
    let associated = client.associate_sound_label(asset.id, &label.name).await?;
    println!("labels after add: {:?}", associated.labels);
    let removed = client.remove_sound_label(asset.id, &label.name).await?;
    println!("labels after remove: {:?}", removed.labels);
    client.associate_sound_label(asset.id, &label.name).await?;
    println!("kept label {:?}", label.name);

    // Download the audio.
    let audio = client.sound_asset_content(asset.id).await?;
    println!(
        "downloaded {} bytes of {}",
        audio.len(),
        audio.content_type().unwrap_or("?")
    );

    Ok(())
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
