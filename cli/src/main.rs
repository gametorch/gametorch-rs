//! `gametorch` — command-line interface for the GameTorch API.
//!
//! Wraps the official Rust SDK and exposes every public API operation:
//! catalogs, projects, sprites, sounds, animations, exports, saved animations,
//! labels, art styles, usage, API keys, account and health.
//!
//! Run `gametorch --help` or `gametorch <command> --help` for details.

use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use gametorch::{
    ApiKeyScope, ArchiveResponse, Client, CreateApiKeyRequest, ExportFormat, ListParams,
    SpendResetCadence, SpriteMode, UpdateApiKeyRequest,
};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "gametorch",
    version,
    about = "Command-line interface for GameTorch (wraps the official Rust SDK)",
    long_about = "Command-line interface for the GameTorch API.\n\n\
        Authenticate with an admin API key, a project-scoped read/write key, or a \
        bearer token. Set GAMETORCH_API_KEY, GAMETORCH_TOKEN and GAMETORCH_BASE_URL \
        to avoid repeating flags."
)]
struct Cli {
    /// API key (`gt2_...`). Falls back to `GAMETORCH_API_KEY`.
    #[arg(long, env = "GAMETORCH_API_KEY", global = true, hide_env_values = true)]
    api_key: Option<String>,

    /// Clerk session bearer token. Falls back to `GAMETORCH_TOKEN`.
    #[arg(long, env = "GAMETORCH_TOKEN", global = true, hide_env_values = true)]
    token: Option<String>,

    /// API base URL.
    #[arg(
        long,
        env = "GAMETORCH_BASE_URL",
        default_value = "https://gametorch.app/api",
        global = true
    )]
    base_url: String,

    /// Print machine-readable JSON instead of a human summary.
    #[arg(long, global = true)]
    json: bool,

    /// Disable the client-side rate limiter.
    #[arg(long, global = true)]
    no_rate_limit: bool,

    /// Maximum automatic retries for retryable failures.
    #[arg(long, default_value_t = 3, global = true)]
    max_retries: u32,

    /// Request timeout in seconds.
    #[arg(long, default_value_t = 120, global = true)]
    timeout: u64,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check API health.
    Health,
    /// Account operations.
    User {
        #[command(subcommand)]
        command: UserCmd,
    },
    /// Model catalogs.
    Catalog {
        #[command(subcommand)]
        command: CatalogCmd,
    },
    /// Projects.
    Project {
        #[command(subcommand)]
        command: ProjectCmd,
    },
    /// Sprites: generation and assets.
    Sprite {
        #[command(subcommand)]
        command: SpriteCmd,
    },
    /// Sounds: generation and assets.
    Sound {
        #[command(subcommand)]
        command: SoundCmd,
    },
    /// Animations: runs, frames, content and exports.
    Animation {
        #[command(subcommand)]
        command: AnimationCmd,
    },
    /// Saved animations (named frame ranges).
    Saved {
        #[command(subcommand)]
        command: SavedCmd,
    },
    /// Labels.
    Label {
        #[command(subcommand)]
        command: LabelCmd,
    },
    /// Art styles.
    ArtStyle {
        #[command(subcommand)]
        command: ArtStyleCmd,
    },
    /// Usage and spending.
    Usage {
        #[command(subcommand)]
        command: UsageCmd,
    },
    /// API key management (requires an admin key).
    Key {
        #[command(subcommand)]
        command: KeyCmd,
    },
}

#[derive(Subcommand)]
enum UserCmd {
    /// Ensure a mirrored user row exists for the caller.
    Ensure,
}

#[derive(Subcommand)]
enum CatalogCmd {
    /// List sprite/image models.
    Sprite,
    /// List sound models.
    Sound,
    /// List animation models.
    Animation,
}

#[derive(Subcommand)]
enum ProjectCmd {
    /// List projects in the caller's scope.
    List,
    /// Create a project.
    Create {
        /// Project name.
        #[arg(long)]
        name: String,
    },
    /// Rename a project by slug.
    Rename {
        /// Project slug.
        slug: String,
        /// New name.
        #[arg(long)]
        name: String,
    },
    /// Permanently delete a project and its assets by slug.
    Delete {
        /// Project slug.
        slug: String,
    },
}

#[derive(Args, Clone)]
struct ListArgs {
    /// Pagination cursor from a previous page.
    #[arg(long)]
    before: Option<String>,
    /// Include archived items.
    #[arg(long)]
    include_archived: bool,
}

#[derive(Args, Clone)]
struct MetadataArgs {
    /// Metadata entry as KEY=VALUE (repeatable).
    #[arg(long = "metadata", value_name = "KEY=VALUE")]
    metadata: Vec<String>,
}

impl MetadataArgs {
    fn parse(&self) -> Result<HashMap<String, String>> {
        let mut map = HashMap::new();
        for item in &self.metadata {
            let (key, value) = item
                .split_once('=')
                .with_context(|| format!("metadata must be KEY=VALUE, got {item:?}"))?;
            map.insert(key.to_string(), value.to_string());
        }
        Ok(map)
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ModeArg {
    Single,
    Multiple,
}

impl From<ModeArg> for SpriteMode {
    fn from(value: ModeArg) -> Self {
        match value {
            ModeArg::Single => SpriteMode::Single,
            ModeArg::Multiple => SpriteMode::Multiple,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ScopeArg {
    Admin,
    ProjectWrite,
    ProjectRead,
}

impl From<ScopeArg> for ApiKeyScope {
    fn from(value: ScopeArg) -> Self {
        match value {
            ScopeArg::Admin => ApiKeyScope::Admin,
            ScopeArg::ProjectWrite => ApiKeyScope::ProjectWrite,
            ScopeArg::ProjectRead => ApiKeyScope::ProjectRead,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum CadenceArg {
    Daily,
    Weekly,
    Monthly,
    Never,
}

impl From<CadenceArg> for SpendResetCadence {
    fn from(value: CadenceArg) -> Self {
        match value {
            CadenceArg::Daily => SpendResetCadence::Daily,
            CadenceArg::Weekly => SpendResetCadence::Weekly,
            CadenceArg::Monthly => SpendResetCadence::Monthly,
            CadenceArg::Never => SpendResetCadence::Never,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    #[value(name = "texturepacker")]
    TexturePacker,
    #[value(name = "texturepacker.zip")]
    TexturePackerZip,
    #[value(name = "aseprite")]
    Aseprite,
    #[value(name = "godot")]
    Godot,
    #[value(name = "godot.zip")]
    GodotZip,
    #[value(name = "grid")]
    Grid,
    #[value(name = "gamemaker")]
    GameMaker,
    #[value(name = "sequence.zip")]
    SequenceZip,
}

impl From<FormatArg> for ExportFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::TexturePacker => ExportFormat::TexturePacker,
            FormatArg::TexturePackerZip => ExportFormat::TexturePackerZip,
            FormatArg::Aseprite => ExportFormat::Aseprite,
            FormatArg::Godot => ExportFormat::Godot,
            FormatArg::GodotZip => ExportFormat::GodotZip,
            FormatArg::Grid => ExportFormat::Grid,
            FormatArg::GameMaker => ExportFormat::GameMaker,
            FormatArg::SequenceZip => ExportFormat::SequenceZip,
        }
    }
}

#[derive(Args)]
struct GenerateSpriteArgs {
    /// Project id or slug.
    #[arg(long)]
    project: String,
    /// Generation prompt.
    #[arg(long)]
    prompt: String,
    /// Generation mode.
    #[arg(long, value_enum, default_value = "single")]
    mode: ModeArg,
    /// Image model id (see `catalog sprite-models`).
    #[arg(long)]
    image_model: String,
    /// Prompt-enhancement text model id.
    #[arg(long)]
    text_model: Option<String>,
    /// Disable prompt enhancement.
    #[arg(long)]
    no_text_model: bool,
    /// Quality setting.
    #[arg(long)]
    quality: Option<String>,
    /// Resolution setting.
    #[arg(long)]
    resolution: Option<String>,
    /// Edit an existing sprite asset.
    #[arg(long)]
    base_asset_id: Option<Uuid>,
    /// Override the idempotency key.
    #[arg(long)]
    request_id: Option<Uuid>,
    /// Wait for the generation to finish and print the assets.
    #[arg(long)]
    wait: bool,
}

#[derive(Subcommand)]
enum SpriteCmd {
    /// Start a sprite generation.
    Generate(GenerateSpriteArgs),
    /// List a project's generations.
    List {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        #[command(flatten)]
        list: ListArgs,
    },
    /// Get one generation and its assets.
    Get {
        /// Generation id.
        id: Uuid,
        /// Include archived assets.
        #[arg(long)]
        include_archived: bool,
    },
    /// Search a project's sprite assets.
    Assets {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        /// Search query (name or label).
        #[arg(long)]
        q: Option<String>,
    },
    /// Sprite asset operations.
    Asset {
        #[command(subcommand)]
        command: SpriteAssetCmd,
    },
    /// Sprite generation operations (archive/unarchive/delete).
    Generation {
        #[command(subcommand)]
        command: GenerationCmd,
    },
}

#[derive(Subcommand)]
enum SpriteAssetCmd {
    /// Get a sprite asset.
    Get { id: Uuid },
    /// Download the trimmed PNG.
    Content {
        id: Uuid,
        /// Output file, or `-` for stdout.
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Download the uncropped original PNG.
    Original {
        id: Uuid,
        /// Output file, or `-` for stdout.
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Rename a sprite asset. Pass an empty string to clear the name.
    Rename { id: Uuid, name: String },
    /// Replace a sprite asset's metadata.
    Metadata {
        id: Uuid,
        #[command(flatten)]
        metadata: MetadataArgs,
    },
    /// Clear a sprite asset's metadata.
    MetadataClear { id: Uuid },
    /// Archive a sprite asset.
    Archive { id: Uuid },
    /// Restore an archived sprite asset.
    Unarchive { id: Uuid },
    /// Permanently delete a sprite asset.
    Delete { id: Uuid },
}

#[derive(Subcommand)]
enum GenerationCmd {
    /// Archive a generation.
    Archive { id: Uuid },
    /// Restore an archived generation.
    Unarchive { id: Uuid },
    /// Permanently delete a generation.
    Delete { id: Uuid },
}

#[derive(Args)]
struct GenerateSoundArgs {
    /// Project id or slug.
    #[arg(long)]
    project: String,
    /// Generation prompt.
    #[arg(long)]
    prompt: String,
    /// Sound model id (see `catalog sound-models`).
    #[arg(long)]
    sound_model: String,
    /// Output format (for example `mp3` or `pcm`).
    #[arg(long)]
    response_format: Option<String>,
    /// Override the idempotency key.
    #[arg(long)]
    request_id: Option<Uuid>,
    /// Wait for the generation to finish and print the assets.
    #[arg(long)]
    wait: bool,
}

#[derive(Subcommand)]
enum SoundCmd {
    /// Start a sound generation.
    Generate(GenerateSoundArgs),
    /// List a project's sound generations.
    List {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        #[command(flatten)]
        list: ListArgs,
    },
    /// Get one sound generation and its assets.
    Get {
        /// Sound generation id.
        id: Uuid,
        /// Include archived assets.
        #[arg(long)]
        include_archived: bool,
    },
    /// Sound asset operations.
    Asset {
        #[command(subcommand)]
        command: SoundAssetCmd,
    },
    /// Sound generation operations (archive/unarchive).
    Generation {
        #[command(subcommand)]
        command: SoundGenerationCmd,
    },
}

#[derive(Subcommand)]
enum SoundAssetCmd {
    /// Download the audio bytes.
    Content {
        id: Uuid,
        /// Output file, or `-` for stdout.
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Rename a sound asset. Pass an empty string to clear the name.
    Rename { id: Uuid, name: String },
    /// Replace a sound asset's metadata.
    Metadata {
        id: Uuid,
        #[command(flatten)]
        metadata: MetadataArgs,
    },
    /// Clear a sound asset's metadata.
    MetadataClear { id: Uuid },
    /// Archive a sound asset.
    Archive { id: Uuid },
    /// Restore an archived sound asset.
    Unarchive { id: Uuid },
    /// Permanently delete a sound asset.
    Delete { id: Uuid },
}

#[derive(Subcommand)]
enum SoundGenerationCmd {
    /// Archive a sound generation.
    Archive { id: Uuid },
    /// Restore an archived sound generation.
    Unarchive { id: Uuid },
}

#[derive(Args)]
struct EstimateAnimationArgs {
    /// Project id or slug.
    #[arg(long)]
    project: String,
    /// Animation model (`ash`, `birch` or `cedar`).
    #[arg(long)]
    animation_model: String,
    /// Duration in seconds.
    #[arg(long)]
    duration: Option<i64>,
    /// Estimate against an existing sprite asset.
    #[arg(long)]
    base_asset_id: Option<Uuid>,
}

#[derive(Args)]
struct GenerateAnimationArgs {
    /// Project id or slug.
    #[arg(long)]
    project: String,
    /// Generation prompt.
    #[arg(long)]
    prompt: String,
    /// Animation model (`ash`, `birch` or `cedar`).
    #[arg(long)]
    animation_model: String,
    /// Duration in seconds.
    #[arg(long)]
    duration: Option<i64>,
    /// Animate an existing sprite asset.
    #[arg(long)]
    base_asset_id: Option<Uuid>,
    /// Override the idempotency key.
    #[arg(long)]
    request_id: Option<Uuid>,
    /// Wait for the run to finish.
    #[arg(long)]
    wait: bool,
}

#[derive(Subcommand)]
enum AnimationCmd {
    /// Estimate the cost of an animation run.
    Estimate(EstimateAnimationArgs),
    /// Start an animation run.
    Generate(GenerateAnimationArgs),
    /// List a project's animation runs.
    List {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        /// Only runs whose base image is this sprite asset.
        #[arg(long)]
        base_asset_id: Option<Uuid>,
        #[command(flatten)]
        list: ListArgs,
    },
    /// Get one animation run and its frames.
    Get { id: Uuid },
    /// Download the animation clip bytes.
    Content {
        id: Uuid,
        /// Output file, or `-` for stdout.
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Archive an animation run.
    Archive { id: Uuid },
    /// Restore an archived animation run.
    Unarchive { id: Uuid },
    /// Permanently delete an animation run.
    Delete { id: Uuid },
    /// Frame generation and content.
    Frames {
        #[command(subcommand)]
        command: FramesCmd,
    },
    /// Return the export plan without building a file.
    ExportPlan {
        id: Uuid,
        #[arg(long)]
        start_frame: i32,
        #[arg(long)]
        end_frame: i32,
    },
    /// Build and return an export in the given format.
    Export {
        id: Uuid,
        /// Export format.
        #[arg(long, value_enum)]
        format: FormatArg,
        #[arg(long)]
        start_frame: i32,
        #[arg(long)]
        end_frame: i32,
        /// Output file, or `-` for stdout.
        #[arg(long, default_value = "-")]
        output: String,
    },
}

#[derive(Subcommand)]
enum FramesCmd {
    /// Generate PNG frames from a finished animation run.
    Generate {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        /// Animation run id.
        #[arg(long)]
        run: Uuid,
        /// Sampling rate (1-30 fps).
        #[arg(long, default_value_t = 12)]
        fps: i64,
        /// Override the idempotency key.
        #[arg(long)]
        request_id: Option<Uuid>,
    },
    /// Download a frame by its frame id.
    Content {
        /// Frame id.
        id: Uuid,
        /// Output file, or `-` for stdout.
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Download a frame by its 1-based number within a run.
    ContentByNumber {
        /// Animation run id.
        #[arg(long)]
        run: Uuid,
        /// 1-based frame number.
        #[arg(long)]
        number: i64,
        /// Output file, or `-` for stdout.
        #[arg(long, default_value = "-")]
        output: String,
    },
}

#[derive(Subcommand)]
enum SavedCmd {
    /// List a project's saved animations.
    List {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        #[command(flatten)]
        list: ListArgs,
    },
    /// Save a frame range from an animation run as a named preset.
    Save {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        /// Source animation run id.
        #[arg(long)]
        generation_id: Uuid,
        /// First frame (1-based).
        #[arg(long)]
        start_frame: i64,
        /// Last frame (1-based).
        #[arg(long)]
        end_frame: i64,
        /// Optional name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Get one saved animation.
    Get { id: Uuid },
    /// Rename a saved animation. Pass an empty string to clear the name.
    Rename { id: Uuid, name: String },
    /// Replace a saved animation's metadata.
    Metadata {
        id: Uuid,
        #[command(flatten)]
        metadata: MetadataArgs,
    },
    /// Clear a saved animation's metadata.
    MetadataClear { id: Uuid },
    /// Archive a saved animation.
    Archive { id: Uuid },
    /// Restore an archived saved animation.
    Unarchive { id: Uuid },
    /// Permanently delete a saved animation.
    Delete { id: Uuid },
}

/// One of `--asset`, `--sound` or `--saved-animation`, required exactly once.
#[derive(Args)]
#[command(group(
    clap::ArgGroup::new("target")
        .required(true)
        .args(["asset", "sound", "saved_animation"])
))]
struct LabelTargetArgs {
    /// Target a sprite asset.
    #[arg(long)]
    asset: Option<Uuid>,
    /// Target a sound asset.
    #[arg(long)]
    sound: Option<Uuid>,
    /// Target a saved animation.
    #[arg(long)]
    saved_animation: Option<Uuid>,
}

#[derive(Subcommand)]
enum LabelCmd {
    /// List a project's labels.
    List {
        /// Project id or slug.
        #[arg(long)]
        project: String,
    },
    /// Create a label.
    Create {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        /// Label name.
        #[arg(long)]
        name: String,
        /// Hex color, for example `#ca8a04`.
        #[arg(long)]
        color: Option<String>,
    },
    /// Update a label.
    Update {
        id: Uuid,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        color: Option<String>,
    },
    /// Delete a label.
    Delete { id: Uuid },
    /// List the items tagged with a label.
    Items { id: Uuid },
    /// Set or clear a label's thumbnail.
    Thumbnail {
        id: Uuid,
        /// Sprite asset to use as the cover.
        #[arg(long)]
        asset_id: Option<Uuid>,
        /// Clear the thumbnail.
        #[arg(long, conflicts_with = "asset_id")]
        clear: bool,
    },
    /// Add a label to an item.
    Associate {
        #[command(flatten)]
        target: LabelTargetArgs,
        /// Label name.
        #[arg(long)]
        name: String,
    },
    /// Remove a label from an item.
    Remove {
        #[command(flatten)]
        target: LabelTargetArgs,
        /// Label name.
        #[arg(long)]
        name: String,
    },
    /// Dismiss a suggested label on an item.
    Dismiss {
        #[command(flatten)]
        target: LabelTargetArgs,
        /// Label name.
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand)]
enum ArtStyleCmd {
    /// List a project's art styles.
    List {
        /// Project id or slug.
        #[arg(long)]
        project: String,
    },
    /// Create an art style.
    Create {
        /// Project id or slug.
        #[arg(long)]
        project: String,
        /// Art style name.
        #[arg(long)]
        name: String,
    },
    /// Generate a suggested art style.
    Generate {
        /// Project id or slug.
        #[arg(long)]
        project: String,
    },
    /// Delete an art style.
    Delete { id: Uuid },
}

#[derive(Subcommand)]
enum UsageCmd {
    /// Show the credit balance, per-source summary and operation log.
    Show {
        /// Pagination cursor.
        #[arg(long)]
        before: Option<String>,
        /// Break usage out per member (`user`).
        #[arg(long)]
        split: Option<String>,
    },
    /// Show spend over time as histogram buckets.
    Histogram {
        /// Range: `24h`, `7d`, `30d` or `365d`.
        #[arg(long)]
        range: Option<String>,
        /// Filter to a single spend source.
        #[arg(long)]
        source: Option<String>,
        /// Break usage out per member (`user`).
        #[arg(long)]
        split: Option<String>,
    },
}

#[derive(Subcommand)]
enum KeyCmd {
    /// List API keys in the caller's scope (admin only).
    List,
    /// Create an API key with a name, scope, spend limit, reset cadence and expiry.
    Create {
        /// Key name.
        #[arg(long)]
        name: Option<String>,
        /// Key scope.
        #[arg(long, value_enum, default_value = "admin")]
        scope: ScopeArg,
        /// Project id or slug (required for project scopes).
        #[arg(long)]
        project: Option<String>,
        /// Maximum spend in the current cycle, in credits.
        #[arg(long)]
        max_spend_limit: Option<String>,
        /// Spend reset cadence.
        #[arg(long, value_enum)]
        spend_reset_cadence: Option<CadenceArg>,
        /// Absolute expiry as RFC 3339, for example `2026-12-31T00:00:00Z`.
        #[arg(long, conflicts_with = "expires_in")]
        expires_at: Option<String>,
        /// Relative expiry, for example `30d`, `12h`, `90m`.
        #[arg(long)]
        expires_in: Option<String>,
    },
    /// Update an API key's name, spend limit, reset cadence or expiry.
    Update {
        id: Uuid,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        max_spend_limit: Option<String>,
        #[arg(long, value_enum)]
        spend_reset_cadence: Option<CadenceArg>,
        #[arg(long, conflicts_with = "expires_in")]
        expires_at: Option<String>,
        #[arg(long)]
        expires_in: Option<String>,
    },
    /// Revoke an API key.
    Delete { id: Uuid },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli).await {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let client = build_client(&cli)?;
    let json = cli.json;

    match cli.command {
        Command::Health => {
            let status = client.health().await?;
            emit_plain(json, &status, || println!("{status}"));
        }
        Command::User { command } => match command {
            UserCmd::Ensure => {
                let response = client.ensure_user().await?;
                emit(json, &response, |r| println!("ok={}", r.ok))?;
            }
        },
        Command::Catalog { command } => run_catalog(&client, json, command).await?,
        Command::Project { command } => run_project(&client, json, command).await?,
        Command::Sprite { command } => run_sprite(&client, json, command).await?,
        Command::Sound { command } => run_sound(&client, json, command).await?,
        Command::Animation { command } => run_animation(&client, json, command).await?,
        Command::Saved { command } => run_saved(&client, json, command).await?,
        Command::Label { command } => run_label(&client, json, command).await?,
        Command::ArtStyle { command } => run_art_style(&client, json, command).await?,
        Command::Usage { command } => run_usage(&client, json, command).await?,
        Command::Key { command } => run_key(&client, json, command).await?,
    }
    Ok(())
}

fn build_client(cli: &Cli) -> Result<Client> {
    let mut builder = Client::builder()
        .base_url(&cli.base_url)
        .max_retries(cli.max_retries)
        .rate_limit(!cli.no_rate_limit)
        .timeout(Duration::from_secs(cli.timeout));
    if let Some(token) = &cli.token {
        builder = builder.bearer_token(token);
    } else if let Some(api_key) = &cli.api_key {
        builder = builder.api_key(api_key);
    }
    Ok(builder.build()?)
}

/// Prints JSON when `json` is set, otherwise calls the human formatter.
fn emit<T: Serialize>(json: bool, value: &T, human: impl FnOnce(&T)) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        human(value);
    }
    Ok(())
}

fn emit_plain(json: bool, value: &str, human: impl FnOnce()) {
    if json {
        println!(
            "{}",
            serde_json::to_string(&value).unwrap_or_else(|_| "\"\"".into())
        );
    } else {
        human();
    }
}

async fn resolve_project(client: &Client, project: &str) -> Result<Uuid> {
    if let Ok(id) = Uuid::parse_str(project) {
        return Ok(id);
    }
    let projects = client.list_projects().await?;
    projects
        .projects
        .iter()
        .find(|p| p.slug == project || p.name.eq_ignore_ascii_case(project))
        .map(|p| p.id)
        .with_context(|| format!("no project matches {project:?}"))
}

fn list_params(list: &ListArgs) -> ListParams {
    ListParams {
        before: list.before.clone(),
        include_archived: list.include_archived,
        base_asset_id: None,
    }
}

fn write_download(download: &gametorch::Download, output: &str) -> Result<()> {
    if output == "-" {
        std::io::stdout().write_all(download.as_bytes())?;
        std::io::stdout().flush()?;
    } else {
        std::fs::write(output, download.as_bytes()).with_context(|| format!("writing {output}"))?;
        eprintln!("wrote {} bytes to {output}", download.len());
    }
    Ok(())
}

fn parse_decimal(value: &str) -> Result<Decimal> {
    value
        .parse::<Decimal>()
        .with_context(|| format!("invalid decimal: {value:?}"))
}

fn parse_expiry(
    at: Option<&str>,
    in_: Option<&str>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
    use chrono::{DateTime, Duration, Utc};
    if let Some(spec) = in_ {
        let (amount, unit) = spec.split_at(
            spec.find(|c: char| !c.is_ascii_digit())
                .filter(|i| *i > 0)
                .unwrap_or(spec.len()),
        );
        let amount: i64 = amount
            .parse()
            .with_context(|| format!("invalid duration: {spec:?}"))?;
        let duration = match unit {
            "s" => Duration::seconds(amount),
            "m" => Duration::minutes(amount),
            "h" => Duration::hours(amount),
            "d" => Duration::days(amount),
            "w" => Duration::weeks(amount),
            other => bail!("unknown duration unit {other:?} (use s, m, h, d or w)"),
        };
        return Ok(Some(Utc::now() + duration));
    }
    match at {
        Some(value) => Ok(Some(
            DateTime::parse_from_rfc3339(value)
                .with_context(|| format!("invalid RFC 3339 timestamp: {value:?}"))?
                .with_timezone(&Utc),
        )),
        None => Ok(None),
    }
}

async fn run_catalog(client: &Client, json: bool, command: CatalogCmd) -> Result<()> {
    match command {
        CatalogCmd::Sprite => {
            let models = client.sprite_models().await?;
            emit(json, &models, |m| {
                println!("default text model: {}", m.default_text_model);
                for model in &m.image_models {
                    println!(
                        "{}  {:<28} {}  {}",
                        if model.available { " " } else { "x" },
                        model.id,
                        model.name,
                        model.blurb
                    );
                }
                println!("\ntext models:");
                for model in &m.text_models {
                    println!("  {}  {}", model.id, model.name);
                }
            })?;
        }
        CatalogCmd::Sound => {
            let models = client.sound_models().await?;
            emit(json, &models, |m| {
                println!("model: {} ({})", m.model.name, m.model.id);
                println!("default format: {}", m.default_format);
                println!("formats: {}", m.formats.join(", "));
            })?;
        }
        CatalogCmd::Animation => {
            let models = client.animation_models().await?;
            emit(json, &models, |m| {
                for model in &m.data {
                    println!(
                        "{}  {}  durations={:?}  resolutions={:?}",
                        model.id,
                        model.name,
                        model.supported_durations,
                        model.supported_resolutions
                    );
                }
            })?;
        }
    }
    Ok(())
}

async fn run_project(client: &Client, json: bool, command: ProjectCmd) -> Result<()> {
    match command {
        ProjectCmd::List => {
            let projects = client.list_projects().await?;
            emit(json, &projects, |p| {
                for project in &p.projects {
                    println!("{}  {:<32} {}", project.id, project.slug, project.name);
                }
            })?;
        }
        ProjectCmd::Create { name } => {
            let project = client.create_project(&name).await?;
            emit(json, &project, |p| {
                println!("{}  {}  {}", p.id, p.slug, p.name)
            })?;
        }
        ProjectCmd::Rename { slug, name } => {
            let project = client.rename_project(&slug, &name).await?;
            emit(json, &project, |p| {
                println!("{}  {}  {}", p.id, p.slug, p.name)
            })?;
        }
        ProjectCmd::Delete { slug } => {
            let response = client.delete_project(&slug).await?;
            emit(json, &response, |r| println!("ok={}", r.ok))?;
        }
    }
    Ok(())
}

async fn run_sprite(client: &Client, json: bool, command: SpriteCmd) -> Result<()> {
    match command {
        SpriteCmd::Generate(args) => {
            let project_id = resolve_project(client, &args.project).await?;
            let mut builder = client
                .generate_sprite(project_id)
                .prompt(&args.prompt)
                .mode(args.mode.into())
                .image_model(&args.image_model);
            if args.no_text_model {
                builder = builder.no_text_model();
            } else if let Some(text_model) = &args.text_model {
                builder = builder.text_model(text_model);
            }
            if let Some(quality) = &args.quality {
                builder = builder.quality(quality);
            }
            if let Some(resolution) = &args.resolution {
                builder = builder.resolution(resolution);
            }
            if let Some(base_asset_id) = args.base_asset_id {
                builder = builder.base_asset_id(base_asset_id);
            }
            if let Some(request_id) = args.request_id {
                builder = builder.request_id(request_id);
            }
            let job = builder.send().await?;
            emit(json, &job, |job| {
                println!(
                    "{}  {}  created={}  reserved={}",
                    job.id, job.status, job.created, job.reserved_credits
                );
            })?;
            if args.wait {
                wait_generation(client, job.id, json).await?;
            }
        }
        SpriteCmd::List { project, list } => {
            let project_id = resolve_project(client, &project).await?;
            let response = client
                .list_generations(project_id, &list_params(&list))
                .await?;
            emit(json, &response, |r| {
                for generation in &r.generations {
                    println!(
                        "{}  {}  {} asset(s)  {}",
                        generation.id,
                        generation.status,
                        generation.assets.len(),
                        generation.prompt
                    );
                }
                print_cursor(r.next_cursor.as_deref(), r.total);
            })?;
        }
        SpriteCmd::Get {
            id,
            include_archived,
        } => {
            let generation = client.get_generation(id, include_archived).await?;
            emit(json, &generation, |g| {
                println!("{}  {}  {}", g.id, g.status, g.prompt);
                for asset in &g.assets {
                    println!(
                        "  {}  {}x{}  name={:?} labels={:?}",
                        asset.id, asset.width, asset.height, asset.name, asset.labels
                    );
                }
            })?;
        }
        SpriteCmd::Assets { project, q } => {
            let project_id = resolve_project(client, &project).await?;
            let assets = client.list_sprite_assets(project_id, q.as_deref()).await?;
            emit(json, &assets, |a| {
                for asset in &a.assets {
                    println!(
                        "{}  {}x{}  name={:?} labels={:?}",
                        asset.id, asset.width, asset.height, asset.name, asset.labels
                    );
                }
            })?;
        }
        SpriteCmd::Asset { command } => run_sprite_asset(client, json, command).await?,
        SpriteCmd::Generation { command } => match command {
            GenerationCmd::Archive { id } => {
                let response = client.archive_generation(id).await?;
                emit_archive(json, &response)?;
            }
            GenerationCmd::Unarchive { id } => {
                let response = client.unarchive_generation(id).await?;
                emit_archive(json, &response)?;
            }
            GenerationCmd::Delete { id } => {
                client.delete_generation(id).await?;
                emit_deleted(json, &id)?;
            }
        },
    }
    Ok(())
}

async fn run_sprite_asset(client: &Client, json: bool, command: SpriteAssetCmd) -> Result<()> {
    match command {
        SpriteAssetCmd::Get { id } => {
            let asset = client.get_asset(id).await?;
            emit(json, &asset, |a| {
                println!("{}  {}x{}  name={:?}", a.id, a.width, a.height, a.name)
            })?;
        }
        SpriteAssetCmd::Content { id, output } => {
            let download = client.asset_content(id).await?;
            write_download(&download, &output)?;
        }
        SpriteAssetCmd::Original { id, output } => {
            let download = client.asset_original(id).await?;
            write_download(&download, &output)?;
        }
        SpriteAssetCmd::Rename { id, name } => {
            let name = optional_name(&name);
            let response = client.rename_asset(id, name.as_deref()).await?;
            emit(json, &response, |r| {
                println!("ok={} name={:?}", r.ok, r.name)
            })?;
        }
        SpriteAssetCmd::Metadata { id, metadata } => {
            let map = metadata.parse()?;
            let response = client.put_asset_metadata(id, &map).await?;
            emit(json, &response, |r| {
                println!("ok={} metadata={:?}", r.ok, r.metadata)
            })?;
        }
        SpriteAssetCmd::MetadataClear { id } => {
            let response = client.put_asset_metadata(id, &HashMap::new()).await?;
            emit(json, &response, |r| {
                println!("ok={} metadata={:?}", r.ok, r.metadata)
            })?;
        }
        SpriteAssetCmd::Archive { id } => {
            let response = client.archive_asset(id).await?;
            emit_archive(json, &response)?;
        }
        SpriteAssetCmd::Unarchive { id } => {
            let response = client.unarchive_asset(id).await?;
            emit_archive(json, &response)?;
        }
        SpriteAssetCmd::Delete { id } => {
            client.delete_asset(id).await?;
            emit_deleted(json, &id)?;
        }
    }
    Ok(())
}

async fn run_sound(client: &Client, json: bool, command: SoundCmd) -> Result<()> {
    match command {
        SoundCmd::Generate(args) => {
            let project_id = resolve_project(client, &args.project).await?;
            let mut builder = client
                .generate_sound(project_id)
                .prompt(&args.prompt)
                .sound_model(&args.sound_model);
            if let Some(format) = &args.response_format {
                builder = builder.response_format(format);
            }
            if let Some(request_id) = args.request_id {
                builder = builder.request_id(request_id);
            }
            let job = builder.send().await?;
            emit(json, &job, |job| {
                println!(
                    "{}  {}  created={}  reserved={}",
                    job.id, job.status, job.created, job.reserved_credits
                );
            })?;
            if args.wait {
                wait_sound(client, job.id, json).await?;
            }
        }
        SoundCmd::List { project, list } => {
            let project_id = resolve_project(client, &project).await?;
            let response = client
                .list_sound_generations(project_id, &list_params(&list))
                .await?;
            emit(json, &response, |r| {
                for generation in &r.sound_generations {
                    println!(
                        "{}  {}  {} asset(s)  {}",
                        generation.id,
                        generation.status,
                        generation.assets.len(),
                        generation.prompt
                    );
                }
                print_cursor(r.next_cursor.as_deref(), r.total);
            })?;
        }
        SoundCmd::Get {
            id,
            include_archived,
        } => {
            let generation = client.get_sound_generation(id, include_archived).await?;
            emit(json, &generation, |g| {
                println!("{}  {}  {}", g.id, g.status, g.prompt);
                for asset in &g.assets {
                    println!("  {}  {}  name={:?}", asset.id, asset.format, asset.name);
                }
            })?;
        }
        SoundCmd::Asset { command } => run_sound_asset(client, json, command).await?,
        SoundCmd::Generation { command } => match command {
            SoundGenerationCmd::Archive { id } => {
                let response = client.archive_sound_generation(id).await?;
                emit_archive(json, &response)?;
            }
            SoundGenerationCmd::Unarchive { id } => {
                let response = client.unarchive_sound_generation(id).await?;
                emit_archive(json, &response)?;
            }
        },
    }
    Ok(())
}

async fn run_sound_asset(client: &Client, json: bool, command: SoundAssetCmd) -> Result<()> {
    match command {
        SoundAssetCmd::Content { id, output } => {
            let download = client.sound_asset_content(id).await?;
            write_download(&download, &output)?;
        }
        SoundAssetCmd::Rename { id, name } => {
            let name = optional_name(&name);
            let response = client.rename_sound_asset(id, name.as_deref()).await?;
            emit(json, &response, |r| {
                println!("ok={} name={:?}", r.ok, r.name)
            })?;
        }
        SoundAssetCmd::Metadata { id, metadata } => {
            let map = metadata.parse()?;
            let response = client.put_sound_asset_metadata(id, &map).await?;
            emit(json, &response, |r| {
                println!("ok={} metadata={:?}", r.ok, r.metadata)
            })?;
        }
        SoundAssetCmd::MetadataClear { id } => {
            let response = client.put_sound_asset_metadata(id, &HashMap::new()).await?;
            emit(json, &response, |r| {
                println!("ok={} metadata={:?}", r.ok, r.metadata)
            })?;
        }
        SoundAssetCmd::Archive { id } => {
            let response = client.archive_sound_asset(id).await?;
            emit_archive(json, &response)?;
        }
        SoundAssetCmd::Unarchive { id } => {
            let response = client.unarchive_sound_asset(id).await?;
            emit_archive(json, &response)?;
        }
        SoundAssetCmd::Delete { id } => {
            client.delete_sound_asset(id).await?;
            emit_deleted(json, &id)?;
        }
    }
    Ok(())
}

async fn run_animation(client: &Client, json: bool, command: AnimationCmd) -> Result<()> {
    match command {
        AnimationCmd::Estimate(args) => {
            let project_id = resolve_project(client, &args.project).await?;
            let mut builder = client
                .estimate_animation(project_id)
                .animation_model(&args.animation_model);
            if let Some(duration) = args.duration {
                builder = builder.duration(duration);
            }
            if let Some(base_asset_id) = args.base_asset_id {
                builder = builder.base_asset_id(base_asset_id);
            }
            let estimate = builder.send().await?;
            emit(json, &estimate, |e| {
                println!(
                    "{}  {}s  {}  {} credits (${})  reserve {}",
                    e.animation_model,
                    e.duration,
                    e.resolution,
                    e.credits,
                    e.usd,
                    e.reserved_credits
                );
            })?;
        }
        AnimationCmd::Generate(args) => {
            let project_id = resolve_project(client, &args.project).await?;
            let mut builder = client
                .generate_animation(project_id)
                .prompt(&args.prompt)
                .animation_model(&args.animation_model);
            if let Some(duration) = args.duration {
                builder = builder.duration(duration);
            }
            if let Some(base_asset_id) = args.base_asset_id {
                builder = builder.base_asset_id(base_asset_id);
            }
            if let Some(request_id) = args.request_id {
                builder = builder.request_id(request_id);
            }
            let job = builder.send().await?;
            emit(json, &job, |job| {
                println!(
                    "{}  {}  created={}  reserved={}",
                    job.id, job.status, job.created, job.reserved_credits
                );
            })?;
            if args.wait {
                wait_animation(client, job.id, json).await?;
            }
        }
        AnimationCmd::List {
            project,
            base_asset_id,
            list,
        } => {
            let project_id = resolve_project(client, &project).await?;
            let params = ListParams {
                before: list.before.clone(),
                include_archived: list.include_archived,
                base_asset_id,
            };
            let response = client.list_animation_runs(project_id, &params).await?;
            emit(json, &response, |r| {
                for run in &r.animations {
                    println!(
                        "{}  {}  {} frame(s)  base={:?}  {}",
                        run.id,
                        run.status,
                        run.frames.len(),
                        run.base_asset_id,
                        run.prompt
                    );
                }
                print_cursor(r.next_cursor.as_deref(), r.total);
            })?;
        }
        AnimationCmd::Get { id } => {
            let run = client.get_animation_run(id).await?;
            emit(json, &run, |r| {
                println!(
                    "{}  {}  {} frame(s)  base={:?}",
                    r.id,
                    r.status,
                    r.frames.len(),
                    r.base_asset_id
                );
            })?;
        }
        AnimationCmd::Content { id, output } => {
            let download = client.animation_content(id).await?;
            write_download(&download, &output)?;
        }
        AnimationCmd::Archive { id } => {
            let response = client.archive_animation_run(id).await?;
            emit_archive(json, &response)?;
        }
        AnimationCmd::Unarchive { id } => {
            let response = client.unarchive_animation_run(id).await?;
            emit_archive(json, &response)?;
        }
        AnimationCmd::Delete { id } => {
            client.delete_animation_run(id).await?;
            emit_deleted(json, &id)?;
        }
        AnimationCmd::Frames { command } => match command {
            FramesCmd::Generate {
                project,
                run,
                fps,
                request_id,
            } => {
                let project_id = resolve_project(client, &project).await?;
                let mut builder = client.generate_frames(project_id, run).fps(fps);
                if let Some(request_id) = request_id {
                    builder = builder.request_id(request_id);
                }
                let job = builder.send().await?;
                emit(json, &job, |job| {
                    println!(
                        "{}  {}  reserved={}",
                        job.id, job.status, job.reserved_credits
                    )
                })?;
            }
            FramesCmd::Content { id, output } => {
                let download = client.frame_content(id).await?;
                write_download(&download, &output)?;
            }
            FramesCmd::ContentByNumber {
                run,
                number,
                output,
            } => {
                let download = client.frame_content_by_number(run, number).await?;
                write_download(&download, &output)?;
            }
        },
        AnimationCmd::ExportPlan {
            id,
            start_frame,
            end_frame,
        } => {
            let plan = client.export_plan(id, start_frame, end_frame).await?;
            emit(json, &plan, |p| {
                println!(
                    "frames {}-{}  {} frame(s)  canvas {}x{}",
                    p.start_frame, p.end_frame, p.frame_count, p.canvas_width, p.canvas_height
                );
            })?;
        }
        AnimationCmd::Export {
            id,
            format,
            start_frame,
            end_frame,
            output,
        } => {
            let export = client
                .export(id, format.into(), start_frame, end_frame)
                .await?;
            write_export(export, &output, json)?;
        }
    }
    Ok(())
}

fn write_export(export: gametorch::Export, output: &str, json: bool) -> Result<()> {
    use gametorch::Export;
    match export {
        Export::Binary(download) => write_download(&download, output)?,
        Export::TexturePacker(value) => write_json_export(&value, output, json)?,
        Export::Godot(value) => write_json_export(&value, output, json)?,
    }
    Ok(())
}

fn write_json_export<T: Serialize>(value: &T, output: &str, json: bool) -> Result<()> {
    let text = if json {
        serde_json::to_string(value)?
    } else {
        serde_json::to_string_pretty(value)?
    };
    if output == "-" {
        println!("{text}");
    } else {
        std::fs::write(output, text).with_context(|| format!("writing {output}"))?;
        eprintln!("wrote JSON export to {output}");
    }
    Ok(())
}

async fn run_saved(client: &Client, json: bool, command: SavedCmd) -> Result<()> {
    match command {
        SavedCmd::List { project, list } => {
            let project_id = resolve_project(client, &project).await?;
            let response = client
                .list_saved_animations(project_id, &list_params(&list))
                .await?;
            emit(json, &response, |r| {
                for saved in &r.saved_animations {
                    println!(
                        "{}  name={:?}  frames {}-{}  labels={:?}",
                        saved.id, saved.name, saved.start_frame, saved.end_frame, saved.labels
                    );
                }
                print_cursor(r.next_cursor.as_deref(), r.total);
            })?;
        }
        SavedCmd::Save {
            project,
            generation_id,
            start_frame,
            end_frame,
            name,
        } => {
            let project_id = resolve_project(client, &project).await?;
            let mut builder = client
                .save_animation(project_id)
                .generation_id(generation_id)
                .range(start_frame, end_frame);
            if let Some(name) = name {
                builder = builder.name(name);
            }
            let saved = builder.send().await?;
            emit(json, &saved, |s| {
                println!(
                    "{}  name={:?}  frames {}-{}",
                    s.id, s.name, s.start_frame, s.end_frame
                )
            })?;
        }
        SavedCmd::Get { id } => {
            let saved = client.get_saved_animation(id).await?;
            emit(json, &saved, |s| {
                println!(
                    "{}  name={:?}  frames {}-{}",
                    s.id, s.name, s.start_frame, s.end_frame
                )
            })?;
        }
        SavedCmd::Rename { id, name } => {
            let name = optional_name(&name);
            let response = client.rename_saved_animation(id, name.as_deref()).await?;
            emit(json, &response, |r| {
                println!("ok={} name={:?}", r.ok, r.name)
            })?;
        }
        SavedCmd::Metadata { id, metadata } => {
            let map = metadata.parse()?;
            let response = client.put_saved_animation_metadata(id, &map).await?;
            emit(json, &response, |r| {
                println!("ok={} metadata={:?}", r.ok, r.metadata)
            })?;
        }
        SavedCmd::MetadataClear { id } => {
            let response = client
                .put_saved_animation_metadata(id, &HashMap::new())
                .await?;
            emit(json, &response, |r| {
                println!("ok={} metadata={:?}", r.ok, r.metadata)
            })?;
        }
        SavedCmd::Archive { id } => {
            let response = client.archive_saved_animation(id).await?;
            emit_archive(json, &response)?;
        }
        SavedCmd::Unarchive { id } => {
            let response = client.unarchive_saved_animation(id).await?;
            emit_archive(json, &response)?;
        }
        SavedCmd::Delete { id } => {
            client.delete_saved_animation(id).await?;
            emit_deleted(json, &id)?;
        }
    }
    Ok(())
}

async fn run_label(client: &Client, json: bool, command: LabelCmd) -> Result<()> {
    match command {
        LabelCmd::List { project } => {
            let project_id = resolve_project(client, &project).await?;
            let labels = client.list_labels(project_id).await?;
            emit(json, &labels, |l| {
                for label in &l.labels {
                    println!(
                        "{}  {:<24} {}  thumb={:?}",
                        label.id,
                        label.name,
                        label.color.as_deref().unwrap_or("-"),
                        label.thumbnail_asset_id
                    );
                }
            })?;
        }
        LabelCmd::Create {
            project,
            name,
            color,
        } => {
            let project_id = resolve_project(client, &project).await?;
            let label = client
                .create_label(project_id, &name, color.as_deref())
                .await?;
            emit(json, &label, |l| {
                println!(
                    "{}  {}  {}",
                    l.id,
                    l.name,
                    l.color.as_deref().unwrap_or("-")
                )
            })?;
        }
        LabelCmd::Update { id, name, color } => {
            let label = client
                .update_label(id, name.as_deref(), color.as_deref())
                .await?;
            emit(json, &label, |l| {
                println!(
                    "{}  {}  {}",
                    l.id,
                    l.name,
                    l.color.as_deref().unwrap_or("-")
                )
            })?;
        }
        LabelCmd::Delete { id } => {
            let response = client.delete_label(id).await?;
            emit(json, &response, |r| println!("ok={}", r.ok))?;
        }
        LabelCmd::Items { id } => {
            let items = client.label_items(id).await?;
            emit(json, &items, |i| {
                println!("label: {} ({})", i.label.name, i.label.id);
                println!(
                    "  {} sprite(s), {} sound(s), {} saved animation(s)",
                    i.assets.len(),
                    i.sounds.len(),
                    i.saved_animations.len()
                );
            })?;
        }
        LabelCmd::Thumbnail {
            id,
            asset_id,
            clear,
        } => {
            let asset_id = if clear { None } else { asset_id };
            let label = client.set_label_thumbnail(id, asset_id).await?;
            emit(json, &label, |l| {
                println!("{}  thumb={:?}", l.name, l.thumbnail_asset_id)
            })?;
        }
        LabelCmd::Associate { target, name } => {
            let association = associate_label(client, &target, &name).await?;
            emit(json, &association, |a| {
                println!("ok={} labels={:?}", a.ok, a.labels)
            })?;
        }
        LabelCmd::Remove { target, name } => {
            let association = remove_label(client, &target, &name).await?;
            emit(json, &association, |a| {
                println!("ok={} labels={:?}", a.ok, a.labels)
            })?;
        }
        LabelCmd::Dismiss { target, name } => {
            let response = dismiss_label(client, &target, &name).await?;
            emit(json, &response, |r| println!("ok={}", r.ok))?;
        }
    }
    Ok(())
}

async fn associate_label(
    client: &Client,
    target: &LabelTargetArgs,
    name: &str,
) -> Result<gametorch::LabelAssociation> {
    if let Some(id) = target.asset {
        Ok(client.associate_asset_label(id, name).await?)
    } else if let Some(id) = target.sound {
        Ok(client.associate_sound_label(id, name).await?)
    } else if let Some(id) = target.saved_animation {
        Ok(client.associate_saved_animation_label(id, name).await?)
    } else {
        bail!("one of --asset, --sound or --saved-animation is required")
    }
}

async fn remove_label(
    client: &Client,
    target: &LabelTargetArgs,
    name: &str,
) -> Result<gametorch::LabelAssociation> {
    if let Some(id) = target.asset {
        Ok(client.remove_asset_label(id, name).await?)
    } else if let Some(id) = target.sound {
        Ok(client.remove_sound_label(id, name).await?)
    } else if let Some(id) = target.saved_animation {
        Ok(client.remove_saved_animation_label(id, name).await?)
    } else {
        bail!("one of --asset, --sound or --saved-animation is required")
    }
}

async fn dismiss_label(
    client: &Client,
    target: &LabelTargetArgs,
    name: &str,
) -> Result<gametorch::OkResponse> {
    if let Some(id) = target.asset {
        Ok(client.dismiss_asset_label_suggestion(id, name).await?)
    } else if let Some(id) = target.sound {
        Ok(client.dismiss_sound_label_suggestion(id, name).await?)
    } else {
        bail!("dismiss is only supported for --asset or --sound targets")
    }
}

async fn run_art_style(client: &Client, json: bool, command: ArtStyleCmd) -> Result<()> {
    match command {
        ArtStyleCmd::List { project } => {
            let project_id = resolve_project(client, &project).await?;
            let styles = client.list_art_styles(project_id).await?;
            emit(json, &styles, |s| {
                for style in &s.art_styles {
                    println!("{}  {}", style.id, style.name);
                }
            })?;
        }
        ArtStyleCmd::Create { project, name } => {
            let project_id = resolve_project(client, &project).await?;
            let style = client.create_art_style(project_id, &name).await?;
            emit(json, &style, |s| println!("{}  {}", s.id, s.name))?;
        }
        ArtStyleCmd::Generate { project } => {
            let project_id = resolve_project(client, &project).await?;
            let suggestion = client.generate_art_style(project_id).await?;
            emit(json, &suggestion, |s| println!("{}", s.name))?;
        }
        ArtStyleCmd::Delete { id } => {
            let response = client.delete_art_style(id).await?;
            emit(json, &response, |r| println!("ok={}", r.ok))?;
        }
    }
    Ok(())
}

async fn run_usage(client: &Client, json: bool, command: UsageCmd) -> Result<()> {
    match command {
        UsageCmd::Show { before, split } => {
            let usage = client.usage(before.as_deref(), split.as_deref()).await?;
            emit(json, &usage, |u| {
                println!(
                    "balance={:?} reserved={:?} records={} total={}",
                    u.balance_credits,
                    u.reserved_credits,
                    u.records.len(),
                    u.total
                );
                for record in &u.records {
                    println!(
                        "  {}  {}  {:?}  {} credits  settled={:?}",
                        record.created_at,
                        record.source,
                        record.operation,
                        record.credits_consumed,
                        record.settled
                    );
                }
                print_cursor(u.next_cursor.as_deref(), u.total);
            })?;
        }
        UsageCmd::Histogram {
            range,
            source,
            split,
        } => {
            let histogram = client
                .usage_histogram(range.as_deref(), source.as_deref(), split.as_deref())
                .await?;
            emit(json, &histogram, |h| {
                println!(
                    "range={} width={}s buckets={}",
                    h.range,
                    h.width_seconds,
                    h.buckets.len()
                );
                for bucket in &h.buckets {
                    println!("  {}  {} credits", bucket.start, bucket.total);
                }
            })?;
        }
    }
    Ok(())
}

async fn run_key(client: &Client, json: bool, command: KeyCmd) -> Result<()> {
    match command {
        KeyCmd::List => {
            let keys = client.list_keys().await?;
            emit(json, &keys, |k| {
                for key in &k.keys {
                    println!(
                        "{}  {:<24} scope={:<13} project={:?} limit={:?} cadence={} prefix={}",
                        key.id,
                        key.name.as_deref().unwrap_or("<unnamed>"),
                        key.key_scope,
                        key.project_id,
                        key.max_spend_limit,
                        key.spend_reset_cadence,
                        key.key_prefix
                    );
                }
            })?;
        }
        KeyCmd::Create {
            name,
            scope,
            project,
            max_spend_limit,
            spend_reset_cadence,
            expires_at,
            expires_in,
        } => {
            let scope: ApiKeyScope = scope.into();
            let project_id = match project {
                Some(project) => Some(resolve_project(client, &project).await?),
                None => None,
            };
            if matches!(scope, ApiKeyScope::ProjectRead | ApiKeyScope::ProjectWrite)
                && project_id.is_none()
            {
                bail!("--project is required for project scopes");
            }
            if scope == ApiKeyScope::Admin && project_id.is_some() {
                bail!("--project must be omitted for admin scope");
            }
            let mut request = match scope {
                ApiKeyScope::Admin => CreateApiKeyRequest::admin(),
                ApiKeyScope::ProjectWrite => {
                    CreateApiKeyRequest::project_write(project_id.unwrap())
                }
                ApiKeyScope::ProjectRead => CreateApiKeyRequest::project_read(project_id.unwrap()),
            };
            if let Some(name) = name {
                request = request.name(name);
            }
            if let Some(limit) = max_spend_limit {
                request = request.max_spend_limit(parse_decimal(&limit)?);
            }
            if let Some(cadence) = spend_reset_cadence {
                request = request.spend_reset_cadence(cadence.into());
            }
            if let Some(expires_at) = parse_expiry(expires_at.as_deref(), expires_in.as_deref())? {
                request = request.expires_at(expires_at);
            }
            let created = client.create_key(&request).await?;
            emit(json, &created, |c| {
                println!(
                    "{}  scope={} project={:?} name={:?} limit={:?} cadence={} expires_at={:?}",
                    c.key.id,
                    c.key.key_scope,
                    c.key.project_id,
                    c.key.name,
                    c.key.max_spend_limit,
                    c.key.spend_reset_cadence,
                    c.key.expires_at
                );
                println!("key_full (shown once): {}", c.key_full);
            })?;
        }
        KeyCmd::Update {
            id,
            name,
            max_spend_limit,
            spend_reset_cadence,
            expires_at,
            expires_in,
        } => {
            let mut request = UpdateApiKeyRequest::new();
            if let Some(name) = name {
                request = request.name(name);
            }
            if let Some(limit) = max_spend_limit {
                request = request.max_spend_limit(parse_decimal(&limit)?);
            }
            if let Some(cadence) = spend_reset_cadence {
                request = request.spend_reset_cadence(cadence.into());
            }
            if let Some(expires_at) = parse_expiry(expires_at.as_deref(), expires_in.as_deref())? {
                request = request.expires_at(expires_at);
            }
            let key = client.update_key(id, &request).await?;
            emit(json, &key, |k| {
                println!(
                    "{}  name={:?} limit={:?} cadence={} expires_at={:?}",
                    k.id, k.name, k.max_spend_limit, k.spend_reset_cadence, k.expires_at
                );
            })?;
        }
        KeyCmd::Delete { id } => {
            let response = client.delete_key(id).await?;
            emit(json, &response, |r| println!("ok={}", r.ok))?;
        }
    }
    Ok(())
}

async fn wait_generation(client: &Client, id: Uuid, json: bool) -> Result<()> {
    for _ in 0..180 {
        let generation = client.get_generation(id, true).await?;
        if generation.status != "queued" && generation.status != "running" {
            emit(json, &generation, |g| {
                println!("{}  {}", g.id, g.status);
                for asset in &g.assets {
                    println!(
                        "  {}  {}x{}  name={:?}",
                        asset.id, asset.width, asset.height, asset.name
                    );
                }
            })?;
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    bail!("timed out waiting for generation {id}")
}

async fn wait_sound(client: &Client, id: Uuid, json: bool) -> Result<()> {
    for _ in 0..180 {
        let generation = client.get_sound_generation(id, true).await?;
        if generation.status != "queued" && generation.status != "running" {
            emit(json, &generation, |g| {
                println!("{}  {}", g.id, g.status);
                for asset in &g.assets {
                    println!("  {}  {}  name={:?}", asset.id, asset.format, asset.name);
                }
            })?;
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    bail!("timed out waiting for sound generation {id}")
}

async fn wait_animation(client: &Client, id: Uuid, json: bool) -> Result<()> {
    for _ in 0..240 {
        let run = client.get_animation_run(id).await?;
        if run.status != "queued" && run.status != "running" {
            emit(json, &run, |r| {
                println!("{}  {}  {} frame(s)", r.id, r.status, r.frames.len());
            })?;
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    bail!("timed out waiting for animation run {id}")
}

fn optional_name(name: &str) -> Option<String> {
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn print_cursor(next_cursor: Option<&str>, total: i64) {
    match next_cursor {
        Some(cursor) => println!("next_cursor: {cursor}  (total {total})"),
        None => println!("(total {total})"),
    }
}

fn emit_archive(json: bool, response: &ArchiveResponse) -> Result<()> {
    emit(json, response, |r| {
        println!("ok={} archived_at={:?}", r.ok, r.archived_at)
    })
}

fn emit_deleted(json: bool, id: &Uuid) -> Result<()> {
    if json {
        println!("{}", serde_json::json!({ "ok": true, "id": id }));
    } else {
        println!("deleted {id}");
    }
    Ok(())
}
