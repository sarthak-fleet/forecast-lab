use std::process::Command;

#[test]
fn evaluate_cli_writes_fixture_metrics_json() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest_dir}/tests/fixtures/sample-stream.json");
    let output = std::env::temp_dir().join("event-forecast-eval-metrics.json");

    let status = Command::new(env!("CARGO_BIN_EXE_evaluate"))
        .arg(&fixture)
        .arg("--history-ratio")
        .arg("0.6")
        .arg("--output")
        .arg(&output)
        .status()
        .expect("run evaluate binary");

    assert!(status.success(), "evaluate binary failed");

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output).expect("read metrics file"))
            .expect("parse metrics JSON");

    assert!(json["event_type_accuracy"].is_number());
    assert!(json["timestamp_error"]["mean_ms"].is_number());
    assert!(json["timestamp_error"]["median_ms"].is_number());
    assert!(json["uncertainty"]["mean_event_confidence"].is_number());
    assert!(json["uncertainty"]["mean_property_confidence"].is_number());
    assert!(json["property_accuracy"]["location"]["accuracy"].is_number());
    assert!(json["property_accuracy"]["service_type"]["accuracy"].is_number());
    assert!(json["property_accuracy"]["product_type"]["accuracy"].is_number());
    assert!(json["per_step"].is_array());
}
