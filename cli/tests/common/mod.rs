//! Shared helpers for the CLI integration tests.

#![allow(dead_code)]

use std::process::{Command, Output};

use serde_json::Value;

/// Path to the compiled `gametorch` binary.
pub fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_gametorch")
}

/// A command preconfigured with the base URL and API key from the environment.
pub fn cmd() -> Command {
    let mut command = Command::new(bin());
    if let Ok(url) = std::env::var("GAMETORCH_BASE_URL") {
        command.args(["--base-url", &url]);
    }
    if let Ok(key) = std::env::var("GAMETORCH_API_KEY") {
        command.args(["--api-key", &key]);
    }
    command
}

pub fn has_api_key() -> bool {
    std::env::var("GAMETORCH_API_KEY")
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

pub fn env_enabled(name: &str) -> bool {
    std::env::var(name).ok().as_deref() == Some("1")
}

pub fn keep_project() -> bool {
    env_enabled("GAMETORCH_KEEP_PROJECT")
}

/// Runs `gametorch` with the given args and returns the raw output.
pub fn run(args: &[&str]) -> Output {
    cmd().args(args).output().expect("spawn gametorch")
}

/// Runs `gametorch`, asserts success, and returns stdout.
pub fn run_ok(args: &[&str]) -> String {
    let output = run(args);
    assert!(
        output.status.success(),
        "command failed: {args:?}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is utf8")
}

/// Runs `gametorch ... --json`, asserts success, and parses stdout as JSON.
pub fn run_json(args: &[&str]) -> Value {
    let mut args: Vec<&str> = args.to_vec();
    args.push("--json");
    let stdout = run_ok(&args);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("invalid JSON from {args:?}: {err}\nstdout: {stdout}"))
}

/// Runs a command expected to fail and returns its stderr.
pub fn run_err(args: &[&str]) -> String {
    let output = run(args);
    assert!(
        !output.status.success(),
        "expected failure but command succeeded: {args:?}\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Creates a project via the CLI and returns `(id, slug)`.
pub fn create_project(kind: &str) -> (String, String) {
    let value = run_json(&[
        "project",
        "create",
        "--name",
        &format!("SDK CLI {kind} {}", short()),
    ]);
    let id = value["id"].as_str().expect("project id").to_string();
    let slug = value["slug"].as_str().expect("project slug").to_string();
    (id, slug)
}

/// Deletes the project unless `GAMETORCH_KEEP_PROJECT=1`.
pub fn finish_project(slug: &str) {
    if keep_project() {
        eprintln!("keeping project '{slug}' (GAMETORCH_KEEP_PROJECT=1)");
    } else {
        run_ok(&["project", "delete", slug]);
    }
}

pub fn short() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..8].to_string()
}

/// Polls `sprite get` until the generation reaches a terminal state.
pub fn wait_sprite(id: &str) -> Value {
    for _ in 0..180 {
        let value = run_json(&["sprite", "get", id, "--include-archived"]);
        let status = value["status"].as_str().unwrap_or("");
        if status != "queued" && status != "running" {
            return value;
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    panic!("timed out waiting for sprite generation {id}");
}

pub fn wait_sound(id: &str) -> Value {
    for _ in 0..180 {
        let value = run_json(&["sound", "get", id, "--include-archived"]);
        let status = value["status"].as_str().unwrap_or("");
        if status != "queued" && status != "running" {
            return value;
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    panic!("timed out waiting for sound generation {id}");
}

pub fn wait_animation(id: &str) -> Value {
    for _ in 0..240 {
        let value = run_json(&["animation", "get", id]);
        let status = value["status"].as_str().unwrap_or("");
        if status != "queued" && status != "running" {
            return value;
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    panic!("timed out waiting for animation run {id}");
}

/// Waits until frame generation has settled, then returns the animation run.
pub fn wait_frames(id: &str) -> Value {
    let mut last_len = 0usize;
    for _ in 0..240 {
        let value = run_json(&["animation", "get", id]);
        let settled = value["frame_runs"]
            .as_array()
            .map(|runs| {
                runs.iter().all(|run| {
                    let status = run["status"].as_str().unwrap_or("");
                    status != "queued" && status != "running"
                })
            })
            .unwrap_or(true);
        let len = value["frames"].as_array().map(Vec::len).unwrap_or(0);
        if len > 0 && settled && len == last_len {
            return value;
        }
        last_len = len;
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    run_json(&["animation", "get", id])
}
