//! Deserialization tests against representative API payloads.
//!
//! These fixtures mirror the wire format (including fields where the running
//! service differs from the OpenAPI schema, such as `assets_delivered` being an
//! integer and `dismissed_suggestions` being an array).

use gametorch::models::*;
use gametorch::types::JobCreated;

#[test]
fn reservation_uses_boolean_created() {
    let json = r##"{
        "id": "b58cc74c-ea81-4a9b-b418-d785192011a4",
        "status": "queued",
        "created": true,
        "reserved_credits": "300.000000000000"
    }"##;
    let reservation: JobCreated = serde_json::from_str(json).unwrap();
    assert!(reservation.created);
    assert_eq!(reservation.reserved_credits.to_string(), "300.000000000000");
}

#[test]
fn sprite_generation_tolerates_int_assets_and_array_suggestions() {
    let json = r##"{
        "id": "b58cc74c-ea81-4a9b-b418-d785192011a4",
        "project_id": "afb90af1-80a4-4c75-9328-2de4edde4b16",
        "prompt": "fauna you'd find in a Western video game",
        "mode": "multiple",
        "image_model": "black-forest-labs/flux-3-image",
        "text_model": "openai/gpt-6-luna",
        "quality": null,
        "resolution": "1K",
        "base_asset_id": null,
        "status": "succeeded",
        "credits_consumed": "5.003952358800",
        "reserved_credits": "0.000000000000",
        "assets_delivered": 4,
        "archived_assets": 0,
        "error": null,
        "label_suggestions": ["enemy", "fauna"],
        "art_style_suggestion": null,
        "created_at": "2026-10-01T21:10:48.237007+00:00",
        "completed_at": "2026-10-01T21:11:22.052778+00:00",
        "archived_at": null,
        "user_id": "user_3JvOvirRbpZA5wZsIh3wgoRqYCQ",
        "source": "API key",
        "api_key_id": "93d6fe0b-b513-45cc-940c-2e8dc168f130",
        "key_name": "ci",
        "assets": [
            {
                "id": "857f82f2-f5a7-445f-a97d-a30097434324",
                "name": null,
                "width": 473,
                "height": 342,
                "archived_at": null,
                "has_original": true,
                "metadata": {},
                "labels": ["fauna"],
                "dismissed_suggestions": [],
                "generation_id": null,
                "project_id": null,
                "created_at": null,
                "prompt": null,
                "user_id": "user_abc",
                "source": "API key",
                "api_key_id": "93d6fe0b-b513-45cc-940c-2e8dc168f130",
                "key_name": null
            }
        ]
    }"##;
    let generation: Generation = serde_json::from_str(json).unwrap();
    assert_eq!(generation.assets_delivered, 4);
    assert_eq!(generation.label_suggestions.len(), 2);
    assert_eq!(generation.assets.len(), 1);
    assert_eq!(generation.assets[0].width, 473);
    assert!(generation.assets[0].has_original);

    // Provenance is flattened onto the resource.
    assert_eq!(generation.provenance.source.as_deref(), Some("API key"));
    assert_eq!(
        generation.provenance.user_id.as_deref(),
        Some("user_3JvOvirRbpZA5wZsIh3wgoRqYCQ")
    );
    assert!(generation.provenance.api_key_id.is_some());
    assert_eq!(
        generation.assets[0].provenance.user_id.as_deref(),
        Some("user_abc")
    );
}

#[test]
fn sprite_catalog_decodes() {
    let json = r##"{
        "default_text_model": "openai/gpt-6-luna",
        "empirical_evidence": "v2/notes/sprite-provider-costs.md",
        "image_models": [
            {
                "id": "openai/gpt-image-2.5-flare",
                "name": "GPT Image 2.5 Flare",
                "blurb": "Best suited for base generation.",
                "available": true,
                "unavailable_reason": null,
                "qualities": ["auto", "low", "medium", "high"],
                "resolutions": [],
                "native_transparency": true,
                "editing": false,
                "transparency_status": "empirically_verified",
                "default_quality": "medium",
                "default_resolution": null,
                "default_canvas_size": "1024x1024",
                "capabilities_url": "https://openrouter.ai/api/v1/images/models/openai/gpt-image-2.5-flare/endpoints",
                "reservation_credits": "5.000000000000"
            }
        ],
        "modes": ["single", "multiple"],
        "multiple_layout": {"rows": 2, "columns": 2, "images_per_request": 1},
        "reservation_credits": 300,
        "resolution_note": "Resolution describes the entire canvas.",
        "resolution_scope": "whole_canvas",
        "text_models": [{"id": "none", "name": "No prompt enhancement"}],
        "verified_at": "2026-09-27"
    }"##;
    let catalog: SpriteModels = serde_json::from_str(json).unwrap();
    assert_eq!(catalog.image_models.len(), 1);
    assert_eq!(catalog.reservation_credits.unwrap().to_string(), "300");
    assert_eq!(catalog.multiple_layout.columns, 2);
}

#[test]
fn animation_run_decodes() {
    let json = r##"{
        "id": "29dc50d6-f8da-44c7-afa2-c2f5dcbd3cbb",
        "project_id": "afb90af1-80a4-4c75-9328-2de4edde4b16",
        "prompt": "show him draw his pistol and aim to the right",
        "animation_model": "ash",
        "duration": 4,
        "animation": {
            "id": "23d05f16-46f0-4672-8562-a9b90a9a46fb",
            "duration_seconds": 4,
            "resolution": "720p",
            "created_at": "2026-10-01T02:06:52.153372+00:00"
        },
        "status": "succeeded",
        "credits_consumed": "73.347960000000",
        "reserved_credits": "0.000000000000",
        "assets_delivered": 1,
        "base_asset_id": "6021352a-ab35-4a17-960b-71d30adf53a0",
        "error": null,
        "created_at": "2026-10-01T02:04:15.744412+00:00",
        "completed_at": "2026-10-01T02:06:52.157269+00:00",
        "archived_at": null,
        "frame_runs": [
            {
                "id": "3241879e-833f-4dee-b6a6-a19661102893",
                "status": "succeeded",
                "fps": 12,
                "frame_count": 49,
                "credits_consumed": "3.282263405160",
                "reserved_credits": "0.000000000000",
                "error": null,
                "created_at": "2026-10-01T02:06:52.155037+00:00"
            }
        ],
        "frames": [
            {
                "id": "a2efe498-fa0b-4a65-a2a1-d24edd554954",
                "frame_number": 1,
                "generation_id": "3241879e-833f-4dee-b6a6-a19661102893",
                "created_at": "2026-10-01T02:06:55.498587+00:00"
            }
        ]
    }"##;
    let run: AnimationRun = serde_json::from_str(json).unwrap();
    assert_eq!(run.animation_model.as_deref(), Some("ash"));
    assert_eq!(run.animation.as_ref().unwrap().resolution, "720p");
    assert_eq!(run.frame_runs[0].fps, 12);
    assert_eq!(run.frames[0].frame_number, 1);
}

#[test]
fn usage_histogram_decodes_sources() {
    let json = r##"{
        "range": "24h",
        "width_seconds": 3600,
        "sources": [{"id": "api_key:abc", "label": "My Key", "kind": "api_key"}],
        "buckets": [
            {
                "start": "2026-10-04T01:00:00Z",
                "end": "2026-10-04T02:00:00Z",
                "total": "0",
                "values": {"api_key:abc": "0"}
            }
        ]
    }"##;
    let histogram: UsageHistogram = serde_json::from_str(json).unwrap();
    assert_eq!(histogram.sources[0].id, "api_key:abc");
    assert_eq!(histogram.buckets[0].total.to_string(), "0");
    assert_eq!(histogram.buckets[0].values["api_key:abc"].to_string(), "0");
}

#[test]
fn export_plan_decodes() {
    let json = r##"{
        "start_frame": 1,
        "end_frame": 4,
        "reference": {"bounds": [2, 2, 433, 974], "image_width": 437, "image_height": 978},
        "first_frame": {"frame_number": 1, "image_width": 720, "image_height": 1280, "bounds": [115, 89, 490, 1102]},
        "frames": [
            {
                "frame_number": 1,
                "image_width": 720,
                "image_height": 1280,
                "bounds": [115, 89, 490, 1102],
                "offset_x": 0,
                "offset_y": 1,
                "scaled_width": 433,
                "scaled_height": 974
            }
        ],
        "max_width": 490,
        "max_height": 1104,
        "scale": 0.8838475499092558,
        "scale_width": 0.8836734693877552,
        "scale_height": 0.8838475499092558,
        "canvas_width": 433,
        "canvas_height": 976,
        "frame_count": 4
    }"##;
    let plan: ExportPlan = serde_json::from_str(json).unwrap();
    assert_eq!(plan.frame_count, 4);
    assert_eq!(plan.first_frame.as_ref().unwrap().frame_number, 1);
    assert_eq!(plan.frames[0].scaled_width, Some(433));
}

#[test]
fn label_items_decodes() {
    let json = r##"{
        "label": {
            "id": "7fb262e8-6697-4d3f-b055-073874ce786e",
            "name": "enemy",
            "color": "#e57b7b",
            "thumbnail_asset_id": "ebe4fcb6-abaf-4a9e-84d6-b460d87e10a6",
            "created_at": "2026-09-28T21:07:15.245443Z"
        },
        "assets": [
            {
                "id": "7cf2e92f-5fe8-420e-8684-75017f01f766",
                "width": 955,
                "height": 1001,
                "archived_at": null,
                "name": null,
                "has_original": true,
                "metadata": {},
                "generation_id": "c4edd487-1b17-4d9a-b9cd-c6cfed1cfe9d",
                "prompt": "make its eyes solid black",
                "created_at": "2026-10-01T18:11:15.153977+00:00",
                "labels": ["enemy", "fauna"]
            }
        ],
        "sounds": [],
        "saved_animations": []
    }"##;
    let items: LabelItems = serde_json::from_str(json).unwrap();
    assert_eq!(items.label.name, "enemy");
    assert_eq!(items.assets[0].width, 955);
}

#[test]
fn archive_response_decodes() {
    let json = r##"{"ok": true, "archived_at": "2026-09-29T19:49:33.368293+00:00"}"##;
    let response: gametorch::ArchiveResponse = serde_json::from_str(json).unwrap();
    assert!(response.ok);
    assert!(response.archived_at.is_some());
}

#[test]
fn label_association_decodes() {
    let json = r##"{"ok": true, "labels": ["character", "cowboy"]}"##;
    let association: LabelAssociation = serde_json::from_str(json).unwrap();
    assert_eq!(association.labels, vec!["character", "cowboy"]);
}

#[test]
fn api_key_scopes_decode() {
    let read = r##"{
        "id": "11111111-1111-1111-1111-111111111111",
        "name": "Read-only CI key",
        "key_prefix": "gt2_abc",
        "expires_at": null,
        "max_spend_limit": "0",
        "spend_reset_cadence": "monthly",
        "key_scope": "project_read",
        "project_id": "22222222-2222-2222-2222-222222222222",
        "spend": "0.000000000000",
        "lifetime_spend": "0.000000000000",
        "spend_reset_at": null,
        "created_at": "2026-10-05T00:00:00Z"
    }"##;
    let key: ApiKey = serde_json::from_str(read).unwrap();
    assert_eq!(key.key_scope, gametorch::ApiKeyScope::ProjectRead);
    assert_eq!(
        key.project_id,
        Some(uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap())
    );

    let write = r##"{
        "id": "33333333-3333-3333-3333-333333333333",
        "name": "Write CI key",
        "key_prefix": "gt2_def",
        "expires_at": null,
        "max_spend_limit": "500",
        "spend_reset_cadence": "weekly",
        "key_scope": "project_write",
        "project_id": "22222222-2222-2222-2222-222222222222",
        "spend": "0.000000000000",
        "lifetime_spend": "0.000000000000",
        "spend_reset_at": null,
        "created_at": "2026-10-05T00:00:00Z"
    }"##;
    let key: ApiKey = serde_json::from_str(write).unwrap();
    assert_eq!(key.key_scope, gametorch::ApiKeyScope::ProjectWrite);

    let admin = r##"{
        "id": "44444444-4444-4444-4444-444444444444",
        "key_prefix": "gt2_ghi",
        "spend_reset_cadence": "never",
        "key_scope": "admin",
        "project_id": null,
        "spend": "0",
        "lifetime_spend": "0",
        "created_at": "2026-10-05T00:00:00Z"
    }"##;
    let key: ApiKey = serde_json::from_str(admin).unwrap();
    assert_eq!(key.key_scope, gametorch::ApiKeyScope::Admin);
    assert_eq!(key.project_id, None);
}
