use anyhow::{Context, Result};
use event_forecast::{default_fields, evaluate_stream, normalize_events, RawEvent};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug)]
struct Args {
    path: PathBuf,
    history_ratio: f64,
    output: Option<PathBuf>,
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
    let mut history_ratio = 0.6;
    let mut output: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--history-ratio" | "-r" => {
                let value = args
                    .next()
                    .context("--history-ratio requires a value between 0.1 and 0.9")?;
                history_ratio = parse_history_ratio(&value)?;
            }
            "--output" | "-o" => {
                output = Some(PathBuf::from(
                    args.next().context("--output requires a file path")?,
                ));
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            value if !value.starts_with('-') && path.is_none() => {
                path = Some(PathBuf::from(value));
            }
            other => anyhow::bail!("unexpected argument: {other}"),
        }
    }

    Ok(Args {
        path: path.unwrap_or_else(|| PathBuf::from("tests/fixtures/sample-stream.json")),
        history_ratio,
        output,
    })
}

fn parse_history_ratio(value: &str) -> Result<f64> {
    let ratio: f64 = value.parse().context("history ratio must be a number")?;
    if !(0.1..=0.9).contains(&ratio) {
        anyhow::bail!("history ratio must be between 0.1 and 0.9");
    }
    Ok(ratio)
}

fn print_help() {
    println!(
        "usage: evaluate [path] [--history-ratio <ratio>] [--output <file>]\n\n\
        Reads a JSON array of events, withholds the stream suffix, predicts it,\n\
        and writes evaluation metrics as JSON.\n\
        Defaults: path=tests/fixtures/sample-stream.json history-ratio=0.6.\n\
        Without --output, metrics are printed to stdout."
    );
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let raw = fs::read_to_string(&args.path)
        .with_context(|| format!("read events file {}", args.path.display()))?;
    let raw_events: Vec<RawEvent> = serde_json::from_str(&raw).context("parse events JSON")?;
    let events = normalize_events(raw_events).context("normalize events")?;
    let result = evaluate_stream(events, args.history_ratio, &default_fields())
        .context("evaluate held-out stream")?;
    let json = serde_json::to_string_pretty(&result).context("serialize metrics JSON")?;

    if let Some(output_path) = args.output {
        fs::write(&output_path, format!("{json}\n"))
            .with_context(|| format!("write metrics to {}", output_path.display()))?;
        eprintln!("wrote metrics to {}", output_path.display());
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(json.as_bytes())?;
        stdout.write_all(b"\n")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_rejects_out_of_range_history_ratio() {
        let err =
            parse_args_from(vec!["--history-ratio".to_string(), "1.2".to_string()]).unwrap_err();
        assert!(err.to_string().contains("between 0.1 and 0.9"));
    }
}
