use event_forecast::{
    build_action_report, default_fields, normalize_events, quiet_default_profile, RawEvent,
    LocationResolver, DEFAULT_BUCKET_FIELD, DEFAULT_WINDOW_MINUTES,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn action_report_fixture_writes_decision_surface_artifact() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/fixtures/sample-stream.json");
    let raw: Vec<RawEvent> =
        serde_json::from_str(&fs::read_to_string(&fixture).expect("read sample stream")).unwrap();
    let events = normalize_events(raw).expect("normalize sample stream");
    let report = build_action_report(
        &events,
        6,
        0.6,
        &default_fields(),
        DEFAULT_BUCKET_FIELD,
        DEFAULT_WINDOW_MINUTES,
        &LocationResolver::default_demo(),
        &quiet_default_profile(),
    )
    .expect("build action report");

    let artifact_dir = manifest_dir.join("artifacts");
    fs::create_dir_all(&artifact_dir).expect("create artifacts dir");
    let artifact_path = artifact_dir.join("action-report-sample.json");
    fs::write(
        &artifact_path,
        serde_json::to_string_pretty(&report).expect("serialize action report"),
    )
    .expect("write action report artifact");

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&artifact_path).expect("read artifact"))
            .expect("parse artifact");

    assert!(json["decision"]["narrative"].is_string());
    assert!(json["decision"]["forecast_quality"]["event_type_accuracy"].is_number());
    assert!(json["heatmap"]["rows"].is_array());
    assert!(json["heatmap"]["locations"].is_object());
    assert!(json["alerts"]["alerts"].is_array());
    assert!(json["predictions"].is_array());
    assert_eq!(json["predictions"].as_array().unwrap().len(), 6);
}
