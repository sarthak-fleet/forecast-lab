use event_forecast::{build_heatmap, default_fields, fit_model, normalize_events, predict_next_stream, LocationResolver, RawEvent};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[test]
fn heatmap_uses_injected_catalog_for_arbitrary_stream() {
    let events = normalize_events(vec![
        serde_json::from_value(json!({
            "id": "e1",
            "ts": "2026-06-03T09:00:00Z",
            "event_type": "order_created",
            "properties": { "location": "pier-39" }
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "id": "e2",
            "ts": "2026-06-03T09:05:00Z",
            "event_type": "order_created",
            "properties": { "location": "pier-39" }
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "id": "e3",
            "ts": "2026-06-03T09:10:00Z",
            "event_type": "driver_assigned",
            "properties": { "location": "union-square" }
        }))
        .unwrap(),
    ])
    .unwrap();

    let mut catalog = std::collections::HashMap::new();
    catalog.insert(
        "pier-39".to_string(),
        event_forecast::LocationCoords {
            lat: 37.8087,
            lng: -122.4098,
        },
    );
    catalog.insert(
        "union-square".to_string(),
        event_forecast::LocationCoords {
            lat: 37.7879,
            lng: -122.4075,
        },
    );
    let resolver = LocationResolver::new(catalog);
    let model = fit_model(events.clone(), &default_fields()).unwrap();
    let predictions = predict_next_stream(&model, &events, 2);
    let heatmap = build_heatmap(&events, &predictions, "location", 30, &resolver);

    assert!(heatmap.locations["pier-39"].lat.is_some());
    assert_eq!(heatmap.locations["pier-39"].source, "catalog");
    assert!(heatmap.locations.contains_key("union-square"));
}

#[test]
fn coordinate_stream_renders_without_catalog() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/fixtures/coordinate-stream.json");
    let raw: Vec<RawEvent> =
        serde_json::from_str(&fs::read_to_string(fixture).expect("read coordinate stream")).unwrap();
    let events = normalize_events(raw).unwrap();
    let resolver = LocationResolver::new(std::collections::HashMap::new());
    let model = fit_model(events.clone(), &default_fields()).unwrap();
    let predictions = predict_next_stream(&model, &events, 2);
    let heatmap = build_heatmap(&events, &predictions, "location", 30, &resolver);

    assert!(heatmap
        .locations
        .values()
        .any(|meta| meta.source == "coordinates"));
}
