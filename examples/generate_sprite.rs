//! Creates a new project, generates a single sprite in it, then names the
//! sprite, tags it with metadata and a label, and cleans those annotations up
//! again.
//!
//! **This example spends credits.** It creates a fresh project and deletes it
//! again at the end. Set `GAMETORCH_KEEP_PROJECT=1` to keep the project so you
//! can inspect it in the GameTorch UI.
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_... cargo run --example generate_sprite
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_KEEP_PROJECT=1 cargo run --example generate_sprite
//! ```

use std::collections::HashMap;
use std::time::Duration;

use gametorch::{Client, Project, SpriteMode};
use uuid::Uuid;

/// A unix-style file path used for the metadata round-trip.
const FILEPATH: &str = "/foo/bar/sprites/hero.png";

#[tokio::main]
async fn main() -> gametorch::Result<()> {
    let client = Client::from_env()?;

    let project = client
        .create_project(project_name("Sprite Example"))
        .await?;
    println!("created project '{}' ({})", project.name, project.slug);

    let result = run(&client, &project).await;

    finish(&client, &project.slug).await?;
    result
}

async fn run(client: &Client, project: &Project) -> gametorch::Result<()> {
    let models = client.sprite_models().await?;
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
        .await?;

    println!("generation {} is {}", job.id, job.status);

    // Poll until the generation reaches a terminal state.
    let generation = loop {
        let generation = client.get_generation(job.id, true).await?;
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
    println!("asset {} ({}x{})", asset.id, asset.width, asset.height);

    // Name the sprite.
    let named = client.rename_asset(asset.id, Some("Hero Fox")).await?;
    println!("named asset: {:?}", named.name);

    // Add metadata, then remove it again.
    let mut metadata = HashMap::new();
    metadata.insert("filepath".to_string(), FILEPATH.to_string());
    let updated = client.put_asset_metadata(asset.id, &metadata).await?;
    println!("metadata after add: {:?}", updated.metadata);
    let cleared = client.put_asset_metadata(asset.id, &HashMap::new()).await?;
    println!("metadata after remove: {:?}", cleared.metadata);
    // Re-apply so the project shows the metadata when kept.
    client.put_asset_metadata(asset.id, &metadata).await?;

    // Add a label, remove it (to prove the round-trip), then re-apply it.
    let label_name = format!("sprite-{}", &Uuid::new_v4().simple().to_string()[..8]);
    let label = client
        .create_label(project.id, &label_name, Some("#4caf50"))
        .await?;
    let associated = client.associate_asset_label(asset.id, &label.name).await?;
    println!("labels after add: {:?}", associated.labels);
    let removed = client.remove_asset_label(asset.id, &label.name).await?;
    println!("labels after remove: {:?}", removed.labels);
    client.associate_asset_label(asset.id, &label.name).await?;
    println!("kept label {:?}", label.name);

    // Download the trimmed PNG.
    let png = client.asset_content(asset.id).await?;
    println!(
        "downloaded {} bytes of {}",
        png.len(),
        png.content_type().unwrap_or("?")
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
