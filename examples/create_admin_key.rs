//! Creates an **admin** API key using an existing admin key, setting its name,
//! spend limit, reset cadence and expiry date.
//!
//! This must be run with an **admin** API key (`GAMETORCH_API_KEY`). It does
//! not spend credits. It revokes the key it creates at the end unless
//! `GAMETORCH_KEEP_KEYS=1` is set.
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_admin_... cargo run --example create_admin_key
//! GAMETORCH_API_KEY=gt2_admin_... GAMETORCH_KEEP_KEYS=1 \
//!   cargo run --example create_admin_key
//! ```

use chrono::{Duration, Utc};
use gametorch::{Client, CreateApiKeyRequest, SpendResetCadence, UpdateApiKeyRequest};
use rust_decimal::Decimal;

#[tokio::main]
async fn main() -> gametorch::Result<()> {
    let client = Client::from_env()?;

    let created = client
        .create_key(
            &CreateApiKeyRequest::admin()
                .name("Admin CI key")
                .max_spend_limit(Decimal::from(5000))
                .spend_reset_cadence(SpendResetCadence::Monthly)
                .expires_at(Utc::now() + Duration::days(90)),
        )
        .await?;

    println!(
        "created admin key: name={:?} scope={} limit={:?} cadence={} expires_at={:?}",
        created.key.name,
        created.key.key_scope,
        created.key.max_spend_limit,
        created.key.spend_reset_cadence,
        created.key.expires_at,
    );
    // Shown only once, at creation time. Treat it like a password.
    println!(
        "key_full (store securely, shown once): {}",
        created.key_full
    );

    // Name, spend limit, cadence and expiry can all be changed afterwards.
    let updated = client
        .update_key(
            created.key.id,
            &UpdateApiKeyRequest::new()
                .name("Admin CI key (renamed)")
                .max_spend_limit(Decimal::from(7500))
                .expires_at(Utc::now() + Duration::days(180)),
        )
        .await?;
    println!(
        "updated: name={:?} limit={:?} expires_at={:?}",
        updated.name, updated.max_spend_limit, updated.expires_at
    );

    if std::env::var("GAMETORCH_KEEP_KEYS").ok().as_deref() == Some("1") {
        println!("keeping key (GAMETORCH_KEEP_KEYS=1): {}", created.key_full);
    } else {
        client.delete_key(created.key.id).await?;
        println!("revoked admin key");
    }

    Ok(())
}
