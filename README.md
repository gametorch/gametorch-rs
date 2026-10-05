# GameTorch Rust SDK

The official async Rust SDK for the [GameTorch](https://gametorch.app) API.
GameTorch generates game-ready **sprites**, **sound effects** and **animations**
from text prompts and organizes them into projects.

- [API reference](https://gametorch.app/api/docs)
- [Agent / LLM guide](https://gametorch.app/llms.txt)
- [Privacy policy](https://gametorch.app/privacy)
- [Terms and conditions](https://gametorch.app/terms)

## Features

- Full coverage of the public GameTorch API, with strongly typed request and
  response models.
- Async-first, built on [`reqwest`](https://crates.io/crates/reqwest) and
  [`tokio`](https://crates.io/crates/tokio).
- Polite by default: client-side rate limiting that mirrors GameTorch's
  published limits, concurrency caps, and automatic retries with exponential
  backoff that honors `Retry-After`.
- Cursor pagination with async `Paginator` helpers.
- Accurate money handling with [`rust_decimal`](https://crates.io/crates/rust_decimal)
  (100 credits = $1).
- Sensible errors with status-code helpers.
- MIT licensed.

## Installation

```toml
[dependencies]
gametorch = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quickstart

```rust,no_run
use gametorch::{Client, SpriteMode};

#[tokio::main]
async fn main() -> gametorch::Result<()> {
    // Reads GAMETORCH_API_KEY; or call .api_key("gt2_...") explicitly.
    let client = Client::from_env()?;

    let models = client.sprite_models().await?;
    let project = client.create_project("My Game").await?;

    let job = client
        .generate_sprite(project.id)
        .prompt("a red fox, side view")
        .mode(SpriteMode::Single)
        .image_model(&models.image_models[0].id)
        .send()
        .await?;

    println!("generation {} is {}", job.id, job.status);
    Ok(())
}
```

## Authentication

Every request uses `Authorization: Bearer <token>`, where the token is either a
server-to-server API key (`gt2_...`) or a Clerk session token:

```rust,no_run
use gametorch::Client;

# fn main() -> gametorch::Result<()> {
// API key (server-to-server)
let client = Client::builder().api_key("gt2_...").build()?;

// Clerk session token (browser / trusted backend)
let client = Client::builder().bearer_token("eyJ...").build()?;
# Ok(())
# }
```

Create an API key from the GameTorch dashboard or with
[`Client::create_key`](https://gametorch.app/api/docs). Keys are shown only once
and can carry a spend limit.

### Key scopes

Every API key has a `key_scope` fixed at creation:

- `admin` — full access to the owning account/organization, exactly like an
  admin session. Carries no project.
- `project_write` — bound to one project: read and write its sprites, sounds,
  animations, labels, names and metadata. No account/admin access, no other
  projects.
- `project_read` — bound to one project: read-only.

Project-scoped keys get `403` on account/admin routes and `404` when they name
another project. Build scoped keys with the request constructors:

```rust,no_run
# async fn run(client: gametorch::Client, project: uuid::Uuid) -> gametorch::Result<()> {
use chrono::{Duration, Utc};
use gametorch::{CreateApiKeyRequest, SpendResetCadence};
use rust_decimal::Decimal;

let read = client
    .create_key(
        &CreateApiKeyRequest::project_read(project)
            .name("CI read key")
            .max_spend_limit(Decimal::from(0))
            .spend_reset_cadence(SpendResetCadence::Monthly)
            .expires_at(Utc::now() + Duration::days(30)),
    )
    .await?;
println!("scope: {}, project: {:?}", read.key.key_scope, read.key.project_id);
# Ok(())
# }
```

See [`ApiKeyScope`], [`CreateApiKeyRequest`] and the
`create_admin_key` / `create_project_keys` examples.

## Base URL

The SDK defaults to `https://gametorch.app/api`. For local development, point
it at your local deployment:

```rust,no_run
# fn main() -> gametorch::Result<()> {
let client = gametorch::Client::builder()
    .api_key("gt2_local_dev_key")
    .base_url("http://localhost:8300/api")
    .build()?;
# Ok(())
# }
```

You can also set `GAMETORCH_BASE_URL`.

## Rate limits

GameTorch rate limits every account per route and returns `429` when a limit is
exceeded. The SDK is respectful by default:

- Throttles **Tier 1** routes (generation creates, frame generation, animation
  exports) to 1 request/second per route.
- Throttles **Tier 2** routes (content, usage, search, single-item reads) to
  2 requests/second per route.
- Shares a **token bucket** (100-request burst, 5 requests/second refill) across
  unbounded writes.
- Caps in-flight hold-creating requests at 25 and concurrent frame generations
  at 5.
- Retries `429`, `408` and `5xx` responses with exponential backoff and jitter,
  honoring `Retry-After`.

Tune or disable this behavior on the builder:

```rust,no_run
# fn main() -> gametorch::Result<()> {
let client = gametorch::Client::builder()
    .api_key("gt2_...")
    .max_retries(5)
    .retry_base_delay(std::time::Duration::from_millis(250))
    .rate_limit(false) // only if you manage limits yourself
    .build()?;
# Ok(())
# }
```

## Pagination

List endpoints accept `ListParams` and return a page with a `next_cursor`. The
`stream_*` helpers fetch pages on demand:

```rust,no_run
# async fn run(client: gametorch::Client, project: uuid::Uuid) -> gametorch::Result<()> {
let mut generations = client.stream_generations(project, false);
while let Some(generation) = generations.next_item().await {
    let generation = generation?;
    println!("{}", generation.id);
}
# Ok(())
# }
```

## Filtering animations by base image

Animation runs link back to the sprite asset they were generated from via
`base_asset_id` (`null` when generated from scratch). You can filter animation
queries by it:

```rust,no_run
# async fn run(client: gametorch::Client, project: uuid::Uuid, sprite: uuid::Uuid) -> gametorch::Result<()> {
use gametorch::ListParams;

let runs = client
    .list_animation_runs(project, &ListParams::new().base_asset_id(sprite))
    .await?;
for run in runs.animations {
    println!("{} from {:?}", run.id, run.base_asset_id);
}

// The stream helper honors the same filters.
let mut stream = client.stream_animation_runs(project, ListParams::new().base_asset_id(sprite));
while let Some(run) = stream.next_item().await {
    let run = run?;
    println!("{}", run.id);
}
# Ok(())
# }
```

## Provenance

Generation and asset responses expose who or what created them. The fields are
flattened onto the resource as `provenance` (`user_id`, `source`, `api_key_id`,
`key_name`):

```rust,no_run
# async fn run(client: gametorch::Client, generation: uuid::Uuid) -> gametorch::Result<()> {
let generation = client.get_generation(generation, false).await?;
println!(
    "created by {:?} via {:?}",
    generation.provenance.user_id, generation.provenance.source
);
# Ok(())
# }
```

## Error handling

Every fallible operation returns `gametorch::Result<T>`. `Error` exposes the
HTTP status, the API's `error` message and convenience predicates:

```rust,no_run
# async fn run(client: gametorch::Client) -> gametorch::Result<()> {
match client.get_asset(uuid::Uuid::nil()).await {
    Ok(asset) => println!("{}", asset.id),
    Err(err) if err.is_not_found() => println!("no such asset"),
    Err(err) if err.is_rate_limited() => println!("slow down"),
    Err(err) => return Err(err),
}
# Ok(())
# }
```

## API coverage

| Area | Methods |
| --- | --- |
| Catalog | `sprite_models`, `sound_models`, `animation_models` |
| Projects | `list_projects`, `create_project`, `rename_project`, `delete_project` |
| Sprites | `generate_sprite`, `list_generations`, `get_generation`, `list_sprite_assets`, `get_asset`, `asset_content`, `asset_original`, `rename_asset`, `put_asset_metadata`, `archive_asset`, `unarchive_asset`, `delete_asset`, `archive_generation`, `unarchive_generation`, `delete_generation` |
| Sounds | `generate_sound`, `list_sound_generations`, `get_sound_generation`, `sound_asset_content`, `rename_sound_asset`, `put_sound_asset_metadata`, `archive_sound_asset`, `unarchive_sound_asset`, `delete_sound_asset`, `archive_sound_generation`, `unarchive_sound_generation` |
| Animations | `estimate_animation`, `generate_animation`, `list_animation_runs`, `get_animation_run`, `animation_content`, `archive_animation_run`, `unarchive_animation_run`, `delete_animation_run`, `generate_frames`, `frame_content`, `frame_content_by_number` |
| Exports | `export_plan`, `export`, `export_texturepacker`, `export_texturepacker_zip`, `export_aseprite`, `export_godot`, `export_godot_zip`, `export_grid`, `export_gamemaker`, `export_sequence_zip` |
| Saved animations | `list_saved_animations`, `save_animation`, `get_saved_animation`, `rename_saved_animation`, `put_saved_animation_metadata`, `archive_saved_animation`, `unarchive_saved_animation`, `delete_saved_animation` |
| Labels | `list_labels`, `create_label`, `update_label`, `delete_label`, `label_items`, `set_label_thumbnail`, `associate_asset_label`, `remove_asset_label`, `dismiss_asset_label_suggestion`, `associate_sound_label`, `remove_sound_label`, `dismiss_sound_label_suggestion`, `associate_saved_animation_label`, `remove_saved_animation_label` |
| Art styles | `list_art_styles`, `create_art_style`, `generate_art_style`, `delete_art_style` |
| Usage | `usage`, `usage_histogram` |
| API keys | `list_keys`, `create_key`, `update_key`, `delete_key` |
| Account | `ensure_user`, `health` |

## Examples

Runnable examples live in [`examples/`](examples):

```sh
export GAMETORCH_API_KEY=gt2_...
cargo run --example list_projects
cargo run --example generate_sprite      # spends credits
cargo run --example generate_sound       # spends credits
cargo run --example generate_animation   # spends credits
cargo run --example export_animation     # read-only; exports every format
cargo run --example create_admin_key     # needs an admin key
cargo run --example create_project_keys  # needs an admin key
GAMETORCH_PROJECT=<project-uuid> cargo run --example stream_generations
```

See each example's header for details. `generate_animation` walks the full
animation workflow: generation, frame generation, saving a sub-range as a named
preset, naming, metadata, labels and exporting in every supported format.
`create_project_keys` creates a project-scoped read-only key and write key with
names, spend limits, reset cadence and expiry.

## Command-line interface

The `gametorch` CLI wraps the SDK and exposes **every** public API operation:
catalogs, projects, sprites, sounds, animations, exports, saved animations,
labels, art styles, usage, API keys, account and health.

### CLI installation

The CLI lives in the `cli/` workspace member and builds a binary named
**`gametorch`**. Install it from a clone:

```sh
cargo install --path cli        # installs the `gametorch` binary
```

A plain `cargo build` at the repo root also produces `target/debug/gametorch`,
and `cargo run` runs it:

```sh
cargo build
./target/debug/gametorch --help

cargo run -- --help
```

### Getting started

Provide credentials via flags or environment variables:

```sh
export GAMETORCH_API_KEY=gt2_...          # API key (admin or project-scoped)
# or: export GAMETORCH_TOKEN=eyJ...       # Clerk session token
export GAMETORCH_BASE_URL=http://localhost:8300/api   # optional; defaults to production

gametorch health
gametorch catalog sprite
gametorch project list
```

Every command supports `--json` for machine-readable output, plus the global
options `--api-key`, `--token`, `--base-url`, `--no-rate-limit`,
`--max-retries <n>` and `--timeout <secs>`. Commands that take `--project`
accept a project **id or slug**. Downloads accept `--output <file>` (`-` means
stdout).

### CLI examples

**Health and account**

```sh
gametorch health
gametorch user ensure
```

**Catalogs**

```sh
gametorch catalog sprite
gametorch catalog sound
gametorch catalog animation
```

**Projects**

```sh
gametorch project list
gametorch project create --name "My Game"
gametorch project rename my-game --name "My Game (2026)"
gametorch project delete my-game
```

**Sprites**

```sh
# Generate (add --wait to poll to completion).
gametorch sprite generate --project my-game --prompt "a red fox, side view" \
  --mode single --image-model openai/gpt-image-2.5-flare --wait

gametorch sprite list --project my-game --include-archived
gametorch sprite list --project my-game --before <cursor>
gametorch sprite get <generation-id> --include-archived
gametorch sprite assets --project my-game --q fox

gametorch sprite asset get <asset-id>
gametorch sprite asset content <asset-id> --output fox.png
gametorch sprite asset original <asset-id> --output fox-original.png
gametorch sprite asset rename <asset-id> "Hero Fox"      # "" clears the name
gametorch sprite asset metadata <asset-id> --metadata filepath=/foo/bar/fox.png

gametorch sprite asset metadata-clear <asset-id>
gametorch sprite asset archive <asset-id>
gametorch sprite asset unarchive <asset-id>
gametorch sprite asset delete <asset-id>

gametorch sprite generation archive <generation-id>
gametorch sprite generation unarchive <generation-id>
gametorch sprite generation delete <generation-id>
```

**Sounds**

```sh
gametorch sound generate --project my-game --prompt "a sword unsheathing" \
  --sound-model bytedance-seed/seed-audio-1-0 --response-format mp3 --wait

gametorch sound list --project my-game --include-archived
gametorch sound get <sound-generation-id> --include-archived
gametorch sound asset content <sound-asset-id> --output sword.mp3
gametorch sound asset rename <sound-asset-id> "Sword Unsheath"
gametorch sound asset metadata <sound-asset-id> --metadata filepath=/foo/bar/sword.mp3
gametorch sound asset metadata-clear <sound-asset-id>
gametorch sound asset archive <sound-asset-id>
gametorch sound asset unarchive <sound-asset-id>
gametorch sound asset delete <sound-asset-id>
gametorch sound generation archive <sound-generation-id>
gametorch sound generation unarchive <sound-generation-id>
```

**Animations, frames and exports**

```sh
gametorch animation estimate --project my-game --animation-model ash --duration 4
gametorch animation generate --project my-game --prompt "draw the sword" \
  --animation-model ash --duration 4 --base-asset-id <asset-id> --wait

gametorch animation list --project my-game --include-archived
gametorch animation list --project my-game --base-asset-id <asset-id>   # filter by base image
gametorch animation get <run-id>
gametorch animation content <run-id> --output clip.bin
gametorch animation archive <run-id>
gametorch animation unarchive <run-id>
gametorch animation delete <run-id>

gametorch animation frames generate --project my-game --run <run-id> --fps 12
gametorch animation frames content <frame-id> --output frame.png
gametorch animation frames content-by-number --run <run-id> --number 3 --output frame.png

gametorch animation export-plan <run-id> --start-frame 1 --end-frame 10
gametorch animation export <run-id> --format texturepacker --start-frame 1 --end-frame 10
gametorch animation export <run-id> --format grid --start-frame 1 --end-frame 10 --output strip.png
# formats: texturepacker, texturepacker.zip, aseprite, godot, godot.zip, grid, gamemaker, sequence.zip
```

**Saved animations**

```sh
gametorch saved list --project my-game --include-archived
gametorch saved save --project my-game --generation-id <run-id> \
  --start-frame 2 --end-frame 20 --name "Sword Raise"
gametorch saved get <saved-id>
gametorch saved rename <saved-id> "Sword Raise (v2)"
gametorch saved metadata <saved-id> --metadata filepath=/foo/bar/raise.aseprite
gametorch saved metadata-clear <saved-id>
gametorch saved archive <saved-id>
gametorch saved unarchive <saved-id>
gametorch saved delete <saved-id>
```

**Labels**

```sh
gametorch label list --project my-game
gametorch label create --project my-game --name enemy --color "#e57b7b"
gametorch label update <label-id> --name foes --color "#c0392b"
gametorch label delete <label-id>
gametorch label items <label-id>
gametorch label thumbnail <label-id> --asset-id <asset-id>   # or --clear

# Associate/remove labels on any item; dismiss suggested labels.
gametorch label associate --asset <asset-id> --name enemy
gametorch label remove --asset <asset-id> --name enemy
gametorch label dismiss --asset <asset-id> --name enemy
gametorch label associate --sound <sound-asset-id> --name enemy
gametorch label associate --saved-animation <saved-id> --name enemy
```

**Art styles**

```sh
gametorch art-style list --project my-game
gametorch art-style create --project my-game --name "16-bit pixel art"
gametorch art-style generate --project my-game
gametorch art-style delete <art-style-id>
```

**Usage**

```sh
gametorch usage show
gametorch usage show --split user --before <cursor>
gametorch usage histogram --range 24h
gametorch usage histogram --range 7d --source "API key" --split user
```

**API keys** (require an admin key)

```sh
# Admin key with a name, spend limit, reset cadence and expiry.
gametorch key create --scope admin --name "Admin CI key" \
  --max-spend-limit 5000 --spend-reset-cadence monthly --expires-in 90d

# Project-scoped keys: full read/write or read-only.
gametorch key create --scope project_write --project my-game --name "Write CI" \
  --max-spend-limit 500 --spend-reset-cadence weekly --expires-in 30d
gametorch key create --scope project_read --project my-game --name "Read CI" \
  --expires-at 2026-12-31T00:00:00Z

gametorch key list
gametorch key update <key-id> --max-spend-limit 750 --spend-reset-cadence monthly --expires-in 60d
gametorch key delete <key-id>
```

`key create` prints `key_full` once — store it securely. Expiry accepts
`--expires-at <RFC3339>` or `--expires-in <30d|12h|90m>`.

## Testing

`cargo test` runs the offline unit and fixture tests by default. The live
suites are opt-in so they never hit the network or spend credits unless you ask:

| Suite | Env vars | Spends credits |
| --- | --- | --- |
| `tests/live.rs` | `GAMETORCH_API_KEY` | No (read-only + free estimate) |
| `tests/live_writes.rs` | `GAMETORCH_API_KEY`, `GAMETORCH_LIVE_WRITES=1` | No (creates and cleans up its own data) |
| `tests/live_spend.rs` | `GAMETORCH_API_KEY`, `GAMETORCH_LIVE_SPEND=1` | **Yes** |

```sh
GAMETORCH_API_KEY=gt2_... GAMETORCH_BASE_URL=http://localhost:8300/api \
  cargo test --test live -- --test-threads=1

GAMETORCH_API_KEY=gt2_... GAMETORCH_LIVE_WRITES=1 \
  cargo test --test live_writes -- --test-threads=1

# Costs money: generates a sprite, a sound and a 4s animation plus exports.
GAMETORCH_API_KEY=gt2_... GAMETORCH_LIVE_SPEND=1 \
  cargo test --test live_spend -- --test-threads=1
```

The CLI has a matching suite in `cli/tests/`: `cli_offline` (no network),
`cli_live`, `cli_live_writes` and `cli_live_spend`, gated by the same env vars:

```sh
cargo test -p gametorch-cli --test cli_offline

GAMETORCH_API_KEY=gt2_... GAMETORCH_BASE_URL=http://localhost:8300/api \
  cargo test -p gametorch-cli --test cli_live -- --test-threads=1

GAMETORCH_API_KEY=gt2_... GAMETORCH_LIVE_WRITES=1 \
  cargo test -p gametorch-cli --test cli_live_writes -- --test-threads=1

GAMETORCH_API_KEY=gt2_... GAMETORCH_LIVE_SPEND=1 \
  cargo test -p gametorch-cli --test cli_live_spend -- --test-threads=1
```

The live tests and the generation examples each create their own project. By
default they delete it again at the end; set `GAMETORCH_KEEP_PROJECT=1` to keep
the project (and its generations, names, labels and metadata) so you can inspect
it in the GameTorch UI.

## License

MIT. See [LICENSE](LICENSE).
