//! Opt-in live write tests that do **not** spend credits.
//!
//! Enabled only when both `GAMETORCH_API_KEY` and `GAMETORCH_LIVE_WRITES=1` are
//! set. Each test creates its own project and deletes it again at the end,
//! unless `GAMETORCH_KEEP_PROJECT=1` is set (then the project is left behind so
//! you can inspect it in the UI).
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_LIVE_WRITES=1 \
//! GAMETORCH_BASE_URL=http://localhost:8300/api \
//!   cargo test --test live_writes -- --test-threads=1 --nocapture
//!
//! # Keep the created projects for inspection:
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_LIVE_WRITES=1 GAMETORCH_KEEP_PROJECT=1 \
//!   cargo test --test live_writes -- --test-threads=1 --nocapture
//! ```

use chrono::{Duration, Utc};
use gametorch::{
    ApiKeyScope, Client, CreateApiKeyRequest, Project, SpendResetCadence, UpdateApiKeyRequest,
};
use rust_decimal::Decimal;
use uuid::Uuid;

fn client() -> Option<Client> {
    if std::env::var("GAMETORCH_LIVE_WRITES").ok().as_deref() != Some("1") {
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
                eprintln!(
                    "skipping live write test: set GAMETORCH_LIVE_WRITES=1 and GAMETORCH_API_KEY"
                );
                return;
            }
        }
    };
}

/// A short, unique, label-safe name (letters, numbers, dashes and underscores
/// only).
fn short_name(prefix: &str) -> String {
    format!("{prefix}-{}", &Uuid::new_v4().simple().to_string()[..8])
}

fn project_name(kind: &str) -> String {
    format!("SDK {kind} {}", &Uuid::new_v4().simple().to_string()[..8])
}

fn keep_project() -> bool {
    std::env::var("GAMETORCH_KEEP_PROJECT").ok().as_deref() == Some("1")
}

/// Creates a fresh project, or returns `None` if this key may not create them.
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

/// Deletes the project unless `GAMETORCH_KEEP_PROJECT=1`.
async fn finish_project(client: &Client, slug: &str) {
    if keep_project() {
        println!("keeping project '{slug}' (GAMETORCH_KEEP_PROJECT=1)");
    } else {
        client.delete_project(slug).await.expect("delete project");
        println!("deleted project '{slug}'");
    }
}

#[tokio::test]
async fn project_lifecycle() {
    let client = require_client!();
    let Some(project) = create_project(&client, "Project Test").await else {
        return;
    };

    let projects = client.list_projects().await.expect("list projects");
    assert!(projects.projects.iter().any(|p| p.id == project.id));

    let renamed = client
        .rename_project(&project.slug, project_name("Project Test Renamed"))
        .await
        .expect("rename project");
    assert_eq!(renamed.id, project.id);

    finish_project(&client, &renamed.slug).await;
}

#[tokio::test]
async fn label_lifecycle() {
    let client = require_client!();
    let Some(project) = create_project(&client, "Label Test").await else {
        return;
    };

    let name = short_name("sdk-label");
    let label = client
        .create_label(project.id, &name, Some("#123456"))
        .await
        .expect("create label");
    assert_eq!(label.name, name);

    let new_name = short_name("sdk-renamed");
    let updated = client
        .update_label(label.id, Some(&new_name), Some("#654321"))
        .await
        .expect("update label");
    assert_eq!(updated.name, new_name);

    let labels = client.list_labels(project.id).await.expect("list labels");
    assert!(labels.labels.iter().any(|l| l.id == label.id));

    client.delete_label(label.id).await.expect("delete label");

    // Leave a label behind so a kept project shows one in the UI.
    let kept = client
        .create_label(project.id, short_name("sdk-kept"), Some("#8bc34a"))
        .await
        .expect("create kept label");
    println!("kept label '{}'", kept.name);

    finish_project(&client, &project.slug).await;
}

#[tokio::test]
async fn art_style_lifecycle() {
    let client = require_client!();
    let Some(project) = create_project(&client, "Art Style Test").await else {
        return;
    };

    let name = short_name("sdk-style");
    let style = client
        .create_art_style(project.id, &name)
        .await
        .expect("create art style");
    assert_eq!(style.name, name);

    let styles = client
        .list_art_styles(project.id)
        .await
        .expect("list styles");
    assert!(styles.art_styles.iter().any(|s| s.id == style.id));

    client
        .delete_art_style(style.id)
        .await
        .expect("delete art style");

    // Leave an art style behind so a kept project shows one in the UI.
    let kept = client
        .create_art_style(project.id, short_name("sdk-kept-style"))
        .await
        .expect("create kept art style");
    println!("kept art style '{}'", kept.name);

    finish_project(&client, &project.slug).await;
}

#[tokio::test]
async fn generate_art_style_decodes() {
    let client = require_client!();
    let Some(project) = create_project(&client, "Art Style Suggest Test").await else {
        return;
    };

    let suggestion = client
        .generate_art_style(project.id)
        .await
        .expect("generate art style");
    assert!(!suggestion.name.is_empty());

    finish_project(&client, &project.slug).await;
}

#[tokio::test]
async fn ensure_user_ok() {
    let client = require_client!();
    let response = client.ensure_user().await.expect("ensure user");
    assert!(response.ok);
}

#[tokio::test]
async fn key_lifecycle() {
    let client = require_client!();

    let created = match client
        .create_key(&CreateApiKeyRequest::new().name(short_name("sdk-key")))
        .await
    {
        Ok(created) => created,
        Err(err) if err.is_forbidden() => {
            eprintln!("skipping: this key cannot manage API keys (admin required)");
            return;
        }
        Err(err) => panic!("create key: {err}"),
    };
    assert!(!created.key_full.is_empty());
    assert!(!created.key.key_prefix.is_empty());

    let updated = client
        .update_key(
            created.key.id,
            &UpdateApiKeyRequest::new().name(short_name("sdk-key-renamed")),
        )
        .await
        .expect("update key");
    assert!(updated.name.is_some());

    client.delete_key(created.key.id).await.expect("delete key");
}

#[tokio::test]
async fn scoped_key_lifecycle() {
    let client = require_client!();
    let Some(project) = create_project(&client, "Scoped Key Test").await else {
        return;
    };

    let expires_at = Utc::now() + Duration::days(30);
    let read = match client
        .create_key(
            &CreateApiKeyRequest::project_read(project.id)
                .name(short_name("sdk-read"))
                .max_spend_limit(Decimal::from(0))
                .spend_reset_cadence(SpendResetCadence::Monthly)
                .expires_at(expires_at),
        )
        .await
    {
        Ok(key) => key,
        Err(err) if err.is_forbidden() => {
            eprintln!("skipping: this key cannot manage API keys (admin required)");
            finish_project(&client, &project.slug).await;
            return;
        }
        Err(err) => panic!("create read key: {err}"),
    };
    assert_eq!(read.key.key_scope, ApiKeyScope::ProjectRead);
    assert_eq!(read.key.project_id, Some(project.id));
    assert!(read.key.name.is_some());
    assert!(read.key.expires_at.is_some());

    let write = client
        .create_key(
            &CreateApiKeyRequest::project_write(project.id)
                .name(short_name("sdk-write"))
                .max_spend_limit(Decimal::from(500))
                .spend_reset_cadence(SpendResetCadence::Weekly)
                .expires_at(expires_at),
        )
        .await
        .expect("create write key");
    assert_eq!(write.key.key_scope, ApiKeyScope::ProjectWrite);
    assert_eq!(write.key.project_id, Some(project.id));
    assert_eq!(write.key.max_spend_limit, Some(Decimal::from(500)));

    // Scope is fixed, but name/limit/cadence/expiry are updatable.
    let updated = client
        .update_key(
            write.key.id,
            &UpdateApiKeyRequest::new()
                .name(short_name("sdk-write-renamed"))
                .max_spend_limit(Decimal::from(750))
                .spend_reset_cadence(SpendResetCadence::Monthly)
                .expires_at(expires_at + Duration::days(30)),
        )
        .await
        .expect("update write key");
    assert_eq!(updated.max_spend_limit, Some(Decimal::from(750)));
    assert_eq!(updated.spend_reset_cadence, "monthly");

    client
        .delete_key(read.key.id)
        .await
        .expect("delete read key");
    client
        .delete_key(write.key.id)
        .await
        .expect("delete write key");

    finish_project(&client, &project.slug).await;
}
