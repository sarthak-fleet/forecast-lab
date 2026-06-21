use anyhow::Context;
use event_forecast::{
    build_action_report, build_decision_report, build_heatmap, build_replay, build_report,
    default_fields, detect_anomalies, evaluate_stream, fit_model, fit_per_entity_models,
    forecast_numeric_fields, normalize_events, parse_location_catalog, predict_next_stream,
    quiet_default_profile, summarize_predictions, ActionReport, AnomalyFlag, DecisionReport,
    EvaluationResult, Event, EventPrediction, ForecastReport, Heatmap, LocationCoords,
    NumericForecast, RawEvent, ReplayPlayback, DEFAULT_BUCKET_FIELD, DEFAULT_WINDOW_MINUTES,
};
use rocket::http::Status;
use rocket::response::content::RawHtml;
use rocket::serde::json::Json;
use rocket::{get, launch, post, routes, State};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::env;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: Option<PgPool>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
    database: &'static str,
}

#[derive(Debug, Deserialize)]
struct IngestRequest {
    #[serde(default = "default_stream_id")]
    stream_id: String,
    events: Vec<RawEvent>,
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    stream_id: String,
    inserted: usize,
}

#[derive(Debug, Deserialize)]
struct PredictRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_horizon")]
    horizon: usize,
    fields: Option<Vec<String>>,
    #[serde(default)]
    summary: bool,
    entity_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum PredictResponse {
    Full(Vec<EventPrediction>),
    Summary(Vec<event_forecast::PredictionSummary>),
}

#[derive(Debug, Deserialize)]
struct AnomalyRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_z_threshold")]
    z_threshold: f64,
}

#[derive(Debug, Deserialize)]
struct NumericRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_horizon")]
    horizon: usize,
    fields: Option<Vec<String>>,
    numeric_fields: Vec<String>,
    entity_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReportRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_horizon")]
    horizon: usize,
    fields: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct EvaluateRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_history_ratio")]
    history_ratio: f64,
    fields: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct DecisionRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_horizon")]
    horizon: usize,
    fields: Option<Vec<String>>,
    #[serde(default = "default_history_ratio")]
    history_ratio: f64,
}

#[derive(Debug, Deserialize)]
struct ActionReportRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_horizon")]
    horizon: usize,
    fields: Option<Vec<String>>,
    #[serde(default = "default_history_ratio")]
    history_ratio: f64,
    bucket_field: Option<String>,
    window_minutes: Option<i64>,
    location_catalog: Option<HashMap<String, LocationCoords>>,
}

#[derive(Debug, Deserialize)]
struct ReplayRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_horizon")]
    horizon: usize,
    fields: Option<Vec<String>>,
    #[serde(default = "default_history_ratio")]
    history_ratio: f64,
    bucket_field: Option<String>,
    window_minutes: Option<i64>,
    #[serde(default = "default_step_minutes")]
    step_minutes: i64,
    location_catalog: Option<HashMap<String, LocationCoords>>,
}

#[derive(Debug, Deserialize)]
struct HeatmapRequest {
    events: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
    #[serde(default = "default_horizon")]
    horizon: usize,
    fields: Option<Vec<String>>,
    bucket_field: Option<String>,
    window_minutes: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
}

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Event Forecast</title>
    <style>
      body { font-family: system-ui, sans-serif; max-width: 760px; margin: 48px auto; padding: 0 20px; line-height: 1.5; }
      code, pre { background: #f3f4f6; border-radius: 6px; }
      code { padding: 2px 5px; }
      pre { padding: 14px; overflow: auto; }
    </style>
  </head>
  <body>
    <h1>Event Forecast</h1>
    <p>Rust + Rocket service for predicting the next event stream and likely properties.</p>
    <h2>Endpoints</h2>
    <ul>
      <li><a href="/health"><code>GET /health</code></a></li>
      <li><code>POST /events</code> ingest events into TimescaleDB</li>
      <li><code>POST /predict</code> predict from inline events or a stored stream</li>
      <li><code>POST /report</code> aggregate next-stream mix and uncertainty</li>
      <li><code>POST /evaluate</code> score the model against a held-out stream suffix</li>
      <li><code>POST /heatmap</code> location-by-time buckets for observed and predicted demand</li>
      <li><code>POST /anomalies</code> flag events whose inter-arrival interval is anomalous</li>
      <li><code>POST /numeric</code> forecast numeric properties (price, dwell time, etc.) over the horizon</li>
      <li><code>POST /decision-report</code> compare windows, flag hot/cooling zones, narrate the forecast</li>
      <li><code>POST /action-report</code> map + decision report + held-out forecast quality in one payload</li>
      <li><code>POST /replay</code> windowed stream replay with per-step heatmap, forecast, and drift markers</li>
      <li><a href="/explorer"><code>GET /explorer</code></a> interactive map + timeline UI</li>
    </ul>
    <h2>Stored stream example</h2>
    <pre>curl -s http://127.0.0.1:8088/predict \
  -H 'content-type: application/json' \
  -d '{"stream_id":"orders-demo","horizon":2,"summary":true}'</pre>
  </body>
</html>"#,
    )
}

#[get("/explorer")]
fn explorer() -> RawHtml<&'static str> {
    RawHtml(include_str!("explorer.html"))
}

#[get("/sample-events.json")]
fn sample_events() -> rocket::response::content::RawJson<&'static str> {
    rocket::response::content::RawJson(include_str!("../data/sample-events.json"))
}

#[get("/health")]
async fn health(state: &State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        database: if state.db.is_some() {
            "configured"
        } else {
            "not_configured"
        },
    })
}

#[post("/events", data = "<body>")]
async fn ingest_events(
    state: &State<AppState>,
    body: Json<IngestRequest>,
) -> Result<Json<IngestResponse>, (Status, Json<ApiError>)> {
    let db = state.db.as_ref().ok_or_else(db_not_configured)?;
    let request = body.into_inner();
    let events = normalize_events(request.events).map_err(bad_request)?;
    let stream_id = events
        .first()
        .and_then(|event| event.stream_id.clone())
        .unwrap_or(request.stream_id);

    let tx = db.begin().await.map_err(internal_error)?;
    let inserted = insert_events(tx, &stream_id, &events)
        .await
        .map_err(internal_error)?;

    Ok(Json(IngestResponse {
        stream_id,
        inserted,
    }))
}

#[post("/predict", data = "<body>")]
async fn predict(
    state: &State<AppState>,
    body: Json<PredictRequest>,
) -> Result<Json<PredictResponse>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    let horizon = validate_horizon(body.horizon)?;
    let fields = body.fields.unwrap_or_else(default_fields);
    let events = resolve_events(state, body.events, body.stream_id).await?;
    let (model, seed) = pick_model(&events, &fields, body.entity_id.as_deref())?;
    let predictions = predict_next_stream(&model, &seed, horizon);

    if body.summary {
        Ok(Json(PredictResponse::Summary(summarize_predictions(
            &predictions,
        ))))
    } else {
        Ok(Json(PredictResponse::Full(predictions)))
    }
}

#[post("/anomalies", data = "<body>")]
async fn anomalies(
    state: &State<AppState>,
    body: Json<AnomalyRequest>,
) -> Result<Json<Vec<AnomalyFlag>>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    let events = resolve_events(state, body.events, body.stream_id).await?;
    Ok(Json(detect_anomalies(&events, body.z_threshold)))
}

#[post("/numeric", data = "<body>")]
async fn numeric(
    state: &State<AppState>,
    body: Json<NumericRequest>,
) -> Result<Json<Vec<NumericForecast>>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    let horizon = validate_horizon(body.horizon)?;
    let fields = body.fields.unwrap_or_else(default_fields);
    let numeric_fields = body.numeric_fields;
    if numeric_fields.is_empty() {
        return Err(bad_request(
            "numeric_fields must include at least one field",
        ));
    }
    let events = resolve_events(state, body.events, body.stream_id).await?;
    let (model, seed) = pick_model(&events, &fields, body.entity_id.as_deref())?;
    let predictions = predict_next_stream(&model, &seed, horizon);
    let numeric_events = numeric_sample_events(&events, &seed, body.entity_id.as_deref());
    Ok(Json(forecast_numeric_fields(
        numeric_events,
        &predictions,
        &numeric_fields,
    )))
}

fn numeric_sample_events<'a>(
    events: &'a [Event],
    seed: &'a [Event],
    entity_id: Option<&str>,
) -> &'a [Event] {
    if entity_id.is_some() {
        seed
    } else {
        events
    }
}

fn pick_model(
    events: &[Event],
    fields: &[String],
    entity_id: Option<&str>,
) -> Result<(event_forecast::ForecastModel, Vec<Event>), (Status, Json<ApiError>)> {
    if let Some(entity) = entity_id {
        let per_entity = fit_per_entity_models(events.to_vec(), fields);
        if let Some(model) = per_entity.get(entity) {
            let seed = events
                .iter()
                .filter(|event| event.entity_id.as_deref() == Some(entity))
                .cloned()
                .collect();
            return Ok((model.clone(), seed));
        }
    }
    let model = fit_model(events.to_vec(), fields).map_err(bad_request)?;
    Ok((model, events.to_vec()))
}

#[post("/report", data = "<body>")]
async fn report(
    state: &State<AppState>,
    body: Json<ReportRequest>,
) -> Result<Json<ForecastReport>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    let horizon = validate_horizon(body.horizon)?;
    let fields = body.fields.unwrap_or_else(default_fields);
    let events = resolve_events(state, body.events, body.stream_id).await?;
    let model = fit_model(events.clone(), &fields).map_err(bad_request)?;
    let predictions = predict_next_stream(&model, &events, horizon);
    Ok(Json(build_report(&predictions)))
}

#[post("/evaluate", data = "<body>")]
async fn evaluate(
    state: &State<AppState>,
    body: Json<EvaluateRequest>,
) -> Result<Json<EvaluationResult>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    validate_history_ratio(body.history_ratio)?;
    let fields = body.fields.unwrap_or_else(default_fields);
    let events = resolve_events(state, body.events, body.stream_id).await?;
    let result = evaluate_stream(events, body.history_ratio, &fields).map_err(bad_request)?;
    Ok(Json(result))
}

#[post("/decision-report", data = "<body>")]
async fn decision_report(
    state: &State<AppState>,
    body: Json<DecisionRequest>,
) -> Result<Json<DecisionReport>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    let horizon = validate_horizon(body.horizon)?;
    validate_history_ratio(body.history_ratio)?;
    let fields = body.fields.unwrap_or_else(default_fields);
    let events = resolve_events(state, body.events, body.stream_id).await?;
    let model = fit_model(events.clone(), &fields).map_err(bad_request)?;
    let predictions = predict_next_stream(&model, &events, horizon);
    let mut report = build_decision_report(&events, &predictions, &fields)
        .ok_or_else(|| bad_request("stream too short for a decision report"))?;
    event_forecast::attach_forecast_quality(&mut report, &events, body.history_ratio, &fields);
    Ok(Json(report))
}

#[post("/action-report", data = "<body>")]
async fn action_report(
    state: &State<AppState>,
    body: Json<ActionReportRequest>,
) -> Result<Json<ActionReport>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    let horizon = validate_horizon(body.horizon)?;
    validate_history_ratio(body.history_ratio)?;
    let fields = body.fields.unwrap_or_else(default_fields);
    let bucket_field = body
        .bucket_field
        .unwrap_or_else(|| DEFAULT_BUCKET_FIELD.to_string());
    let window_minutes = body.window_minutes.unwrap_or(DEFAULT_WINDOW_MINUTES);
    let events = resolve_events(state, body.events, body.stream_id).await?;
    let resolver = parse_location_catalog(body.location_catalog);
    let report = build_action_report(
        &events,
        horizon,
        body.history_ratio,
        &fields,
        &bucket_field,
        window_minutes,
        &resolver,
        &quiet_default_profile(),
    )
    .map_err(bad_request)?;
    Ok(Json(report))
}

#[post("/replay", data = "<body>")]
async fn replay(
    state: &State<AppState>,
    body: Json<ReplayRequest>,
) -> Result<Json<ReplayPlayback>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    let horizon = validate_horizon(body.horizon)?;
    validate_history_ratio(body.history_ratio)?;
    let fields = body.fields.unwrap_or_else(default_fields);
    let bucket_field = body
        .bucket_field
        .unwrap_or_else(|| DEFAULT_BUCKET_FIELD.to_string());
    let window_minutes = body.window_minutes.unwrap_or(DEFAULT_WINDOW_MINUTES);
    let events = resolve_events(state, body.events, body.stream_id).await?;
    let resolver = parse_location_catalog(body.location_catalog);
    let playback = build_replay(
        &events,
        window_minutes,
        body.step_minutes,
        horizon,
        &fields,
        &bucket_field,
        body.history_ratio,
        &resolver,
        &quiet_default_profile(),
    )
    .map_err(bad_request)?;
    Ok(Json(playback))
}

#[post("/heatmap", data = "<body>")]
async fn heatmap(
    state: &State<AppState>,
    body: Json<HeatmapRequest>,
) -> Result<Json<Heatmap>, (Status, Json<ApiError>)> {
    let body = body.into_inner();
    let horizon = validate_horizon(body.horizon)?;
    let fields = body.fields.unwrap_or_else(default_fields);
    let bucket_field = body
        .bucket_field
        .unwrap_or_else(|| DEFAULT_BUCKET_FIELD.to_string());
    let window_minutes = body.window_minutes.unwrap_or(DEFAULT_WINDOW_MINUTES);
    let events = resolve_events(state, body.events, body.stream_id).await?;
    let model = fit_model(events.clone(), &fields).map_err(bad_request)?;
    let predictions = predict_next_stream(&model, &events, horizon);
    let resolver = parse_location_catalog(None);
    let map = build_heatmap(
        &events,
        &predictions,
        &bucket_field,
        window_minutes,
        &resolver,
    );
    Ok(Json(map))
}

async fn resolve_events(
    state: &State<AppState>,
    inline: Option<Vec<RawEvent>>,
    stream_id: Option<String>,
) -> Result<Vec<Event>, (Status, Json<ApiError>)> {
    if let Some(raw_events) = inline {
        normalize_events(raw_events).map_err(bad_request)
    } else if let Some(stream_id) = stream_id {
        let db = state.db.as_ref().ok_or_else(db_not_configured)?;
        load_events(db, &stream_id).await.map_err(internal_error)
    } else {
        Err(bad_request("events or stream_id is required"))
    }
}

async fn insert_events(
    mut tx: sqlx::Transaction<'_, sqlx::Postgres>,
    fallback_stream_id: &str,
    events: &[Event],
) -> anyhow::Result<usize> {
    let mut inserted = 0;
    for event in events {
        let id = parse_event_id(event.id.as_deref())?;
        let stream_id = event.stream_id.as_deref().unwrap_or(fallback_stream_id);
        let result = sqlx::query(
            r#"
            insert into events (id, stream_id, entity_id, ts, event_type, properties)
            values ($1, $2, $3, $4, $5, $6)
            on conflict (id, ts) do nothing
            "#,
        )
        .bind(id)
        .bind(stream_id)
        .bind(&event.entity_id)
        .bind(event.ts)
        .bind(&event.event_type)
        .bind(Value::Object(event.properties.clone()))
        .execute(&mut *tx)
        .await
        .context("insert event")?;
        inserted += result.rows_affected() as usize;
    }
    tx.commit().await.context("commit event insert")?;
    Ok(inserted)
}

fn parse_event_id(id: Option<&str>) -> anyhow::Result<Uuid> {
    match id {
        Some(value) => Ok(Uuid::parse_str(value).context("invalid event id")?),
        None => anyhow::bail!("event id is required for ingestion"),
    }
}

async fn load_events(db: &PgPool, stream_id: &str) -> anyhow::Result<Vec<Event>> {
    let rows = sqlx::query(
        r#"
        select id::text, stream_id, entity_id, ts, event_type, properties
        from events
        where stream_id = $1
        order by ts asc
        "#,
    )
    .bind(stream_id)
    .fetch_all(db)
    .await
    .context("load events")?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let properties: Value = row.get("properties");
            Event {
                id: Some(row.get("id")),
                stream_id: Some(row.get("stream_id")),
                entity_id: row.get("entity_id"),
                ts: row.get("ts"),
                event_type: row.get("event_type"),
                properties: match properties {
                    Value::Object(map) => map,
                    _ => Map::new(),
                },
            }
        })
        .collect())
}

async fn build_state() -> AppState {
    let db = match env::var("DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => Some(
            PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .expect("connect to Timescale/Postgres DATABASE_URL"),
        ),
        _ => None,
    };
    AppState { db }
}

fn default_horizon() -> usize {
    5
}

/// Upper bound on forecast steps per request. Prediction work and memory grow
/// linearly with `horizon`, so an unchecked request-supplied value can abort
/// the process on allocation failure.
const MAX_HORIZON: usize = 10_000;

fn validate_horizon(horizon: usize) -> Result<usize, (Status, Json<ApiError>)> {
    if horizon > MAX_HORIZON {
        Err(bad_request(format!(
            "horizon must be at most {MAX_HORIZON}"
        )))
    } else {
        Ok(horizon)
    }
}

fn validate_history_ratio(history_ratio: f64) -> Result<f64, (Status, Json<ApiError>)> {
    if !(0.1..=0.9).contains(&history_ratio) {
        Err(bad_request("history_ratio must be between 0.1 and 0.9"))
    } else {
        Ok(history_ratio)
    }
}

fn default_step_minutes() -> i64 {
    15
}

fn default_history_ratio() -> f64 {
    0.6
}

fn default_z_threshold() -> f64 {
    2.0
}

fn default_stream_id() -> String {
    "default".to_string()
}

fn db_not_configured() -> (Status, Json<ApiError>) {
    (
        Status::ServiceUnavailable,
        Json(ApiError {
            error: "DATABASE_URL is not configured".to_string(),
        }),
    )
}

fn bad_request(error: impl ToString) -> (Status, Json<ApiError>) {
    (
        Status::BadRequest,
        Json(ApiError {
            error: error.to_string(),
        }),
    )
}

fn internal_error(error: impl ToString) -> (Status, Json<ApiError>) {
    (
        Status::InternalServerError,
        Json(ApiError {
            error: error.to_string(),
        }),
    )
}

#[launch]
async fn rocket() -> _ {
    rocket::build().manage(build_state().await).mount(
        "/",
        routes![
            index,
            health,
            ingest_events,
            predict,
            report,
            evaluate,
            heatmap,
            anomalies,
            numeric,
            decision_report,
            action_report,
            replay,
            explorer,
            sample_events
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_forecast::{
        default_fields, fit_per_entity_models, normalize_events, predict_next_stream,
    };
    use serde_json::json;

    #[test]
    fn numeric_forecast_uses_entity_scoped_samples_when_requested() {
        let raw = vec![
            json!({"id":"a1","ts":"2026-06-03T09:00:00Z","event_type":"order_created","entity_id":"A","properties":{"amount":10}}),
            json!({"id":"a2","ts":"2026-06-03T09:01:00Z","event_type":"order_created","entity_id":"A","properties":{"amount":20}}),
            json!({"id":"b1","ts":"2026-06-03T09:02:00Z","event_type":"order_created","entity_id":"B","properties":{"amount":1000}}),
            json!({"id":"b2","ts":"2026-06-03T09:03:00Z","event_type":"order_created","entity_id":"B","properties":{"amount":2000}}),
        ];
        let raw_events: Vec<event_forecast::RawEvent> =
            serde_json::from_value(serde_json::Value::Array(raw)).unwrap();
        let events = normalize_events(raw_events).unwrap();
        let models = fit_per_entity_models(events.clone(), &default_fields());
        let model = models.get("A").unwrap();
        let seed: Vec<_> = events
            .iter()
            .filter(|event| event.entity_id.as_deref() == Some("A"))
            .cloned()
            .collect();
        let predictions = predict_next_stream(model, &seed, 1);

        let global = numeric_sample_events(&events, &seed, None);
        let scoped = numeric_sample_events(&events, &seed, Some("A"));

        assert_eq!(global.len(), events.len());
        assert_eq!(scoped.len(), seed.len());

        let global_forecast =
            event_forecast::forecast_numeric_fields(global, &predictions, &["amount".to_string()]);
        let scoped_forecast =
            event_forecast::forecast_numeric_fields(scoped, &predictions, &["amount".to_string()]);

        assert_eq!(global_forecast[0].expected, 1000.0);
        assert_eq!(scoped_forecast[0].expected, 20.0);
    }

    #[test]
    fn parse_event_id_rejects_malformed_uuids() {
        let err = parse_event_id(Some("not-a-uuid")).unwrap_err();
        assert!(err.to_string().contains("invalid event id"));
    }

    #[test]
    fn parse_event_id_rejects_missing_ids() {
        let err = parse_event_id(None).unwrap_err();
        assert!(err
            .to_string()
            .contains("event id is required for ingestion"));
    }

    #[test]
    fn validate_history_ratio_rejects_out_of_range_values() {
        let err = validate_history_ratio(1.2).unwrap_err();
        assert!(err.1 .0.error.contains("between 0.1 and 0.9"));
    }
}
