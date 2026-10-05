//! Opt-in live CLI tests that **spend credits**.
//!
//! Enabled with `GAMETORCH_LIVE_SPEND=1` plus `GAMETORCH_API_KEY`. Each test
//! creates its own project and deletes it at the end unless
//! `GAMETORCH_KEEP_PROJECT=1`. Approximate cost per run: sprite ~5 credits,
//! sound ~4 credits, 4s `ash` animation ~73 credits plus frame generation.

mod common;

use std::path::PathBuf;

use common::{
    create_project, env_enabled, finish_project, has_api_key, run_json, short, wait_animation,
    wait_frames, wait_sound, wait_sprite,
};

macro_rules! require {
    () => {
        if !has_api_key() || !env_enabled("GAMETORCH_LIVE_SPEND") {
            eprintln!("skipping: set GAMETORCH_LIVE_SPEND=1 and GAMETORCH_API_KEY");
            return;
        }
    };
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gametorch-cli-{}", short()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn first_available_model(kind: &str) -> String {
    let value = run_json(&["catalog", kind]);
    let models = value["image_models"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    models
        .iter()
        .find(|model| model["available"] == true)
        .or_else(|| models.first())
        .map(|model| model["id"].as_str().unwrap().to_string())
        .expect("no image models")
}

fn label_id(project: &str, prefix: &str) -> String {
    let label = run_json(&[
        "label",
        "create",
        "--project",
        project,
        "--name",
        &format!("cli-{prefix}-{}", short()),
    ]);
    label["id"].as_str().unwrap().to_string()
}

#[test]
fn sprite_generation_full_flow() {
    require!();
    let (project, slug) = create_project("Sprite Spend Test");
    let image_model = first_available_model("sprite");

    let job = run_json(&[
        "sprite",
        "generate",
        "--project",
        &project,
        "--prompt",
        "a friendly red fox, side view, game sprite",
        "--mode",
        "single",
        "--image-model",
        &image_model,
    ]);
    let generation = wait_sprite(job["id"].as_str().unwrap());
    assert_eq!(generation["status"], "succeeded");
    let asset = generation["assets"][0]["id"].as_str().unwrap().to_string();

    // Naming.
    let named = run_json(&["sprite", "asset", "rename", &asset, "Hero Fox"]);
    assert_eq!(named["name"], "Hero Fox");

    // Metadata add, remove, then re-apply for browsing.
    let tagged = run_json(&[
        "sprite",
        "asset",
        "metadata",
        &asset,
        "--metadata",
        "filepath=/foo/bar/sprites/hero.png",
    ]);
    assert_eq!(tagged["metadata"]["filepath"], "/foo/bar/sprites/hero.png");
    let cleared = run_json(&["sprite", "asset", "metadata-clear", &asset]);
    assert!(cleared["metadata"].as_object().unwrap().is_empty());
    run_json(&[
        "sprite",
        "asset",
        "metadata",
        &asset,
        "--metadata",
        "filepath=/foo/bar/sprites/hero.png",
    ]);

    // Labels add, remove, then re-apply.
    let label = label_id(&project, "sprite");
    // Use the label's actual name by listing it.
    let labels = run_json(&["label", "list", "--project", &project]);
    let label_name = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"].as_str() == Some(label.as_str()))
        .map(|l| l["name"].as_str().unwrap().to_string())
        .unwrap();
    let associated = run_json(&[
        "label",
        "associate",
        "--asset",
        &asset,
        "--name",
        &label_name,
    ]);
    assert!(associated["labels"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == &serde_json::Value::String(label_name.clone())));
    let removed = run_json(&["label", "remove", "--asset", &asset, "--name", &label_name]);
    assert!(!removed["labels"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == &serde_json::Value::String(label_name.clone())));
    run_json(&[
        "label",
        "associate",
        "--asset",
        &asset,
        "--name",
        &label_name,
    ]);

    // Archive / unarchive.
    let archived = run_json(&["sprite", "asset", "archive", &asset]);
    assert!(archived["archived_at"].is_string());
    let unarchived = run_json(&["sprite", "asset", "unarchive", &asset]);
    assert!(unarchived["archived_at"].is_null());

    // Download content.
    let dir = temp_dir();
    let path = dir.join("hero.png");
    common::run_ok(&[
        "sprite",
        "asset",
        "content",
        &asset,
        "--output",
        path.to_str().unwrap(),
    ]);
    assert!(std::fs::metadata(&path).unwrap().len() > 0);

    finish_project(&slug);
}

#[test]
fn sound_generation_full_flow() {
    require!();
    let (project, slug) = create_project("Sound Spend Test");
    let models = run_json(&["catalog", "sound"]);
    let sound_model = models["model"]["id"].as_str().unwrap().to_string();
    let format = models["default_format"].as_str().unwrap().to_string();

    let job = run_json(&[
        "sound",
        "generate",
        "--project",
        &project,
        "--prompt",
        "a single sword unsheathing, then a heavy metal thud",
        "--sound-model",
        &sound_model,
        "--response-format",
        &format,
    ]);
    let generation = wait_sound(job["id"].as_str().unwrap());
    assert_eq!(generation["status"], "succeeded");
    let asset = generation["assets"][0]["id"].as_str().unwrap().to_string();

    let named = run_json(&["sound", "asset", "rename", &asset, "Sword Unsheath"]);
    assert_eq!(named["name"], "Sword Unsheath");

    let tagged = run_json(&[
        "sound",
        "asset",
        "metadata",
        &asset,
        "--metadata",
        "filepath=/foo/bar/audio/sword-unsheath.mp3",
    ]);
    assert_eq!(
        tagged["metadata"]["filepath"],
        "/foo/bar/audio/sword-unsheath.mp3"
    );
    run_json(&["sound", "asset", "metadata-clear", &asset]);
    run_json(&[
        "sound",
        "asset",
        "metadata",
        &asset,
        "--metadata",
        "filepath=/foo/bar/audio/sword-unsheath.mp3",
    ]);

    let label = label_id(&project, "sound");
    let labels = run_json(&["label", "list", "--project", &project]);
    let label_name = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"].as_str() == Some(label.as_str()))
        .map(|l| l["name"].as_str().unwrap().to_string())
        .unwrap();
    run_json(&[
        "label",
        "associate",
        "--sound",
        &asset,
        "--name",
        &label_name,
    ]);

    let dir = temp_dir();
    let path = dir.join("sound.mp3");
    common::run_ok(&[
        "sound",
        "asset",
        "content",
        &asset,
        "--output",
        path.to_str().unwrap(),
    ]);
    assert!(std::fs::metadata(&path).unwrap().len() > 0);

    finish_project(&slug);
}

#[test]
fn animation_generation_exports_and_saved_animation() {
    require!();
    let (project, slug) = create_project("Animation Spend Test");

    let job = run_json(&[
        "animation",
        "generate",
        "--project",
        &project,
        "--prompt",
        "the hero draws her sword and raises it overhead",
        "--animation-model",
        "ash",
        "--duration",
        "4",
    ]);
    let id = job["id"].as_str().unwrap().to_string();
    let run = wait_animation(&id);
    assert_eq!(run["status"], "succeeded");

    if run["frames"]
        .as_array()
        .map(|f| f.is_empty())
        .unwrap_or(true)
    {
        run_json(&[
            "animation",
            "frames",
            "generate",
            "--project",
            &project,
            "--run",
            &id,
            "--fps",
            "12",
        ]);
    }
    let run = wait_frames(&id);
    let frames = run["frames"].as_array().unwrap();
    assert!(!frames.is_empty(), "animation produced no frames");
    let last = frames
        .iter()
        .map(|f| f["frame_number"].as_i64().unwrap())
        .max()
        .unwrap();
    let start_frame = 2.min(last);
    let end_frame = (last - 1).max(start_frame);

    // Save a sub-range.
    let saved = run_json(&[
        "saved",
        "save",
        "--project",
        &project,
        "--generation-id",
        &id,
        "--start-frame",
        &start_frame.to_string(),
        "--end-frame",
        &end_frame.to_string(),
        "--name",
        "CLI Sword Raise",
    ]);
    assert_eq!(saved["start_frame"].as_i64().unwrap(), start_frame);
    assert_eq!(saved["end_frame"].as_i64().unwrap(), end_frame);
    let saved_id = saved["id"].as_str().unwrap().to_string();

    // Naming, metadata, labels.
    let renamed = run_json(&["saved", "rename", &saved_id, "CLI Sword Raise (named)"]);
    assert_eq!(renamed["name"], "CLI Sword Raise (named)");
    let tagged = run_json(&[
        "saved",
        "metadata",
        &saved_id,
        "--metadata",
        "filepath=/foo/bar/animations/sword-raise.aseprite",
    ]);
    assert_eq!(
        tagged["metadata"]["filepath"],
        "/foo/bar/animations/sword-raise.aseprite"
    );
    let label = label_id(&project, "anim");
    let labels = run_json(&["label", "list", "--project", &project]);
    let label_name = labels["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"].as_str() == Some(label.as_str()))
        .map(|l| l["name"].as_str().unwrap().to_string())
        .unwrap();
    run_json(&[
        "label",
        "associate",
        "--saved-animation",
        &saved_id,
        "--name",
        &label_name,
    ]);

    // Archive / unarchive.
    let archived = run_json(&["saved", "archive", &saved_id]);
    assert!(archived["archived_at"].is_string());
    run_json(&["saved", "unarchive", &saved_id]);

    // Frame content by number.
    let dir = temp_dir();
    let frame_path = dir.join("frame.png");
    common::run_ok(&[
        "animation",
        "frames",
        "content-by-number",
        "--run",
        &id,
        "--number",
        &start_frame.to_string(),
        "--output",
        frame_path.to_str().unwrap(),
    ]);
    assert!(std::fs::metadata(&frame_path).unwrap().len() > 0);

    // Export every format.
    for format in [
        "texturepacker",
        "texturepacker.zip",
        "aseprite",
        "godot",
        "godot.zip",
        "grid",
        "gamemaker",
        "sequence.zip",
    ] {
        let path = dir.join(format!("export-{format}"));
        common::run_ok(&[
            "animation",
            "export",
            &id,
            "--format",
            format,
            "--start-frame",
            &start_frame.to_string(),
            "--end-frame",
            &end_frame.to_string(),
            "--output",
            path.to_str().unwrap(),
        ]);
        assert!(
            std::fs::metadata(&path).unwrap().len() > 0,
            "export {format} was empty"
        );
    }

    finish_project(&slug);
}
