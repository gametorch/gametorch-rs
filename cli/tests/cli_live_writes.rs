//! Opt-in live CLI write tests (no credits spent).
//!
//! Enabled with `GAMETORCH_LIVE_WRITES=1` plus `GAMETORCH_API_KEY`. Each test
//! creates its own project and deletes it at the end unless
//! `GAMETORCH_KEEP_PROJECT=1`.

mod common;

use common::{create_project, env_enabled, finish_project, has_api_key, run, run_json, short};

macro_rules! require {
    () => {
        if !has_api_key() || !env_enabled("GAMETORCH_LIVE_WRITES") {
            eprintln!("skipping: set GAMETORCH_LIVE_WRITES=1 and GAMETORCH_API_KEY");
            return;
        }
    };
}

#[test]
fn project_lifecycle() {
    require!();
    let (_id, slug) = create_project("Project Test");
    let renamed = run_json(&[
        "project",
        "rename",
        &slug,
        "--name",
        &format!("SDK CLI Renamed {}", short()),
    ]);
    assert!(renamed["id"].is_string());
    let projects = run_json(&["project", "list"]);
    assert!(projects["projects"].is_array());
    finish_project(renamed["slug"].as_str().unwrap());
}

#[test]
fn label_lifecycle() {
    require!();
    let (project, slug) = create_project("Label Test");
    let name = format!("cli-label-{}", short());
    let label = run_json(&[
        "label",
        "create",
        "--project",
        &project,
        "--name",
        &name,
        "--color",
        "#123456",
    ]);
    assert_eq!(label["name"], name);

    let renamed = format!("cli-renamed-{}", short());
    let updated = run_json(&[
        "label",
        "update",
        label["id"].as_str().unwrap(),
        "--name",
        &renamed,
        "--color",
        "#654321",
    ]);
    assert_eq!(updated["name"], renamed);

    let labels = run_json(&["label", "list", "--project", &project]);
    assert!(labels["labels"].is_array());
    common::run_ok(&["label", "delete", label["id"].as_str().unwrap()]);

    // Leave a label behind for browsing.
    let kept = run_json(&[
        "label",
        "create",
        "--project",
        &project,
        "--name",
        &format!("cli-kept-{}", short()),
    ]);
    assert!(kept["id"].is_string());

    finish_project(&slug);
}

#[test]
fn art_style_lifecycle() {
    require!();
    let (project, slug) = create_project("Art Style Test");
    let name = format!("cli-style-{}", short());
    let style = run_json(&[
        "art-style",
        "create",
        "--project",
        &project,
        "--name",
        &name,
    ]);
    assert_eq!(style["name"], name);
    let styles = run_json(&["art-style", "list", "--project", &project]);
    assert!(styles["art_styles"].is_array());
    common::run_ok(&["art-style", "delete", style["id"].as_str().unwrap()]);

    let kept = run_json(&[
        "art-style",
        "create",
        "--project",
        &project,
        "--name",
        &format!("cli-kept-style-{}", short()),
    ]);
    assert!(kept["id"].is_string());

    finish_project(&slug);
}

#[test]
fn generate_art_style_decodes() {
    require!();
    let (project, slug) = create_project("Art Style Suggest Test");
    let suggestion = run_json(&["art-style", "generate", "--project", &project]);
    assert!(suggestion["name"]
        .as_str()
        .map(|s| !s.is_empty())
        .unwrap_or(false));
    finish_project(&slug);
}

#[test]
fn ensure_user_ok() {
    require!();
    let response = run_json(&["user", "ensure"]);
    assert_eq!(response["ok"], true);
}

fn create_key(args: &[&str]) -> Option<serde_json::Value> {
    let output = run(args);
    if output.status.success() {
        Some(serde_json::from_slice(&output.stdout).expect("key json"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("admin"),
            "expected an admin-only failure, got: {stderr}"
        );
        None
    }
}

#[test]
fn key_lifecycle() {
    require!();
    let name = format!("cli-key-{}", short());
    let Some(created) = create_key(&["key", "create", "--name", &name, "--json"]) else {
        eprintln!("skipping: this key cannot manage API keys (admin required)");
        return;
    };
    assert!(created["key_full"].as_str().unwrap().starts_with("gt2_"));
    assert_eq!(created["key_scope"], "admin");

    let updated = run_json(&[
        "key",
        "update",
        created["id"].as_str().unwrap(),
        "--name",
        &format!("cli-key-renamed-{}", short()),
    ]);
    assert!(updated["name"].is_string());

    common::run_ok(&["key", "delete", created["id"].as_str().unwrap()]);
}

#[test]
fn scoped_key_lifecycle() {
    require!();
    let (project, slug) = create_project("Scoped Key Test");

    let Some(read) = create_key(&[
        "key",
        "create",
        "--scope",
        "project_read",
        "--project",
        &project,
        "--name",
        &format!("cli-read-{}", short()),
        "--max-spend-limit",
        "0",
        "--spend-reset-cadence",
        "monthly",
        "--expires-in",
        "30d",
        "--json",
    ]) else {
        eprintln!("skipping: this key cannot manage API keys (admin required)");
        finish_project(&slug);
        return;
    };
    assert_eq!(read["key_scope"], "project_read");
    assert_eq!(read["project_id"], project);
    assert!(read["expires_at"].is_string());

    let write = run_json(&[
        "key",
        "create",
        "--scope",
        "project_write",
        "--project",
        &project,
        "--name",
        &format!("cli-write-{}", short()),
        "--max-spend-limit",
        "500",
        "--spend-reset-cadence",
        "weekly",
        "--expires-in",
        "30d",
    ]);
    assert_eq!(write["key_scope"], "project_write");
    assert_eq!(write["project_id"], project);

    let updated = run_json(&[
        "key",
        "update",
        write["id"].as_str().unwrap(),
        "--max-spend-limit",
        "750",
        "--spend-reset-cadence",
        "monthly",
        "--expires-in",
        "60d",
    ]);
    assert_eq!(updated["spend_reset_cadence"], "monthly");

    common::run_ok(&["key", "delete", read["id"].as_str().unwrap()]);
    common::run_ok(&["key", "delete", write["id"].as_str().unwrap()]);

    finish_project(&slug);
}
