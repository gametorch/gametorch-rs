//! Exports an animation run in every supported format, writing the results to
//! `./gametorch-exports/`.
//!
//! This example is read-only and does **not** spend credits, but the run it
//! exports must already exist. Set `GAMETORCH_RUN` to a run UUID, or it will
//! pick the first run with frames it can find.
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_BASE_URL=http://localhost:8300/api \
//!   cargo run --example export_animation
//! ```

use std::path::PathBuf;

use gametorch::{Client, Export, ExportFormat, ListParams};
use uuid::Uuid;

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
    let out_dir = PathBuf::from("gametorch-exports");
    std::fs::create_dir_all(&out_dir).expect("create output directory");

    let (run_id, last_frame) = match std::env::var("GAMETORCH_RUN") {
        Ok(value) => {
            let run_id: Uuid = value.parse().expect("GAMETORCH_RUN must be a UUID");
            let run = client.get_animation_run(run_id).await?;
            let last = run
                .frames
                .last()
                .map(|frame| frame.frame_number)
                .unwrap_or(1);
            (run_id, last)
        }
        Err(_) => find_run_with_frames(&client).await?,
    };

    let end = last_frame.min(3) as i32;
    println!(
        "exporting run {run_id}, frames 1-{end} to {}",
        out_dir.display()
    );

    for format in ALL_FORMATS {
        let export = client.export(run_id, format, 1, end).await?;
        let path = out_dir.join(filename(format));
        match export {
            Export::Binary(download) => {
                std::fs::write(&path, download.as_bytes()).expect("write export");
                println!(
                    "  {format:<18} -> {} ({} bytes)",
                    path.display(),
                    download.len()
                );
            }
            Export::TexturePacker(json) => {
                let text = serde_json::to_string_pretty(&json).expect("serialize");
                std::fs::write(&path, text).expect("write export");
                println!("  {format:<18} -> {}", path.display());
            }
            Export::Godot(json) => {
                let text = serde_json::to_string_pretty(&json).expect("serialize");
                std::fs::write(&path, text).expect("write export");
                println!("  {format:<18} -> {}", path.display());
            }
        }
    }

    Ok(())
}

fn filename(format: ExportFormat) -> String {
    match format {
        ExportFormat::TexturePacker => "texturepacker.json".into(),
        ExportFormat::TexturePackerZip => "texturepacker.zip".into(),
        ExportFormat::Aseprite => "animation.aseprite".into(),
        ExportFormat::Godot => "godot.json".into(),
        ExportFormat::GodotZip => "godot.zip".into(),
        ExportFormat::Grid => "grid.png".into(),
        ExportFormat::GameMaker => "gamemaker.png".into(),
        ExportFormat::SequenceZip => "sequence.zip".into(),
    }
}

async fn find_run_with_frames(client: &Client) -> gametorch::Result<(Uuid, i64)> {
    let params = ListParams::new().include_archived(true);
    for project in client.list_projects().await?.projects {
        for run in client
            .list_animation_runs(project.id, &params)
            .await?
            .animations
        {
            if let Some(frame) = run.frames.last() {
                return Ok((run.id, frame.frame_number));
            }
        }
    }
    panic!("no animation run with frames found; set GAMETORCH_RUN to a run UUID");
}
