//! Live read-only CLI tests. Require `GAMETORCH_API_KEY`.

mod common;

use common::{has_api_key, run_json};

macro_rules! require_key {
    () => {
        if !has_api_key() {
            eprintln!("skipping live test: GAMETORCH_API_KEY is not set");
            return;
        }
    };
}

fn first_project() -> Option<String> {
    run_json(&["project", "list"])["projects"]
        .as_array()
        .and_then(|projects| projects.first())
        .and_then(|project| project["id"].as_str())
        .map(str::to_string)
}

#[test]
fn health_is_ok() {
    require_key!();
    let out = common::run_ok(&["health"]);
    assert!(out.contains("ok"));
}

#[test]
fn catalogs_decode() {
    require_key!();
    assert!(!run_json(&["catalog", "sprite"])["image_models"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!run_json(&["catalog", "sound"])["formats"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!run_json(&["catalog", "animation"])["data"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn list_projects_decodes() {
    require_key!();
    let value = run_json(&["project", "list"]);
    assert!(value["projects"].is_array());
}

#[test]
fn usage_decodes() {
    require_key!();
    let usage = run_json(&["usage", "show"]);
    assert!(usage["total"].is_i64() || usage["total"].is_u64());
    let histogram = run_json(&["usage", "histogram", "--range", "24h"]);
    assert_eq!(histogram["range"], "24h");
}

#[test]
fn generations_and_assets_decode() {
    require_key!();
    let Some(project) = first_project() else {
        eprintln!("skipping: no projects");
        return;
    };

    let generations = run_json(&[
        "sprite",
        "list",
        "--project",
        &project,
        "--include-archived",
    ]);
    if let Some(generation) = generations["generations"]
        .as_array()
        .and_then(|g| g.first())
    {
        let id = generation["id"].as_str().unwrap();
        let fetched = run_json(&["sprite", "get", id, "--include-archived"]);
        assert_eq!(fetched["id"], generation["id"]);

        if let Some(asset) = fetched["assets"].as_array().and_then(|a| a.first()) {
            let asset_id = asset["id"].as_str().unwrap();
            let asset = run_json(&["sprite", "asset", "get", asset_id]);
            assert_eq!(asset["id"], asset["id"]);
        }
    }

    let assets = run_json(&["sprite", "assets", "--project", &project]);
    assert!(assets["assets"].is_array());
}

#[test]
fn sounds_decode() {
    require_key!();
    let Some(project) = first_project() else {
        eprintln!("skipping: no projects");
        return;
    };
    let sounds = run_json(&["sound", "list", "--project", &project, "--include-archived"]);
    if let Some(generation) = sounds["sound_generations"]
        .as_array()
        .and_then(|g| g.first())
    {
        let id = generation["id"].as_str().unwrap();
        let fetched = run_json(&["sound", "get", id, "--include-archived"]);
        assert_eq!(fetched["id"], generation["id"]);
    }
}

#[test]
fn animations_and_exports_decode() {
    require_key!();
    let Some(project) = first_project() else {
        eprintln!("skipping: no projects");
        return;
    };

    let runs = run_json(&[
        "animation",
        "list",
        "--project",
        &project,
        "--include-archived",
    ]);

    let mut base: Option<String> = None;
    if let Some(run) = runs["animations"].as_array().and_then(|r| r.first()) {
        let id = run["id"].as_str().unwrap();
        let fetched = run_json(&["animation", "get", id]);
        assert_eq!(fetched["id"], run["id"]);
        // Provenance is present on every run.
        assert!(fetched["user_id"].is_string() || fetched["source"].is_string());
        base = fetched["base_asset_id"].as_str().map(str::to_string);

        if fetched["frames"]
            .as_array()
            .map(|f| !f.is_empty())
            .unwrap_or(false)
        {
            let plan = run_json(&[
                "animation",
                "export-plan",
                id,
                "--start-frame",
                "1",
                "--end-frame",
                "2",
            ]);
            assert!(plan["frame_count"].is_number());
        }
    }

    // Filtering by base_asset_id.
    let filter = base.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let filtered = run_json(&[
        "animation",
        "list",
        "--project",
        &project,
        "--base-asset-id",
        &filter,
        "--include-archived",
    ]);
    assert!(filtered["animations"]
        .as_array()
        .unwrap()
        .iter()
        .all(|run| run["base_asset_id"].as_str() == Some(filter.as_str())));
}

#[test]
fn saved_animations_decode() {
    require_key!();
    let Some(project) = first_project() else {
        eprintln!("skipping: no projects");
        return;
    };
    let saved = run_json(&["saved", "list", "--project", &project, "--include-archived"]);
    assert!(saved["saved_animations"].is_array());
}

#[test]
fn labels_and_art_styles_decode() {
    require_key!();
    let Some(project) = first_project() else {
        eprintln!("skipping: no projects");
        return;
    };
    let labels = run_json(&["label", "list", "--project", &project]);
    if let Some(label) = labels["labels"].as_array().and_then(|l| l.first()) {
        let id = label["id"].as_str().unwrap();
        let items = run_json(&["label", "items", id]);
        assert_eq!(items["label"]["id"], label["id"]);
    }
    let styles = run_json(&["art-style", "list", "--project", &project]);
    assert!(styles["art_styles"].is_array());
}

#[test]
fn keys_decode_or_forbidden() {
    require_key!();
    let output = common::run(&["key", "list", "--json"]);
    if output.status.success() {
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(value["keys"].is_array());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("admin"),
            "expected an admin-only failure, got: {stderr}"
        );
    }
}
