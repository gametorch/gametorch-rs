//! Lists the projects in the caller's scope.
//!
//! Run with:
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_... cargo run --example list_projects
//! # against a local deployment:
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_BASE_URL=http://localhost:8300/api \
//!   cargo run --example list_projects
//! ```

use gametorch::Client;

#[tokio::main]
async fn main() -> gametorch::Result<()> {
    let client = Client::from_env()?;

    for project in client.list_projects().await?.projects {
        println!("{}  {}  ({})", project.id, project.name, project.slug);
    }

    Ok(())
}
