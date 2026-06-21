use crate::{
    AnomalyFlag, DecisionReport, ForecastReport, Heatmap, MixShift,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertKind {
    HotZoneGrowth,
    CoolingZoneCollapse,
    ConfidenceDrop,
    AnomalySpike,
    MixShift,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Alert {
    pub kind: AlertKind,
    pub severity: AlertSeverity,
    pub reason: String,
    pub window: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertProfile {
    pub name: String,
    pub hot_zone_delta_threshold: f64,
    pub cooling_zone_delta_threshold: f64,
    pub confidence_floor: f64,
    pub anomaly_count_threshold: usize,
    pub mix_shift_threshold: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AlertSummary {
    pub profile: String,
    pub alerts: Vec<Alert>,
}

impl Default for AlertProfile {
    fn default() -> Self {
        quiet_default_profile()
    }
}

pub fn quiet_default_profile() -> AlertProfile {
    AlertProfile {
        name: "quiet".to_string(),
        hot_zone_delta_threshold: 0.15,
        cooling_zone_delta_threshold: 0.15,
        confidence_floor: 0.4,
        anomaly_count_threshold: 3,
        mix_shift_threshold: 0.15,
    }
}

pub fn evaluate_alerts(
    decision: &DecisionReport,
    forecast: &ForecastReport,
    heatmap: &Heatmap,
    anomalies: &[AnomalyFlag],
    profile: &AlertProfile,
) -> AlertSummary {
    let mut alerts = Vec::new();

    for zone in &decision.hot_zones {
        if zone.delta >= profile.hot_zone_delta_threshold {
            alerts.push(build_hot_zone_alert(zone, profile));
        }
    }

    for zone in &decision.cooling_zones {
        if zone.delta.abs() >= profile.cooling_zone_delta_threshold {
            alerts.push(build_cooling_zone_alert(zone, profile));
        }
    }

    for bucket in &heatmap.hot_buckets {
        if bucket.delta > 0 && bucket.predicted_total as f64 >= profile.hot_zone_delta_threshold * 10.0
        {
            let key = format!("heatmap_hot:{}", bucket.bucket);
            if !alerts.iter().any(|alert| alert_key(alert) == key) {
                alerts.push(Alert {
                    kind: AlertKind::HotZoneGrowth,
                    severity: AlertSeverity::Warning,
                    reason: format!(
                        "predicted demand for {} exceeds observed by {} events",
                        bucket.bucket, bucket.delta
                    ),
                    window: "predicted".to_string(),
                    value: Some(bucket.bucket.clone()),
                    metric: Some(bucket.delta as f64),
                });
            }
        }
    }

    if forecast.mean_event_confidence < profile.confidence_floor {
        alerts.push(Alert {
            kind: AlertKind::ConfidenceDrop,
            severity: AlertSeverity::Warning,
            reason: format!(
                "mean forecast confidence {:.0}% is below {:.0}% floor",
                forecast.mean_event_confidence * 100.0,
                profile.confidence_floor * 100.0
            ),
            window: "predicted".to_string(),
            value: None,
            metric: Some(forecast.mean_event_confidence),
        });
    }

    for step in &forecast.low_confidence_steps {
        if step.confidence < profile.confidence_floor {
            alerts.push(Alert {
                kind: AlertKind::ConfidenceDrop,
                severity: AlertSeverity::Info,
                reason: format!(
                    "step {} ({}) confidence {:.0}% is below floor",
                    step.step,
                    step.event_type,
                    step.confidence * 100.0
                ),
                window: "predicted".to_string(),
                value: Some(step.event_type.clone()),
                metric: Some(step.confidence),
            });
        }
    }

    if anomalies.len() >= profile.anomaly_count_threshold {
        alerts.push(Alert {
            kind: AlertKind::AnomalySpike,
            severity: AlertSeverity::Critical,
            reason: format!(
                "{} anomalous inter-arrival intervals detected",
                anomalies.len()
            ),
            window: "observed".to_string(),
            value: None,
            metric: Some(anomalies.len() as f64),
        });
    } else if !anomalies.is_empty() {
        alerts.push(Alert {
            kind: AlertKind::AnomalySpike,
            severity: AlertSeverity::Info,
            reason: format!(
                "{} unusual interval{} below spike threshold",
                anomalies.len(),
                if anomalies.len() == 1 { "" } else { "s" }
            ),
            window: "observed".to_string(),
            value: None,
            metric: Some(anomalies.len() as f64),
        });
    }

    for shift in &decision.mix_shifts {
        if shift.field != "location" && shift.delta.abs() >= profile.mix_shift_threshold {
            alerts.push(Alert {
                kind: AlertKind::MixShift,
                severity: if shift.delta.abs() >= profile.mix_shift_threshold * 1.5 {
                    AlertSeverity::Warning
                } else {
                    AlertSeverity::Info
                },
                reason: format!(
                    "{} share shifted {:.0}% toward {}",
                    shift.field,
                    shift.delta * 100.0,
                    shift.value
                ),
                window: "observed".to_string(),
                value: Some(shift.value.clone()),
                metric: Some(shift.delta),
            });
        }
    }

    dedupe_alerts(&mut alerts);
    alerts.sort_by(|left, right| {
        severity_rank(&right.severity)
            .cmp(&severity_rank(&left.severity))
            .then_with(|| left.kind.to_string().cmp(&right.kind.to_string()))
    });

    AlertSummary {
        profile: profile.name.clone(),
        alerts,
    }
}

fn build_hot_zone_alert(zone: &MixShift, profile: &AlertProfile) -> Alert {
    Alert {
        kind: AlertKind::HotZoneGrowth,
        severity: if zone.delta >= profile.hot_zone_delta_threshold * 1.5 {
            AlertSeverity::Warning
        } else {
            AlertSeverity::Info
        },
        reason: format!(
            "{} heating up {:.0}% in observed window",
            zone.value,
            zone.delta * 100.0
        ),
        window: "observed".to_string(),
        value: Some(zone.value.clone()),
        metric: Some(zone.delta),
    }
}

fn build_cooling_zone_alert(zone: &MixShift, profile: &AlertProfile) -> Alert {
    Alert {
        kind: AlertKind::CoolingZoneCollapse,
        severity: if zone.delta.abs() >= profile.cooling_zone_delta_threshold * 1.5 {
            AlertSeverity::Warning
        } else {
            AlertSeverity::Info
        },
        reason: format!(
            "{} cooling {:.0}% in observed window",
            zone.value,
            zone.delta * 100.0
        ),
        window: "observed".to_string(),
        value: Some(zone.value.clone()),
        metric: Some(zone.delta),
    }
}

fn alert_key(alert: &Alert) -> String {
    format!(
        "{:?}:{}:{}",
        alert.kind,
        alert.window,
        alert.value.as_deref().unwrap_or("")
    )
}

fn dedupe_alerts(alerts: &mut Vec<Alert>) {
    let mut seen = std::collections::HashSet::new();
    alerts.retain(|alert| seen.insert(alert_key(alert)));
}

fn severity_rank(severity: &AlertSeverity) -> u8 {
    match severity {
        AlertSeverity::Critical => 3,
        AlertSeverity::Warning => 2,
        AlertSeverity::Info => 1,
    }
}

impl AlertKind {
    fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HeatmapBucketSummary, LowConfidenceStep, WindowStats, build_report, default_fields,
        detect_anomalies, fit_model, normalize_events, predict_next_stream, RawEvent,
    };
    use chrono::{Duration, Utc};
    use pretty_assertions::assert_eq;

    fn sample_events() -> Vec<crate::Event> {
        let raw: Vec<RawEvent> =
            serde_json::from_str(include_str!("../data/sample-events.json")).unwrap();
        normalize_events(raw).unwrap()
    }

    fn decision_from(events: &[crate::Event], horizon: usize) -> DecisionReport {
        let model = fit_model(events.to_vec(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, events, horizon);
        crate::build_decision_report(events, &predictions, &default_fields()).unwrap()
    }

    #[test]
    fn quiet_stream_stays_mostly_silent() {
        let events = sample_events();
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 3);
        let decision = crate::build_decision_report(&events, &predictions, &default_fields()).unwrap();
        let forecast = build_report(&predictions);
        let heatmap = crate::build_heatmap(
            &events,
            &predictions,
            "location",
            30,
            &crate::location::LocationResolver::default_demo(),
        );
        let anomalies = detect_anomalies(&events, 2.0);
        let summary = evaluate_alerts(
            &decision,
            &forecast,
            &heatmap,
            &anomalies,
            &quiet_default_profile(),
        );
        let critical = summary
            .alerts
            .iter()
            .filter(|alert| alert.severity == AlertSeverity::Critical)
            .count();
        assert_eq!(critical, 0);
    }

    #[test]
    fn hot_zone_surge_triggers_once_per_window() {
        let mut events = sample_events();
        for idx in 0..6 {
            events.push(crate::Event {
                id: Some(format!("surge_{idx}")),
                ts: events.last().unwrap().ts + Duration::minutes(2),
                event_type: "order_created".to_string(),
                entity_id: Some(format!("surge_{idx}")),
                stream_id: None,
                properties: serde_json::Map::from_iter([(
                    "location".to_string(),
                    serde_json::Value::String("whitefield".to_string()),
                )]),
            });
        }
        let decision = decision_from(&events, 4);
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let predictions = predict_next_stream(&model, &events, 4);
        let forecast = build_report(&predictions);
        let heatmap = crate::build_heatmap(
            &events,
            &predictions,
            "location",
            30,
            &crate::location::LocationResolver::default_demo(),
        );
        let summary = evaluate_alerts(
            &decision,
            &forecast,
            &heatmap,
            &[],
            &quiet_default_profile(),
        );
        let hot_alerts: Vec<_> = summary
            .alerts
            .iter()
            .filter(|alert| alert.kind == AlertKind::HotZoneGrowth)
            .collect();
        assert!(!hot_alerts.is_empty());
        let whitefield_hot = hot_alerts
            .iter()
            .filter(|alert| alert.value.as_deref() == Some("whitefield"))
            .count();
        assert_eq!(whitefield_hot, 1);
    }

    #[test]
    fn low_confidence_and_anomalies_are_distinct() {
        let mut events = sample_events();
        let last_ts = events.last().unwrap().ts;
        events.push(crate::Event {
            id: Some("evt_outlier".to_string()),
            ts: last_ts + Duration::hours(6),
            event_type: "merchant_confirmed".to_string(),
            entity_id: Some("order_5".to_string()),
            stream_id: None,
            properties: serde_json::Map::new(),
        });
        let decision = decision_from(&events, 6);
        let model = fit_model(events.clone(), &default_fields()).unwrap();
        let mut predictions = predict_next_stream(&model, &events, 6);
        for prediction in &mut predictions {
            prediction.confidence = 0.2;
        }
        let forecast = build_report(&predictions);
        let heatmap = crate::build_heatmap(
            &events,
            &predictions,
            "location",
            30,
            &crate::location::LocationResolver::default_demo(),
        );
        let anomalies = detect_anomalies(&events, 2.0);
        let summary = evaluate_alerts(
            &decision,
            &forecast,
            &heatmap,
            &anomalies,
            &quiet_default_profile(),
        );
        let confidence = summary
            .alerts
            .iter()
            .any(|alert| alert.kind == AlertKind::ConfidenceDrop);
        let anomaly = summary
            .alerts
            .iter()
            .any(|alert| alert.kind == AlertKind::AnomalySpike);
        assert!(confidence);
        assert!(anomaly);
        assert!(
            summary
                .alerts
                .iter()
                .any(|alert| alert.kind == AlertKind::ConfidenceDrop && alert.severity != AlertSeverity::Critical)
        );
        assert!(
            summary
                .alerts
                .iter()
                .any(|alert| alert.kind == AlertKind::AnomalySpike && alert.severity == AlertSeverity::Info)
        );
    }

    #[test]
    fn synthetic_decision_report_respects_thresholds() {
        let decision = DecisionReport {
            horizon: 4,
            narrative: "test".to_string(),
            previous_window: WindowStats {
                start: Utc::now(),
                end: Utc::now(),
                event_count: 4,
            },
            current_window: WindowStats {
                start: Utc::now(),
                end: Utc::now(),
                event_count: 8,
            },
            hot_zones: vec![MixShift {
                field: "location".to_string(),
                value: "north".to_string(),
                previous_share: 0.1,
                current_share: 0.4,
                delta: 0.3,
            }],
            cooling_zones: vec![],
            mix_shifts: vec![],
            forecast_headline: "test".to_string(),
            low_confidence_steps: vec![LowConfidenceStep {
                step: 1,
                event_type: "order_created".to_string(),
                confidence: 0.2,
            }],
            forecast_quality: None,
        };
        let forecast = ForecastReport {
            horizon: 4,
            headline: "test".to_string(),
            next_likely_events: vec![],
            property_mix: Default::default(),
            low_confidence_steps: decision.low_confidence_steps.clone(),
            mean_event_confidence: 0.3,
        };
        let heatmap = crate::Heatmap {
            bucket_field: "location".to_string(),
            window_minutes: 30,
            horizon: 4,
            rows: vec![],
            hot_buckets: vec![HeatmapBucketSummary {
                bucket: "north".to_string(),
                observed_total: 1,
                predicted_total: 5,
                delta: 4,
            }],
            cooling_buckets: vec![],
            locations: Default::default(),
        };
        let summary = evaluate_alerts(&decision, &forecast, &heatmap, &[], &quiet_default_profile());
        assert!(summary
            .alerts
            .iter()
            .any(|alert| alert.kind == AlertKind::HotZoneGrowth));
        assert!(summary
            .alerts
            .iter()
            .any(|alert| alert.kind == AlertKind::ConfidenceDrop));
    }
}
