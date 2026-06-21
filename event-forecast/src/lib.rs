pub mod alerts;
pub mod location;
pub mod replay;

pub use alerts::{evaluate_alerts, quiet_default_profile, Alert, AlertKind, AlertProfile, AlertSeverity, AlertSummary};
pub use location::{
    demo_catalog, parse_location_catalog, BucketLocationMeta, LocationCoords, LocationResolver,
};
pub use replay::{build_replay, DriftMarker, ReplayPlayback, ReplayStep};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

pub const DEFAULT_PROPERTY_FIELDS: [&str; 3] = ["location", "service_type", "product_type"];
const COORDINATE_PROPERTY_FIELDS: [&str; 4] = ["lat", "lng", "latitude", "longitude"];

#[derive(Debug, Error)]
pub enum ForecastError {
    #[error("event must include event_type")]
    MissingEventType,
    #[error("event must include ts")]
    MissingTimestamp,
    #[error("at least two events are required to fit a model")]
    NotEnoughEvents,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawEvent {
    pub id: Option<String>,
    pub ts: Option<DateTime<Utc>>,
    pub timestamp: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub event_type: Option<String>,
    #[serde(rename = "type")]
    pub type_alias: Option<String>,
    pub entity_id: Option<String>,
    #[serde(default)]
    pub stream_id: Option<String>,
    #[serde(default)]
    pub properties: Map<String, Value>,
    #[serde(flatten)]
    pub top_level: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<String>,
    pub ts: DateTime<Utc>,
    pub event_type: String,
    pub entity_id: Option<String>,
    pub stream_id: Option<String>,
    pub properties: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PropertyPrediction {
    pub value: String,
    pub confidence: f64,
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PredictionReason {
    pub event_type: String,
    pub interval_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventPrediction {
    pub step: usize,
    pub event_type: String,
    pub expected_ts: DateTime<Utc>,
    pub confidence: f64,
    pub properties: HashMap<String, PropertyPrediction>,
    pub why: PredictionReason,
}

#[derive(Debug, Clone, Serialize)]
pub struct PredictionSummary {
    pub step: usize,
    pub event_type: String,
    pub expected_ts: DateTime<Utc>,
    pub confidence: f64,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ForecastModel {
    fields: Vec<String>,
    events: Vec<Event>,
    event_counts: HashMap<String, f64>,
    event_transitions: HashMap<String, HashMap<String, f64>>,
    intervals_by_event_type: HashMap<String, Vec<i64>>,
    global_interval_ms: i64,
    property_counts: HashMap<String, HashMap<String, f64>>,
    property_counts_by_event_type: HashMap<String, HashMap<String, HashMap<String, f64>>>,
    property_counts_by_event_transition: HashMap<String, HashMap<String, HashMap<String, f64>>>,
    property_transitions: HashMap<String, HashMap<String, HashMap<String, f64>>>,
}

pub fn normalize_event(raw: RawEvent) -> Result<Event, ForecastError> {
    let event_type = raw
        .event_type
        .or(raw.type_alias)
        .filter(|value| !value.trim().is_empty())
        .ok_or(ForecastError::MissingEventType)?;
    let ts = raw
        .ts
        .or(raw.timestamp)
        .or(raw.created_at)
        .ok_or(ForecastError::MissingTimestamp)?;
    let mut properties = raw.properties;

    for field in DEFAULT_PROPERTY_FIELDS {
        if !properties.contains_key(field) {
            if let Some(value) = raw.top_level.get(field) {
                properties.insert(field.to_string(), value.clone());
            }
        }
    }
    for field in COORDINATE_PROPERTY_FIELDS {
        if !properties.contains_key(field) {
            if let Some(value) = raw.top_level.get(field) {
                properties.insert(field.to_string(), value.clone());
            }
        }
    }

    Ok(Event {
        id: raw.id,
        ts,
        event_type,
        entity_id: raw.entity_id,
        stream_id: raw.stream_id,
        properties,
    })
}

pub fn normalize_events(raw_events: Vec<RawEvent>) -> Result<Vec<Event>, ForecastError> {
    raw_events.into_iter().map(normalize_event).collect()
}

pub fn fit_model(events: Vec<Event>, fields: &[String]) -> Result<ForecastModel, ForecastError> {
    if events.len() < 2 {
        return Err(ForecastError::NotEnoughEvents);
    }

    let mut sorted = events;
    sorted.sort_by_key(|event| event.ts);

    let mut event_counts = HashMap::new();
    let mut event_transitions = HashMap::new();
    let mut intervals_by_event_type = HashMap::new();
    let mut global_intervals = Vec::new();
    let mut property_counts = HashMap::new();
    let mut property_counts_by_event_type = HashMap::new();
    let mut property_counts_by_event_transition = HashMap::new();
    let mut property_transitions = HashMap::new();

    for field in fields {
        property_counts.insert(field.clone(), HashMap::new());
        property_counts_by_event_type.insert(field.clone(), HashMap::new());
        property_counts_by_event_transition.insert(field.clone(), HashMap::new());
        property_transitions.insert(field.clone(), HashMap::new());
    }

    for event in &sorted {
        increment(&mut event_counts, &event.event_type, 1.0);
        for field in fields {
            if let Some(value) = property_string(event, field) {
                increment(
                    property_counts.get_mut(field).expect("field initialized"),
                    &value,
                    1.0,
                );
                nested_increment(
                    property_counts_by_event_type
                        .get_mut(field)
                        .expect("field initialized"),
                    &event.event_type,
                    &value,
                    1.0,
                );
            }
        }
    }

    for pair in sorted.windows(2) {
        let current = &pair[0];
        let next = &pair[1];
        let transition_key = format!("{}->{}", current.event_type, next.event_type);
        nested_increment(
            &mut event_transitions,
            &current.event_type,
            &next.event_type,
            1.0,
        );

        let interval = (next.ts - current.ts).num_milliseconds().max(1);
        global_intervals.push(interval);
        intervals_by_event_type
            .entry(current.event_type.clone())
            .or_insert_with(Vec::new)
            .push(interval);

        for field in fields {
            if let (Some(current_value), Some(next_value)) = (
                property_string(current, field),
                property_string(next, field),
            ) {
                nested_increment(
                    property_transitions
                        .get_mut(field)
                        .expect("field initialized"),
                    &current_value,
                    &next_value,
                    1.0,
                );
                nested_increment(
                    property_counts_by_event_transition
                        .get_mut(field)
                        .expect("field initialized"),
                    &transition_key,
                    &next_value,
                    1.0,
                );
            }
        }
    }

    Ok(ForecastModel {
        fields: fields.to_vec(),
        events: sorted,
        event_counts,
        event_transitions,
        intervals_by_event_type,
        global_interval_ms: median(&global_intervals, 60_000),
        property_counts,
        property_counts_by_event_type,
        property_counts_by_event_transition,
        property_transitions,
    })
}

pub fn predict_next_stream(
    model: &ForecastModel,
    seed_events: &[Event],
    horizon: usize,
) -> Vec<EventPrediction> {
    let mut previous = seed_events
        .iter()
        .max_by_key(|event| event.ts)
        .cloned()
        .or_else(|| model.events.last().cloned())
        .expect("model has events");
    let mut predictions = Vec::with_capacity(horizon);

    for step in 1..=horizon {
        let event_choice = predict_event_type(model, &previous.event_type);
        let interval_ms = median(
            model
                .intervals_by_event_type
                .get(&previous.event_type)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            model.global_interval_ms,
        );
        let mut properties = HashMap::new();

        for field in &model.fields {
            if let Some(prediction) = predict_property(
                model,
                field,
                &previous.event_type,
                property_string(&previous, field).as_deref(),
                &event_choice.value,
            ) {
                properties.insert(field.clone(), prediction);
            }
        }

        let expected_ts = previous.ts + Duration::milliseconds(interval_ms);
        predictions.push(EventPrediction {
            step,
            event_type: event_choice.value.clone(),
            expected_ts,
            confidence: round3(event_choice.confidence),
            properties: properties.clone(),
            why: PredictionReason {
                event_type: event_choice.reason,
                interval_ms,
            },
        });

        previous = Event {
            id: None,
            ts: expected_ts,
            event_type: event_choice.value,
            entity_id: None,
            stream_id: previous.stream_id.clone(),
            properties: properties
                .into_iter()
                .map(|(key, value)| (key, Value::String(value.value)))
                .collect(),
        };
    }

    predictions
}

#[derive(Debug, Clone, Serialize)]
pub struct PropertyShare {
    pub value: String,
    pub share: f64,
    pub mean_confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventTypeShare {
    pub event_type: String,
    pub share: f64,
    pub mean_confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LowConfidenceStep {
    pub step: usize,
    pub event_type: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastReport {
    pub horizon: usize,
    pub headline: String,
    pub next_likely_events: Vec<EventTypeShare>,
    pub property_mix: HashMap<String, Vec<PropertyShare>>,
    pub low_confidence_steps: Vec<LowConfidenceStep>,
    pub mean_event_confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepScore {
    pub step: usize,
    pub predicted_event_type: String,
    pub actual_event_type: String,
    pub event_type_correct: bool,
    pub property_correct: HashMap<String, bool>,
    pub timestamp_error_ms: i64,
    pub event_confidence: f64,
    pub property_confidence: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PropertyAccuracy {
    pub correct: usize,
    pub scored: usize,
    pub accuracy: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimestampError {
    pub mean_ms: f64,
    pub median_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UncertaintySummary {
    pub mean_event_confidence: f64,
    pub mean_property_confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationResult {
    pub history_count: usize,
    pub future_count: usize,
    pub event_type_accuracy: f64,
    pub property_accuracy: HashMap<String, PropertyAccuracy>,
    pub timestamp_error: TimestampError,
    pub uncertainty: UncertaintySummary,
    pub per_step: Vec<StepScore>,
}

const LOW_CONFIDENCE_THRESHOLD: f64 = 0.5;
const REPORT_TOP_N: usize = 3;
pub const DEFAULT_BUCKET_FIELD: &str = "location";
pub const DEFAULT_WINDOW_MINUTES: i64 = 30;

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapRow {
    pub bucket: String,
    pub window_start: DateTime<Utc>,
    pub observed_count: usize,
    pub predicted_count: usize,
    pub delta: i64,
    pub mean_confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapBucketSummary {
    pub bucket: String,
    pub observed_total: usize,
    pub predicted_total: usize,
    pub delta: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Heatmap {
    pub bucket_field: String,
    pub window_minutes: i64,
    pub horizon: usize,
    pub rows: Vec<HeatmapRow>,
    pub hot_buckets: Vec<HeatmapBucketSummary>,
    pub cooling_buckets: Vec<HeatmapBucketSummary>,
    pub locations: BTreeMap<String, BucketLocationMeta>,
}

pub fn build_heatmap(
    events: &[Event],
    predictions: &[EventPrediction],
    bucket_field: &str,
    window_minutes: i64,
    resolver: &LocationResolver,
) -> Heatmap {
    let window_minutes = window_minutes.max(1);
    let mut observed: HashMap<(String, DateTime<Utc>), usize> = HashMap::new();
    let mut predicted: HashMap<(String, DateTime<Utc>), (usize, f64)> = HashMap::new();

    for event in events {
        if let Some(bucket) = property_string(event, bucket_field) {
            let window = floor_to_window(event.ts, window_minutes);
            *observed.entry((bucket, window)).or_insert(0) += 1;
        }
    }

    for prediction in predictions {
        if let Some(bucket) = prediction
            .properties
            .get(bucket_field)
            .map(|value| value.value.clone())
        {
            let window = floor_to_window(prediction.expected_ts, window_minutes);
            let entry = predicted.entry((bucket, window)).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += prediction.confidence;
        }
    }

    let mut keys: Vec<(String, DateTime<Utc>)> = observed.keys().cloned().collect();
    for key in predicted.keys() {
        if !keys.contains(key) {
            keys.push(key.clone());
        }
    }
    keys.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let mut rows = Vec::with_capacity(keys.len());
    let mut totals: HashMap<String, (usize, usize)> = HashMap::new();
    for key in &keys {
        let observed_count = observed.get(key).copied().unwrap_or(0);
        let predicted_entry = predicted.get(key).copied().unwrap_or((0, 0.0));
        let predicted_count = predicted_entry.0;
        let mean_confidence = if predicted_count == 0 {
            0.0
        } else {
            round3(predicted_entry.1 / predicted_count as f64)
        };
        rows.push(HeatmapRow {
            bucket: key.0.clone(),
            window_start: key.1,
            observed_count,
            predicted_count,
            delta: predicted_count as i64 - observed_count as i64,
            mean_confidence,
        });
        let entry = totals.entry(key.0.clone()).or_insert((0, 0));
        entry.0 += observed_count;
        entry.1 += predicted_count;
    }

    let mut bucket_summaries: Vec<HeatmapBucketSummary> = totals
        .into_iter()
        .map(
            |(bucket, (observed_total, predicted_total))| HeatmapBucketSummary {
                bucket,
                observed_total,
                predicted_total,
                delta: predicted_total as i64 - observed_total as i64,
            },
        )
        .collect();
    bucket_summaries.sort_by(|a, b| b.delta.cmp(&a.delta).then_with(|| a.bucket.cmp(&b.bucket)));

    let hot_buckets: Vec<HeatmapBucketSummary> = bucket_summaries
        .iter()
        .filter(|summary| summary.delta > 0)
        .take(REPORT_TOP_N)
        .cloned()
        .collect();
    let cooling_buckets: Vec<HeatmapBucketSummary> = bucket_summaries
        .iter()
        .rev()
        .filter(|summary| summary.delta < 0)
        .take(REPORT_TOP_N)
        .cloned()
        .collect();

    let mut bucket_values: Vec<String> = rows.iter().map(|row| row.bucket.clone()).collect();
    for summary in hot_buckets.iter().chain(cooling_buckets.iter()) {
        if !bucket_values.contains(&summary.bucket) {
            bucket_values.push(summary.bucket.clone());
        }
    }
    let locations = resolver.collect_bucket_locations(events, &bucket_values, bucket_field);

    Heatmap {
        bucket_field: bucket_field.to_string(),
        window_minutes,
        horizon: predictions.len(),
        rows,
        hot_buckets,
        cooling_buckets,
        locations,
    }
}

pub fn fit_per_entity_models(
    events: Vec<Event>,
    fields: &[String],
) -> HashMap<String, ForecastModel> {
    let mut by_entity: HashMap<String, Vec<Event>> = HashMap::new();
    for event in events {
        let key = event
            .entity_id
            .clone()
            .unwrap_or_else(|| "_unassigned".to_string());
        by_entity.entry(key).or_default().push(event);
    }
    let mut models = HashMap::new();
    for (entity, group) in by_entity {
        if let Ok(model) = fit_model(group, fields) {
            models.insert(entity, model);
        }
    }
    models
}

#[derive(Debug, Clone, Serialize)]
pub struct AnomalyFlag {
    pub event_id: Option<String>,
    pub ts: DateTime<Utc>,
    pub event_type: String,
    pub interval_ms: i64,
    pub expected_interval_ms: i64,
    pub z_score: f64,
}

pub fn detect_anomalies(events: &[Event], z_threshold: f64) -> Vec<AnomalyFlag> {
    let mut sorted = events.to_vec();
    sorted.sort_by_key(|event| event.ts);
    let intervals: Vec<i64> = sorted
        .windows(2)
        .map(|pair| (pair[1].ts - pair[0].ts).num_milliseconds().max(0))
        .collect();
    if intervals.len() < 2 {
        return Vec::new();
    }
    let mean = intervals.iter().copied().sum::<i64>() as f64 / intervals.len() as f64;
    let variance = intervals
        .iter()
        .map(|value| {
            let diff = *value as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / intervals.len() as f64;
    let std_dev = variance.sqrt();
    if std_dev == 0.0 {
        return Vec::new();
    }
    let mut flagged = Vec::new();
    for (idx, interval) in intervals.iter().enumerate() {
        let z = (*interval as f64 - mean) / std_dev;
        if z.abs() >= z_threshold {
            let event = &sorted[idx + 1];
            flagged.push(AnomalyFlag {
                event_id: event.id.clone(),
                ts: event.ts,
                event_type: event.event_type.clone(),
                interval_ms: *interval,
                expected_interval_ms: mean.round() as i64,
                z_score: round3(z),
            });
        }
    }
    flagged
}

#[derive(Debug, Clone, Serialize)]
pub struct NumericForecast {
    pub step: usize,
    pub event_type: String,
    pub field: String,
    pub expected: f64,
    pub sample_size: usize,
}

pub fn forecast_numeric_fields(
    events: &[Event],
    predictions: &[EventPrediction],
    numeric_fields: &[String],
) -> Vec<NumericForecast> {
    let mut samples: HashMap<(String, String), Vec<f64>> = HashMap::new();
    for event in events {
        for field in numeric_fields {
            if let Some(value) = event
                .properties
                .get(field)
                .and_then(|raw| numeric_value(raw))
            {
                samples
                    .entry((event.event_type.clone(), field.clone()))
                    .or_default()
                    .push(value);
            }
        }
    }
    let mut forecasts = Vec::new();
    for prediction in predictions {
        for field in numeric_fields {
            if let Some(values) = samples.get(&(prediction.event_type.clone(), field.clone())) {
                if !values.is_empty() {
                    let mut sorted = values.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let median = sorted[sorted.len() / 2];
                    forecasts.push(NumericForecast {
                        step: prediction.step,
                        event_type: prediction.event_type.clone(),
                        field: field.clone(),
                        expected: round3(median),
                        sample_size: values.len(),
                    });
                }
            }
        }
    }
    forecasts
}

fn numeric_value(raw: &Value) -> Option<f64> {
    match raw {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MixShift {
    pub field: String,
    pub value: String,
    pub previous_share: f64,
    pub current_share: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowStats {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionReport {
    pub horizon: usize,
    pub narrative: String,
    pub previous_window: WindowStats,
    pub current_window: WindowStats,
    pub hot_zones: Vec<MixShift>,
    pub cooling_zones: Vec<MixShift>,
    pub mix_shifts: Vec<MixShift>,
    pub forecast_headline: String,
    pub low_confidence_steps: Vec<LowConfidenceStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast_quality: Option<ForecastQuality>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastQuality {
    pub history_count: usize,
    pub future_count: usize,
    pub history_ratio: f64,
    pub event_type_accuracy: f64,
    pub property_accuracy: HashMap<String, PropertyAccuracy>,
    pub timestamp_error: TimestampError,
    pub uncertainty: UncertaintySummary,
    pub headline: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionReport {
    pub decision: DecisionReport,
    pub heatmap: Heatmap,
    pub predictions: Vec<EventPrediction>,
    pub alerts: AlertSummary,
}

pub(crate) const MIX_SHIFT_THRESHOLD: f64 = 0.10;

pub fn build_decision_report(
    events: &[Event],
    predictions: &[EventPrediction],
    fields: &[String],
) -> Option<DecisionReport> {
    if events.len() < 2 {
        return None;
    }
    let mut sorted = events.to_vec();
    sorted.sort_by_key(|event| event.ts);
    let midpoint = sorted.len() / 2;
    let (previous, current) = sorted.split_at(midpoint);
    if previous.is_empty() || current.is_empty() {
        return None;
    }

    let previous_window = WindowStats {
        start: previous.first().map(|e| e.ts).unwrap_or_else(Utc::now),
        end: previous.last().map(|e| e.ts).unwrap_or_else(Utc::now),
        event_count: previous.len(),
    };
    let current_window = WindowStats {
        start: current.first().map(|e| e.ts).unwrap_or_else(Utc::now),
        end: current.last().map(|e| e.ts).unwrap_or_else(Utc::now),
        event_count: current.len(),
    };

    let mut mix_shifts = Vec::new();
    for field in fields {
        let prev_shares = property_shares(previous, field);
        let curr_shares = property_shares(current, field);
        let mut all_values: Vec<String> = prev_shares
            .keys()
            .chain(curr_shares.keys())
            .cloned()
            .collect();
        all_values.sort();
        all_values.dedup();
        for value in all_values {
            let previous_share = prev_shares.get(&value).copied().unwrap_or(0.0);
            let current_share = curr_shares.get(&value).copied().unwrap_or(0.0);
            let delta = round3(current_share - previous_share);
            if delta.abs() >= MIX_SHIFT_THRESHOLD {
                mix_shifts.push(MixShift {
                    field: field.clone(),
                    value,
                    previous_share: round3(previous_share),
                    current_share: round3(current_share),
                    delta,
                });
            }
        }
    }
    mix_shifts.sort_by(|a, b| {
        b.delta
            .abs()
            .partial_cmp(&a.delta.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let hot_zones: Vec<MixShift> = mix_shifts
        .iter()
        .filter(|shift| shift.field == "location" && shift.delta > 0.0)
        .take(REPORT_TOP_N)
        .cloned()
        .collect();
    let cooling_zones: Vec<MixShift> = mix_shifts
        .iter()
        .filter(|shift| shift.field == "location" && shift.delta < 0.0)
        .take(REPORT_TOP_N)
        .cloned()
        .collect();

    let report = build_report(predictions);
    let narrative = compose_narrative(
        &previous_window,
        &current_window,
        &hot_zones,
        &cooling_zones,
        &mix_shifts,
        &report,
    );

    Some(DecisionReport {
        horizon: predictions.len(),
        narrative,
        previous_window,
        current_window,
        hot_zones,
        cooling_zones,
        mix_shifts,
        forecast_headline: report.headline,
        low_confidence_steps: report.low_confidence_steps,
        forecast_quality: None,
    })
}

pub fn build_forecast_quality(result: &EvaluationResult, history_ratio: f64) -> ForecastQuality {
    ForecastQuality {
        history_count: result.history_count,
        future_count: result.future_count,
        history_ratio: round3(history_ratio),
        event_type_accuracy: result.event_type_accuracy,
        property_accuracy: result.property_accuracy.clone(),
        timestamp_error: result.timestamp_error.clone(),
        uncertainty: result.uncertainty.clone(),
        headline: forecast_quality_headline(result),
    }
}

fn forecast_quality_headline(result: &EvaluationResult) -> String {
    let location_accuracy = result
        .property_accuracy
        .get("location")
        .map(|entry| entry.accuracy)
        .unwrap_or(0.0);
    format!(
        "held-out quality: {}% event-type, {}% location, median timestamp error {}s",
        (result.event_type_accuracy * 100.0).round() as i64,
        (location_accuracy * 100.0).round() as i64,
        (result.timestamp_error.median_ms / 1000.0).round() as i64,
    )
}

pub fn attach_forecast_quality(
    report: &mut DecisionReport,
    events: &[Event],
    history_ratio: f64,
    fields: &[String],
) {
    if events.len() < 4 {
        return;
    }
    if let Ok(result) = evaluate_stream(events.to_vec(), history_ratio, fields) {
        let quality = build_forecast_quality(&result, history_ratio);
        report.narrative = format!("{}; {}", report.narrative, quality.headline);
        report.forecast_quality = Some(quality);
    }
}

pub fn build_action_report(
    events: &[Event],
    horizon: usize,
    history_ratio: f64,
    fields: &[String],
    bucket_field: &str,
    window_minutes: i64,
    location_resolver: &LocationResolver,
    alert_profile: &AlertProfile,
) -> Result<ActionReport, ForecastError> {
    let model = fit_model(events.to_vec(), fields)?;
    let predictions = predict_next_stream(&model, events, horizon);
    let forecast = build_report(&predictions);
    let mut decision = build_decision_report(events, &predictions, fields)
        .ok_or(ForecastError::NotEnoughEvents)?;
    attach_forecast_quality(&mut decision, events, history_ratio, fields);
    let heatmap = build_heatmap(events, &predictions, bucket_field, window_minutes, location_resolver);
    let anomalies = detect_anomalies(events, 2.0);
    let alerts = evaluate_alerts(
        &decision,
        &forecast,
        &heatmap,
        &anomalies,
        alert_profile,
    );
    Ok(ActionReport {
        decision,
        heatmap,
        predictions,
        alerts,
    })
}

fn property_shares(events: &[Event], field: &str) -> HashMap<String, f64> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total = 0;
    for event in events {
        if let Some(value) = property_string(event, field) {
            *counts.entry(value).or_insert(0) += 1;
            total += 1;
        }
    }
    if total == 0 {
        return HashMap::new();
    }
    counts
        .into_iter()
        .map(|(value, count)| (value, count as f64 / total as f64))
        .collect()
}

fn compose_narrative(
    previous: &WindowStats,
    current: &WindowStats,
    hot: &[MixShift],
    cooling: &[MixShift],
    shifts: &[MixShift],
    forecast: &ForecastReport,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "current window holds {} vs {} in the prior window",
        pluralize(current.event_count, "event"),
        pluralize(previous.event_count, "event")
    ));
    if let Some(hot_zone) = hot.first() {
        parts.push(format!(
            "{} is heating up (+{}%)",
            hot_zone.value,
            (hot_zone.delta * 100.0).round() as i64
        ));
    }
    if let Some(cool_zone) = cooling.first() {
        parts.push(format!(
            "{} is cooling ({}%)",
            cool_zone.value,
            (cool_zone.delta * 100.0).round() as i64
        ));
    }
    if let Some(service_shift) = shifts
        .iter()
        .find(|shift| shift.field == "service_type" && shift.delta.abs() >= MIX_SHIFT_THRESHOLD)
    {
        parts.push(format!(
            "service mix shifting toward {}",
            service_shift.value
        ));
    }
    parts.push(forecast.headline.clone());
    if !forecast.low_confidence_steps.is_empty() {
        parts.push(format!(
            "{} below 0.5 confidence",
            pluralize(forecast.low_confidence_steps.len(), "step")
        ));
    }
    parts.join("; ")
}

fn pluralize(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

fn floor_to_window(ts: DateTime<Utc>, window_minutes: i64) -> DateTime<Utc> {
    let window_seconds = window_minutes.max(1).saturating_mul(60);
    let truncated = ts.timestamp().div_euclid(window_seconds) * window_seconds;
    DateTime::<Utc>::from_timestamp(truncated, 0).unwrap_or(ts)
}

pub fn build_report(predictions: &[EventPrediction]) -> ForecastReport {
    let horizon = predictions.len();
    let mut event_counts: HashMap<String, (usize, f64)> = HashMap::new();
    let mut property_counts: HashMap<String, HashMap<String, (usize, f64)>> = HashMap::new();
    let mut low_confidence_steps = Vec::new();
    let mut confidence_sum = 0.0;

    for prediction in predictions {
        let entry = event_counts
            .entry(prediction.event_type.clone())
            .or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += prediction.confidence;
        confidence_sum += prediction.confidence;

        if prediction.confidence < LOW_CONFIDENCE_THRESHOLD {
            low_confidence_steps.push(LowConfidenceStep {
                step: prediction.step,
                event_type: prediction.event_type.clone(),
                confidence: prediction.confidence,
            });
        }

        for (field, value) in &prediction.properties {
            let field_entry = property_counts.entry(field.clone()).or_default();
            let value_entry = field_entry.entry(value.value.clone()).or_insert((0, 0.0));
            value_entry.0 += 1;
            value_entry.1 += value.confidence;
        }
    }

    let next_likely_events: Vec<EventTypeShare> = rank_shares(&event_counts, horizon)
        .into_iter()
        .take(REPORT_TOP_N)
        .map(|(value, share, mean)| EventTypeShare {
            event_type: value,
            share,
            mean_confidence: mean,
        })
        .collect();

    let mut property_mix: HashMap<String, Vec<PropertyShare>> = HashMap::new();
    for (field, counts) in property_counts {
        let total: usize = counts.values().map(|(count, _)| *count).sum();
        let shares = rank_shares(&counts, total)
            .into_iter()
            .take(REPORT_TOP_N)
            .map(|(value, share, mean)| PropertyShare {
                value,
                share,
                mean_confidence: mean,
            })
            .collect();
        property_mix.insert(field, shares);
    }

    let mean_event_confidence = if horizon == 0 {
        0.0
    } else {
        round3(confidence_sum / horizon as f64)
    };

    let headline = headline(horizon, &next_likely_events, &property_mix);

    ForecastReport {
        horizon,
        headline,
        next_likely_events,
        property_mix,
        low_confidence_steps,
        mean_event_confidence,
    }
}

pub fn evaluate_stream(
    events: Vec<Event>,
    history_ratio: f64,
    fields: &[String],
) -> Result<EvaluationResult, ForecastError> {
    let total = events.len();
    if total < 4 {
        return Err(ForecastError::NotEnoughEvents);
    }
    let ratio = history_ratio.clamp(0.1, 0.9);
    let history_count = ((total as f64) * ratio).round() as usize;
    let history_count = history_count.clamp(2, total - 1);

    let mut sorted = events;
    sorted.sort_by_key(|event| event.ts);
    let (history, future) = sorted.split_at(history_count);
    let history_vec = history.to_vec();
    let future_vec = future.to_vec();

    let model = fit_model(history_vec.clone(), fields)?;
    let predictions = predict_next_stream(&model, &history_vec, future_vec.len());

    let mut event_correct = 0;
    let mut property_totals: HashMap<String, (usize, usize)> = HashMap::new();
    let mut per_step = Vec::with_capacity(predictions.len());
    let mut timestamp_errors_ms = Vec::with_capacity(predictions.len());
    let mut event_confidence_sum = 0.0;
    let mut property_confidence_sum = 0.0;
    let mut property_confidence_count = 0usize;

    for (idx, prediction) in predictions.iter().enumerate() {
        let actual = &future_vec[idx];
        let event_type_correct = prediction.event_type == actual.event_type;
        if event_type_correct {
            event_correct += 1;
        }
        event_confidence_sum += prediction.confidence;

        let timestamp_error_ms = (prediction.expected_ts - actual.ts)
            .num_milliseconds()
            .abs();
        timestamp_errors_ms.push(timestamp_error_ms);

        let mut property_correct = HashMap::new();
        let mut property_confidence = HashMap::new();
        for field in fields {
            if let Some(actual_value) = property_string(actual, field) {
                let entry = property_totals.entry(field.clone()).or_insert((0, 0));
                entry.1 += 1;
                let matched = prediction
                    .properties
                    .get(field)
                    .map(|p| p.value == actual_value)
                    .unwrap_or(false);
                if matched {
                    entry.0 += 1;
                }
                property_correct.insert(field.clone(), matched);
                if let Some(confidence) = prediction.properties.get(field).map(|p| p.confidence) {
                    property_confidence.insert(field.clone(), confidence);
                    property_confidence_sum += confidence;
                    property_confidence_count += 1;
                }
            }
        }
        per_step.push(StepScore {
            step: prediction.step,
            predicted_event_type: prediction.event_type.clone(),
            actual_event_type: actual.event_type.clone(),
            event_type_correct,
            property_correct,
            timestamp_error_ms,
            event_confidence: prediction.confidence,
            property_confidence,
        });
    }

    let event_type_accuracy = if predictions.is_empty() {
        0.0
    } else {
        round3(event_correct as f64 / predictions.len() as f64)
    };

    let property_accuracy = property_totals
        .into_iter()
        .map(|(field, (correct, scored))| {
            let accuracy = if scored == 0 {
                0.0
            } else {
                round3(correct as f64 / scored as f64)
            };
            (
                field,
                PropertyAccuracy {
                    correct,
                    scored,
                    accuracy,
                },
            )
        })
        .collect();

    let timestamp_error = TimestampError {
        mean_ms: if timestamp_errors_ms.is_empty() {
            0.0
        } else {
            round3(
                timestamp_errors_ms.iter().map(|ms| *ms as f64).sum::<f64>()
                    / timestamp_errors_ms.len() as f64,
            )
        },
        median_ms: median_ms(&timestamp_errors_ms) as f64,
    };

    let uncertainty = UncertaintySummary {
        mean_event_confidence: if predictions.is_empty() {
            0.0
        } else {
            round3(event_confidence_sum / predictions.len() as f64)
        },
        mean_property_confidence: if property_confidence_count == 0 {
            0.0
        } else {
            round3(property_confidence_sum / property_confidence_count as f64)
        },
    };

    Ok(EvaluationResult {
        history_count: history_vec.len(),
        future_count: future_vec.len(),
        event_type_accuracy,
        property_accuracy,
        timestamp_error,
        uncertainty,
        per_step,
    })
}

fn rank_shares(counts: &HashMap<String, (usize, f64)>, total: usize) -> Vec<(String, f64, f64)> {
    let mut ranked: Vec<_> = counts
        .iter()
        .map(|(value, (count, confidence_sum))| {
            let share = if total == 0 {
                0.0
            } else {
                round3(*count as f64 / total as f64)
            };
            let mean_confidence = if *count == 0 {
                0.0
            } else {
                round3(confidence_sum / *count as f64)
            };
            (value.clone(), share, mean_confidence, *count)
        })
        .collect();
    ranked.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .map(|(value, share, mean, _)| (value, share, mean))
        .collect()
}

fn headline(
    horizon: usize,
    next_events: &[EventTypeShare],
    property_mix: &HashMap<String, Vec<PropertyShare>>,
) -> String {
    if horizon == 0 || next_events.is_empty() {
        return "no forecast available".to_string();
    }
    let lead = &next_events[0];
    let location_hint = property_mix
        .get("location")
        .and_then(|values| values.first())
        .map(|share| format!(" centered on {}", share.value))
        .unwrap_or_default();
    let service_hint = property_mix
        .get("service_type")
        .and_then(|values| values.first())
        .map(|share| format!(", {} mix", share.value))
        .unwrap_or_default();
    format!(
        "next {} steps lean toward {} ({}%){}{}",
        horizon,
        lead.event_type,
        (lead.share * 100.0).round() as i64,
        location_hint,
        service_hint,
    )
}

pub fn summarize_predictions(predictions: &[EventPrediction]) -> Vec<PredictionSummary> {
    predictions
        .iter()
        .map(|prediction| PredictionSummary {
            step: prediction.step,
            event_type: prediction.event_type.clone(),
            expected_ts: prediction.expected_ts,
            confidence: prediction.confidence,
            properties: prediction
                .properties
                .iter()
                .map(|(field, value)| (field.clone(), value.value.clone()))
                .collect(),
        })
        .collect()
}

pub fn default_fields() -> Vec<String> {
    DEFAULT_PROPERTY_FIELDS
        .iter()
        .map(|field| field.to_string())
        .collect()
}

fn increment(map: &mut HashMap<String, f64>, key: &str, amount: f64) {
    *map.entry(key.to_string()).or_insert(0.0) += amount;
}

fn nested_increment(
    map: &mut HashMap<String, HashMap<String, f64>>,
    outer_key: &str,
    inner_key: &str,
    amount: f64,
) {
    let inner = map
        .entry(outer_key.to_string())
        .or_insert_with(HashMap::new);
    increment(inner, inner_key, amount);
}

fn property_string(event: &Event, field: &str) -> Option<String> {
    match event.properties.get(field)? {
        Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn median(values: &[i64], fallback: i64) -> i64 {
    if values.is_empty() {
        return fallback;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2
    }
}

fn top_choice(counts: &HashMap<String, f64>) -> Option<Choice> {
    let total: f64 = counts.values().sum();
    if total <= 0.0 {
        return None;
    }
    let (value, count) = counts.iter().max_by(|left, right| {
        left.1
            .partial_cmp(right.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            // Deterministic tie-break: prefer the lexicographically smallest
            // value so equal counts cannot flip predictions between runs.
            .then_with(|| right.0.cmp(left.0))
    })?;
    Some(Choice {
        value: value.clone(),
        confidence: count / total,
    })
}

fn merge_weighted(maps: Vec<(Option<&HashMap<String, f64>>, f64)>) -> HashMap<String, f64> {
    let mut merged = HashMap::new();
    for (counts, weight) in maps {
        if let Some(counts) = counts {
            for (value, count) in counts {
                increment(&mut merged, value, count * weight);
            }
        }
    }
    merged
}

fn predict_event_type(model: &ForecastModel, previous_event_type: &str) -> ChoiceWithReason {
    if let Some(counts) = model.event_transitions.get(previous_event_type) {
        if let Some(choice) = top_choice(counts) {
            return ChoiceWithReason {
                value: choice.value,
                confidence: choice.confidence,
                reason: format!("transition from {previous_event_type}"),
            };
        }
    }
    let choice = top_choice(&model.event_counts).expect("model has event counts");
    ChoiceWithReason {
        value: choice.value,
        confidence: choice.confidence,
        reason: "global event frequency fallback".to_string(),
    }
}

fn predict_property(
    model: &ForecastModel,
    field: &str,
    previous_event_type: &str,
    previous_value: Option<&str>,
    predicted_event_type: &str,
) -> Option<PropertyPrediction> {
    let transition_key = format!("{previous_event_type}->{predicted_event_type}");
    let event_transition_counts = model
        .property_counts_by_event_transition
        .get(field)?
        .get(&transition_key);
    let event_type_counts = model
        .property_counts_by_event_type
        .get(field)?
        .get(predicted_event_type);
    let transition_counts =
        previous_value.and_then(|value| model.property_transitions.get(field)?.get(value));
    let global_counts = model.property_counts.get(field);
    let merged = merge_weighted(vec![
        (event_transition_counts, 0.55),
        (event_type_counts, 0.25),
        (transition_counts, 0.15),
        (global_counts, 0.05),
    ]);
    let choice = top_choice(&merged)?;

    Some(PropertyPrediction {
        value: choice.value,
        confidence: round3(choice.confidence),
        why: if event_transition_counts.is_some() {
            format!("property distribution for {transition_key}")
        } else {
            format!("distribution for {predicted_event_type}")
        },
    })
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn median_ms(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2
    } else {
        sorted[mid]
    }
}

struct Choice {
    value: String,
    confidence: f64,
}

struct ChoiceWithReason {
    value: String,
    confidence: f64,
    reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn sample_events() -> Vec<Event> {
        let raw: Vec<RawEvent> =
            serde_json::from_str(include_str!("../data/sample-events.json")).unwrap();
        normalize_events(raw).unwrap()
    }

    #[test]
    fn normalizes_top_level_property_aliases() {
        let event = normalize_event(RawEvent {
            id: None,
            ts: Some("2026-06-03T00:00:00Z".parse().unwrap()),
            timestamp: None,
            created_at: None,
            event_type: None,
            type_alias: Some("view".to_string()),
            entity_id: None,
            stream_id: None,
            properties: Map::new(),
            top_level: Map::from_iter([
                ("location".to_string(), Value::String("hsr".to_string())),
                (
                    "service_type".to_string(),
                    Value::String("booking".to_string()),
                ),
                (
                    "product_type".to_string(),
                    Value::String("taxi".to_string()),
                ),
            ]),
        })
        .unwrap();

        assert_eq!(event.event_type, "view");
        assert_eq!(event.properties["location"], "hsr");
        assert_eq!(event.properties["service_type"], "booking");
        assert_eq!(event.properties["product_type"], "taxi");
    }

    #[test]
    fn predicts_next_event_type_from_learned_transitions() {
        let events = sample_events();
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 1);

        assert_eq!(predictions[0].event_type, "order_picked_up");
        assert_eq!(
            predictions[0].why.event_type,
            "transition from driver_assigned"
        );
    }

    #[test]
    fn predicts_from_latest_seed_event_even_when_input_is_unsorted() {
        let events = sample_events();
        let mut unsorted = events.clone();
        let len = unsorted.len();
        unsorted.swap(len - 2, len - 1);
        let model = fit_model(unsorted.clone(), &default_fields()).unwrap();

        let sorted_prediction = predict_next_stream(&model, &events, 1);
        let unsorted_prediction = predict_next_stream(&model, &unsorted, 1);

        assert_eq!(
            unsorted_prediction[0].event_type,
            sorted_prediction[0].event_type
        );
        assert_eq!(
            unsorted_prediction[0].why.event_type,
            sorted_prediction[0].why.event_type
        );
    }

    #[test]
    fn predicts_configured_categorical_properties() {
        let events = sample_events();
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 1);
        let first = &predictions[0];

        assert_eq!(first.properties["location"].value, "indiranagar");
        assert_eq!(first.properties["service_type"].value, "delivery");
        assert_eq!(first.properties["product_type"].value, "food");
        assert!(first.properties["location"].confidence > 0.0);
    }

    #[test]
    fn report_aggregates_event_mix_and_locations() {
        let events = sample_events();
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 6);
        let report = build_report(&predictions);

        assert_eq!(report.horizon, 6);
        assert!(!report.next_likely_events.is_empty());
        assert!(report.property_mix.contains_key("location"));
        assert!(report.property_mix.contains_key("service_type"));
        assert!(report
            .next_likely_events
            .iter()
            .any(|share| share.event_type == "order_picked_up"));
        let location_total: f64 = report.property_mix["location"]
            .iter()
            .map(|share| share.share)
            .sum();
        assert!((location_total - 1.0).abs() < 0.05);
        assert!(report.headline.contains("steps lean toward"));
    }

    #[test]
    fn evaluation_scores_event_type_and_properties() {
        let events = sample_events();
        let result = evaluate_stream(events.clone(), 0.6, &default_fields()).unwrap();

        assert!(result.history_count >= 2);
        assert!(result.future_count >= 1);
        assert_eq!(result.history_count + result.future_count, events.len());
        assert!(result.per_step.len() == result.future_count);
        assert!(result.event_type_accuracy >= 0.0 && result.event_type_accuracy <= 1.0);
        assert!(result.property_accuracy.contains_key("location"));
        assert!(result.timestamp_error.mean_ms >= 0.0);
        assert!(result.timestamp_error.median_ms >= 0.0);
        assert!(result.uncertainty.mean_event_confidence > 0.0);
        assert!(result.uncertainty.mean_property_confidence > 0.0);
        assert!(result
            .per_step
            .iter()
            .all(|step| step.timestamp_error_ms >= 0 && step.event_confidence > 0.0));
    }

    #[test]
    fn evaluation_fixture_matches_sample_stream_shape() {
        let raw: Vec<RawEvent> =
            serde_json::from_str(include_str!("../tests/fixtures/sample-stream.json")).unwrap();
        let events = normalize_events(raw).unwrap();
        let result = evaluate_stream(events, 0.6, &default_fields()).unwrap();
        let json = serde_json::to_value(&result).unwrap();

        assert!(json.get("event_type_accuracy").is_some());
        assert!(json.get("timestamp_error").is_some());
        assert!(json.get("uncertainty").is_some());
        assert!(json["property_accuracy"].get("location").is_some());
        assert!(json["property_accuracy"].get("service_type").is_some());
        assert!(json["property_accuracy"].get("product_type").is_some());
    }

    #[test]
    fn heatmap_buckets_observed_and_predicted_by_location() {
        let events = sample_events();
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 4);
        let heatmap = build_heatmap(
            &events,
            &predictions,
            "location",
            30,
            &LocationResolver::default_demo(),
        );

        assert_eq!(heatmap.window_minutes, 30);
        assert!(!heatmap.rows.is_empty());
        assert!(heatmap
            .rows
            .iter()
            .any(|row| row.bucket == "koramangala" && row.observed_count > 0));
        assert!(heatmap.rows.iter().any(|row| row.predicted_count > 0));
        let total_observed: usize = heatmap.rows.iter().map(|row| row.observed_count).sum();
        assert_eq!(total_observed, events.len());
        assert!(heatmap.locations.contains_key("koramangala"));
        assert!(heatmap.locations["koramangala"].lat.is_some());
    }

    #[test]
    fn decision_report_compares_windows_and_flags_shifts() {
        let events = sample_events();
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 4);
        let report = build_decision_report(&events, &predictions, &default_fields()).unwrap();

        assert_eq!(
            report.previous_window.event_count + report.current_window.event_count,
            events.len()
        );
        assert!(!report.narrative.is_empty());
        assert!(report.narrative.contains("current window holds 7 events"));
        assert_eq!(report.horizon, 4);
        assert!(report.forecast_quality.is_none());
    }

    #[test]
    fn action_report_connects_decision_surface_to_evaluation() {
        let events = sample_events();
        let report = build_action_report(
            &events,
            4,
            0.6,
            &default_fields(),
            DEFAULT_BUCKET_FIELD,
            DEFAULT_WINDOW_MINUTES,
            &LocationResolver::default_demo(),
            &quiet_default_profile(),
        )
        .unwrap();

        assert_eq!(report.predictions.len(), 4);
        assert!(!report.heatmap.rows.is_empty());
        assert!(report.decision.forecast_quality.is_some());
        assert_eq!(report.alerts.profile, "quiet");
        let quality = report.decision.forecast_quality.as_ref().unwrap();
        assert!(quality.event_type_accuracy >= 0.0 && quality.event_type_accuracy <= 1.0);
        assert!(quality.property_accuracy.contains_key("location"));
        assert!(report.decision.narrative.contains("held-out quality"));
    }

    #[test]
    fn per_entity_models_split_by_entity_id() {
        let events = sample_events();
        let models = fit_per_entity_models(events, &default_fields());
        assert!(models.contains_key("order_1"));
        assert!(models.contains_key("order_2"));
        assert!(models.contains_key("order_3"));
    }

    #[test]
    fn anomalies_flag_intervals_far_from_mean() {
        let mut events = sample_events();
        let last_ts = events.last().unwrap().ts;
        events.push(Event {
            id: Some("evt_outlier".to_string()),
            ts: last_ts + Duration::hours(6),
            event_type: "merchant_confirmed".to_string(),
            entity_id: Some("order_5".to_string()),
            stream_id: None,
            properties: Map::new(),
        });
        let flagged = detect_anomalies(&events, 2.0);
        assert!(flagged
            .iter()
            .any(|flag| flag.event_id.as_deref() == Some("evt_outlier")));
    }

    #[test]
    fn numeric_forecast_returns_median_value() {
        let mut events = sample_events();
        for (idx, event) in events.iter_mut().enumerate() {
            event.properties.insert(
                "amount".to_string(),
                Value::Number(serde_json::Number::from(100 + idx as i64 * 5)),
            );
        }
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 3);
        let numeric = forecast_numeric_fields(&events, &predictions, &["amount".to_string()]);
        assert!(!numeric.is_empty());
        assert!(numeric.iter().all(|forecast| forecast.expected > 0.0));
    }

    #[test]
    fn evaluation_rejects_streams_that_are_too_short() {
        let events = sample_events().into_iter().take(3).collect();
        let result = evaluate_stream(events, 0.6, &default_fields());
        assert!(matches!(result, Err(ForecastError::NotEnoughEvents)));
    }

    #[test]
    fn can_roll_predictions_forward_into_future_stream() {
        let events = sample_events();
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 4);
        let summary = summarize_predictions(&predictions);

        assert_eq!(
            summary
                .iter()
                .map(|item| item.event_type.as_str())
                .collect::<Vec<_>>(),
            vec![
                "order_picked_up",
                "order_delivered",
                "order_created",
                "driver_assigned",
            ]
        );
        assert_eq!(summary[0].properties["location"], "indiranagar");
        assert_eq!(summary[2].properties["service_type"], "delivery");
    }
}
