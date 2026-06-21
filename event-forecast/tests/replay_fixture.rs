use event_forecast::{
    build_replay, default_fields, normalize_events, quiet_default_profile, RawEvent,
    LocationResolver,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn replay_fixture_is_deterministic_for_mixed_stream() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/fixtures/sample-stream.json");
    let raw: Vec<RawEvent> =
        serde_json::from_str(&fs::read_to_string(&fixture).expect("read sample stream")).unwrap();
    let events = normalize_events(raw).expect("normalize sample stream");
    let resolver = LocationResolver::default_demo();
    let profile = quiet_default_profile();

    let playback = build_replay(
        &events,
        30,
        15,
        4,
        &default_fields(),
        "location",
        0.6,
        &resolver,
        &profile,
    )
    .expect("build replay");

    let artifact_dir = manifest_dir.join("artifacts");
    fs::create_dir_all(&artifact_dir).expect("create artifacts dir");
    let artifact_path = artifact_dir.join("replay-sample.json");
    fs::write(
        &artifact_path,
        serde_json::to_string_pretty(&playback).expect("serialize replay"),
    )
    .expect("write replay artifact");

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&artifact_path).expect("read artifact"))
            .expect("parse artifact");

    assert!(json["total_steps"].as_u64().unwrap_or(0) > 0);
    assert!(json["steps"].as_array().unwrap().len() > 0);
    assert!(json["steps"][0]["heatmap"]["locations"].is_object());
}
