//! Creates a project-scoped **read-only** key and a project-scoped **write**
//! key using an admin API key, setting each key's name, spend limit, reset
//! cadence and expiry date.
//!
//! This must be run with an **admin** API key (`GAMETORCH_API_KEY`): only admin
//! keys can manage API keys or create projects. It does not spend credits.
//!
//! Unlike generation, key management needs no special opt-in; the example
//! cleans up after itself by revoking the keys and deleting the temporary
//! project. Set `GAMETORCH_KEEP_KEYS=1` to leave them behind.
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_admin_... cargo run --example create_project_keys
//! GAMETORCH_API_KEY=gt2_admin_... GAMETORCH_KEEP_KEYS=1 \
//!   cargo run --example create_project_keys
//! ```

use chrono::{Duration, Utc};
use gametorch::{Client, CreateApiKeyRequest, SpendResetCadence, UpdateApiKeyRequest};
use rust_decimal::Decimal;
use uuid::Uuid;

#[tokio::main]
async fn main() -> gametorch::Result<()> {
    let client = Client::from_env()?;

    // A project the keys will be bound to. In practice you would reuse an
    // existing project id; this example creates a throwaway one.
    let project = client
        .create_project(format!("SDK Keys Example {}", short()))
        .await?;
    println!("created project '{}' ({})", project.name, project.slug);

    let expires_at = Utc::now() + Duration::days(30);

    // Read-only key bound to the project.
    let read = client
        .create_key(
            &CreateApiKeyRequest::project_read(project.id)
                .name("Read-only CI key")
                .max_spend_limit(Decimal::from(0))
                .spend_reset_cadence(SpendResetCadence::Monthly)
                .expires_at(expires_at),
        )
        .await?;
    print_key("read-only", &read);

    // Write key bound to the project.
    let write = client
        .create_key(
            &CreateApiKeyRequest::project_write(project.id)
                .name("Write CI key")
                .max_spend_limit(Decimal::from(500))
                .spend_reset_cadence(SpendResetCadence::Weekly)
                .expires_at(expires_at),
        )
        .await?;
    print_key("write", &write);

    // The scope is fixed, but the name, limit, cadence and expiry can change.
    let updated = client
        .update_key(
            write.key.id,
            &UpdateApiKeyRequest::new()
                .name("Write CI key (renamed)")
                .max_spend_limit(Decimal::from(750))
                .spend_reset_cadence(SpendResetCadence::Monthly)
                .expires_at(expires_at + Duration::days(30)),
        )
        .await?;
    println!(
        "updated write key: name={:?} limit={:?} cadence={} expires_at={:?}",
        updated.name, updated.max_spend_limit, updated.spend_reset_cadence, updated.expires_at
    );

    // List the keys visible to this admin key.
    println!("\nall keys:");
    for key in client.list_keys().await?.keys {
        println!(
            "  {:<24} scope={:<13} project={:?} limit={:?} cadence={} prefix={}",
            key.name.as_deref().unwrap_or("<unnamed>"),
            key.key_scope,
            key.project_id,
            key.max_spend_limit,
            key.spend_reset_cadence,
            key.key_prefix,
        );
    }

    if std::env::var("GAMETORCH_KEEP_KEYS").ok().as_deref() == Some("1") {
        println!("\nkeeping keys and project (GAMETORCH_KEEP_KEYS=1)");
        println!("  read key:  {}", read.key_full);
        println!("  write key: {}", write.key_full);
    } else {
        client.delete_key(read.key.id).await?;
        client.delete_key(write.key.id).await?;
        client.delete_project(&project.slug).await?;
        println!("\nrevoked keys and deleted project");
    }

    Ok(())
}

fn print_key(kind: &str, created: &gametorch::ApiKeyWithSecret) {
    println!(
        "created {kind} key: scope={} project={:?} limit={:?} cadence={} expires_at={:?}",
        created.key.key_scope,
        created.key.project_id,
        created.key.max_spend_limit,
        created.key.spend_reset_cadence,
        created.key.expires_at,
    );
    // Shown only once, at creation time. Treat it like a password.
    println!(
        "  key_full (store securely, shown once): {}",
        created.key_full
    );
}

fn short() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}
