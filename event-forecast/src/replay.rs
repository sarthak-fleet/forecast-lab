use crate::alerts::{evaluate_alerts, AlertProfile, AlertSummary};
use crate::location::LocationResolver;
use crate::{
    build_decision_report, build_heatmap, build_report, detect_anomalies, fit_model,
    predict_next_stream, DecisionReport, Event, EventPrediction, ForecastError, Heatmap,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftMarker {
    pub kind: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayStep {
    pub index: usize,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub observed_event_count: usize,
    pub history_event_count: usize,
    pub heatmap: Heatmap,
    pub decision: Option<DecisionReport>,
    pub predictions: Vec<EventPrediction>,
    pub drift_markers: Vec<DriftMarker>,
    pub alerts: AlertSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayPlayback {
    pub window_minutes: i64,
    pub step_minutes: i64,
    pub horizon: usize,
    pub total_steps: usize,
    pub steps: Vec<ReplayStep>,
}

pub fn build_replay(
    events: &[Event],
    window_minutes: i64,
    step_minutes: i64,
    horizon: usize,
    fields: &[String],
    bucket_field: &str,
    history_ratio: f64,
    location_resolver: &LocationResolver,
    alert_profile: &AlertProfile,
) -> Result<ReplayPlayback, ForecastError> {
    if events.len() < 2 {
        return Err(ForecastError::NotEnoughEvents);
    }

    let mut sorted = events.to_vec();
    sorted.sort_by_key(|event| event.ts);
    let window_minutes = window_minutes.max(1);
    let step_minutes = step_minutes.max(1);
    let min_ts = sorted.first().map(|event| event.ts).unwrap_or_else(Utc::now);
    let max_ts = sorted.last().map(|event| event.ts).unwrap_or_else(Utc::now);

    let mut cutoffs = Vec::new();
    let mut cursor = min_ts + Duration::minutes(window_minutes);
    while cursor <= max_ts {
        cutoffs.push(cursor);
        cursor += Duration::minutes(step_minutes);
    }
    if cutoffs.is_empty() {
        cutoffs.push(max_ts);
    }

    let mut steps = Vec::with_capacity(cutoffs.len());
    let mut previous_hot_bucket: Option<String> = None;
    let mut previous_mean_confidence: Option<f64> = None;

    for (index, cutoff) in cutoffs.iter().enumerate() {
        let window_end = *cutoff;
        let window_start = window_end - Duration::minutes(window_minutes);
        let history: Vec<Event> = sorted
            .iter()
            .filter(|event| event.ts <= window_end)
            .cloned()
            .collect();
        if history.len() < 2 {
            continue;
        }

        let window_events: Vec<Event> = history
            .iter()
            .filter(|event| event.ts > window_start && event.ts <= window_end)
            .cloned()
            .collect();

        let model = fit_model(history.clone(), fields)?;
        let predictions = predict_next_stream(&model, &history, horizon);
        let forecast = build_report(&predictions);
        let heatmap = build_heatmap(
            &window_events,
            &predictions,
            bucket_field,
            window_minutes,
            location_resolver,
        );
        let mut decision =
            build_decision_report(&history, &predictions, fields);
        if let Some(report) = decision.as_mut() {
            crate::attach_forecast_quality(report, &history, history_ratio, fields);
        }
        let anomalies = detect_anomalies(&window_events, 2.0);
        let alerts = decision.as_ref().map(|decision_report| {
            evaluate_alerts(
                decision_report,
                &forecast,
                &heatmap,
                &anomalies,
                alert_profile,
            )
        }).unwrap_or_else(|| AlertSummary {
            profile: alert_profile.name.clone(),
            alerts: vec![],
        });

        let mut drift_markers = Vec::new();
        if let Some(top_hot) = heatmap.hot_buckets.first() {
            if let Some(previous) = &previous_hot_bucket {
                if previous != &top_hot.bucket {
                    drift_markers.push(DriftMarker {
                        kind: "hot_zone_move".to_string(),
                        reason: format!(
                            "predicted hot zone moved from {previous} to {}",
                            top_hot.bucket
                        ),
                    });
                }
            }
            previous_hot_bucket = Some(top_hot.bucket.clone());
        }

        if let Some(decision_report) = &decision {
            for shift in &decision_report.mix_shifts {
                if shift.delta.abs() >= crate::MIX_SHIFT_THRESHOLD {
                    drift_markers.push(DriftMarker {
                        kind: "mix_shift".to_string(),
                        reason: format!(
                            "{} share shifted {:.0}% toward {}",
                            shift.field,
                            shift.delta * 100.0,
                            shift.value
                        ),
                    });
                }
            }
        }

        if let Some(previous_confidence) = previous_mean_confidence {
            if forecast.mean_event_confidence + 0.1 < previous_confidence {
                drift_markers.push(DriftMarker {
                    kind: "confidence_drop".to_string(),
                    reason: format!(
                        "mean confidence fell from {:.0}% to {:.0}%",
                        previous_confidence * 100.0,
                        forecast.mean_event_confidence * 100.0
                    ),
                });
            }
        }
        previous_mean_confidence = Some(forecast.mean_event_confidence);

        steps.push(ReplayStep {
            index,
            window_start,
            window_end,
            observed_event_count: window_events.len(),
            history_event_count: history.len(),
            heatmap,
            decision,
            predictions,
            drift_markers,
            alerts,
        });
    }

    if steps.is_empty() {
        return Err(ForecastError::NotEnoughEvents);
    }

    Ok(ReplayPlayback {
        window_minutes,
        step_minutes,
        horizon,
        total_steps: steps.len(),
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{default_fields, normalize_events, quiet_default_profile, RawEvent};

    fn sample_events() -> Vec<Event> {
        let raw: Vec<RawEvent> =
            serde_json::from_str(include_str!("../data/sample-events.json")).unwrap();
        normalize_events(raw).unwrap()
    }

    #[test]
    fn replay_produces_deterministic_steps_for_sample_stream() {
        let events = sample_events();
        let resolver = crate::location::LocationResolver::default_demo();
        let profile = quiet_default_profile();
        let first = build_replay(
            &events,
            30,
            15,
            3,
            &default_fields(),
            "location",
            0.6,
            &resolver,
            &profile,
        )
        .unwrap();
        let second = build_replay(
            &events,
            30,
            15,
            3,
            &default_fields(),
            "location",
            0.6,
            &resolver,
            &profile,
        )
        .unwrap();
        assert!(first.total_steps > 0);
        assert_eq!(first.total_steps, second.total_steps);
        assert_eq!(first.window_minutes, second.window_minutes);
        assert_eq!(first.step_minutes, second.step_minutes);
        assert_eq!(first.horizon, second.horizon);
        for (left, right) in first.steps.iter().zip(second.steps.iter()) {
            assert_eq!(left.index, right.index);
            assert_eq!(left.window_start, right.window_start);
            assert_eq!(left.window_end, right.window_end);
            assert_eq!(left.observed_event_count, right.observed_event_count);
            assert_eq!(left.history_event_count, right.history_event_count);
            assert_eq!(left.predictions.len(), right.predictions.len());
            assert_eq!(left.drift_markers.len(), right.drift_markers.len());
            assert_eq!(left.alerts.alerts.len(), right.alerts.alerts.len());
        }
    }

    #[test]
    fn replay_steps_carry_window_metadata_and_heatmap_rows() {
        let events = sample_events();
        let playback = build_replay(
            &events,
            30,
            15,
            2,
            &default_fields(),
            "location",
            0.6,
            &crate::location::LocationResolver::default_demo(),
            &quiet_default_profile(),
        )
        .unwrap();
        let step = playback.steps.first().expect("replay step");
        assert!(step.window_end > step.window_start);
        assert!(!step.heatmap.rows.is_empty() || step.observed_event_count == 0);
        assert!(!step.heatmap.locations.is_empty());
    }
}
