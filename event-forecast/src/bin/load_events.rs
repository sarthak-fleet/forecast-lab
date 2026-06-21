use anyhow::{Context, Result};
use event_forecast::{normalize_events, Event, RawEvent};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug)]
struct Args {
    path: PathBuf,
    stream_id: String,
}

fn parse_args() -> Result<Args> {
    parse_args_from(env::args().skip(1))
}

fn parse_args_from<I>(args: I) -> Result<Args>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let mut path: Option<PathBuf> = None;
    let mut stream_id: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stream-id" | "-s" => {
                stream_id = Some(args.next().context("--stream-id requires a value")?);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            value if !value.starts_with('-') && path.is_none() => {
                path = Some(PathBuf::from(value));
            }
            other => {
                anyhow::bail!("unexpected argument: {other}");
            }
        }
    }
    Ok(Args {
        path: path.unwrap_or_else(|| PathBuf::from("data/sample-events.json")),
        stream_id: stream_id.unwrap_or_else(|| "sample".to_string()),
    })
}

fn print_help() {
    println!(
        "usage: load_events [path] [--stream-id <id>]\n\n\
        Reads a JSON array of events and inserts them into the events hypertable.\n\
        Defaults: path=data/sample-events.json stream-id=sample.\n\
        Requires DATABASE_URL to point at a TimescaleDB/Postgres instance."
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let database_url =
        env::var("DATABASE_URL").context("DATABASE_URL must be set to a Postgres URL")?;
    let raw = fs::read_to_string(&args.path)
        .with_context(|| format!("read events file {}", args.path.display()))?;
    let raw_events: Vec<RawEvent> = serde_json::from_str(&raw).context("parse events JSON")?;
    let events = normalize_events(raw_events).context("normalize events")?;

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .context("connect to DATABASE_URL")?;

    let inserted = insert_events(&pool, &args.stream_id, &events).await?;
    println!(
        "loaded {inserted} events into stream {stream}",
        stream = args.stream_id
    );
    Ok(())
}

async fn insert_events(pool: &PgPool, fallback_stream: &str, events: &[Event]) -> Result<usize> {
    let mut tx = pool.begin().await.context("begin transaction")?;
    let mut inserted = 0;
    for event in events {
        let id = parse_event_id(event.id.as_deref())?;
        let stream_id = event.stream_id.as_deref().unwrap_or(fallback_stream);
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
    tx.commit().await.context("commit transaction")?;
    Ok(inserted)
}

fn parse_event_id(id: Option<&str>) -> Result<Uuid> {
    match id {
        Some(value) => Ok(Uuid::parse_str(value).context("invalid event id")?),
        None => anyhow::bail!("event id is required for ingestion"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_rejects_missing_stream_id_value() {
        let err = parse_args_from(vec!["--stream-id".to_string()]).unwrap_err();
        assert!(err.to_string().contains("--stream-id requires a value"));
    }

    #[test]
    fn parse_event_id_rejects_missing_ids() {
        let err = parse_event_id(None).unwrap_err();
        assert!(err
            .to_string()
            .contains("event id is required for ingestion"));
    }
}
