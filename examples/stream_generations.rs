//! Streams every generation in a project, page by page.
//!
//! Run with:
//!
//! ```sh
//! GAMETORCH_API_KEY=gt2_... GAMETORCH_PROJECT=<project-uuid> \
//!   cargo run --example stream_generations
//! ```

use gametorch::Client;

#[tokio::main]
async fn main() -> gametorch::Result<()> {
    let client = Client::from_env()?;
    let project: uuid::Uuid = std::env::var("GAMETORCH_PROJECT")
        .expect("set GAMETORCH_PROJECT to a project UUID")
        .parse()
        .expect("GAMETORCH_PROJECT must be a UUID");

    let mut generations = client.stream_generations(project, false);
    let mut count = 0usize;

    while let Some(generation) = generations.next_item().await {
        let generation = generation?;
        count += 1;
        println!(
            "{}  {}  {} asset(s)",
            generation.id,
            generation.status,
            generation.assets.len()
        );
    }

    println!(
        "streamed {count} generation(s), total={:?}",
        generations.total()
    );
    Ok(())
}
