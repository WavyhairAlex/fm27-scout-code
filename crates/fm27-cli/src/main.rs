use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fm27_core::{inspect_save, parse_save, store::Store};

#[derive(Parser)]
#[command(
    name = "fm27scout",
    version,
    about = "Read-only Football Manager 27 save inspector"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show container metadata without parsing players.
    Inspect {
        /// Path to a .fm save.
        save: PathBuf,
    },
    /// Parse a save and print players as JSON or CSV.
    Players {
        save: PathBuf,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Parse a save into a local SQLite cache.
    Index {
        save: PathBuf,
        #[arg(long, default_value = "fm27scout.db")]
        db: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Export players to CSV or JSON.
    Export {
        save: PathBuf,
        #[arg(long, default_value = "csv")]
        format: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 50000)]
        limit: usize,
    },
    /// Parse a save and print clubs as JSON.
    Clubs {
        save: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Parse a save and print the competition index as JSON.
    Competitions {
        save: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Parse a save and print the parse report.
    Validate { save: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { save } => {
            let summary = inspect_save(&save)?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Players {
            save,
            limit,
            format,
            output,
        } => {
            let parsed = parse_save(&save)?;
            let selected: Vec<_> = parsed.players.iter().take(limit).collect();
            let mut writer: Box<dyn Write> = match &output {
                Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
                None => Box::new(BufWriter::new(io::stdout().lock())),
            };
            match format.as_str() {
                "json" => serde_json::to_writer(&mut writer, &selected)?,
                "csv" => write_csv(&mut writer, &selected)?,
                other => anyhow::bail!("unknown format {other:?}; expected json or csv"),
            }
            writer.flush()?;
            if let Some(path) = output {
                eprintln!("wrote {}", path.display());
            } else {
                println!();
            }
        }
        Command::Index { save, db, force } => {
            let started = std::time::Instant::now();
            let parsed = parse_save(&save)?;
            let mut store = Store::open(&db)?;
            if !force {
                if let Some(id) =
                    store.find_cached(&parsed.summary.fingerprint, fm27_core::parser_version())?
                {
                    println!("cached save_id={id}");
                    return Ok(());
                }
            }
            let id = store.insert_parsed(&parsed)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "save_id": id,
                    "db": db,
                    "players": parsed.players.len(),
                    "clubs": parsed.clubs.len(),
                    "contracts": parsed.report.contracts,
                    "competitions": parsed.report.competitions,
                    "competition_stages": parsed.report.competition_stages,
                    "elapsed_ms": started.elapsed().as_millis(),
                }))?
            );
        }
        Command::Export {
            save,
            format,
            output,
            limit,
        } => {
            let parsed = parse_save(&save)?;
            let selected: Vec<_> = parsed.players.iter().take(limit).collect();
            let file = BufWriter::new(std::fs::File::create(&output)?);
            let mut writer: Box<dyn Write> = Box::new(file);
            match format.as_str() {
                "json" => serde_json::to_writer_pretty(&mut writer, &selected)?,
                "csv" => write_csv(&mut writer, &selected)?,
                other => anyhow::bail!("unknown format {other:?}; expected json or csv"),
            }
            writer.flush()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "path": output,
                    "format": format,
                    "rows": selected.len(),
                }))?
            );
        }
        Command::Clubs { save, output } => {
            let parsed = parse_save(&save)?;
            let mut writer: Box<dyn Write> = match &output {
                Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
                None => Box::new(BufWriter::new(io::stdout().lock())),
            };
            serde_json::to_writer(&mut writer, &parsed.clubs)?;
            writer.flush()?;
            if output.is_some() {
                println!();
            }
        }
        Command::Competitions { save, output } => {
            let parsed = parse_save(&save)?;
            let mut writer: Box<dyn Write> = match &output {
                Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
                None => Box::new(BufWriter::new(io::stdout().lock())),
            };
            serde_json::to_writer(&mut writer, &parsed.competitions)?;
            writer.flush()?;
            if output.is_some() {
                println!();
            }
        }
        Command::Validate { save } => {
            let parsed = parse_save(&save).context("parse save")?;
            println!("{}", serde_json::to_string_pretty(&parsed.report)?);
        }
    }
    Ok(())
}

fn write_csv(writer: &mut dyn Write, players: &[&fm27_core::Player]) -> Result<()> {
    let mut csv = csv::Writer::from_writer(writer);
    csv.write_record([
        "uid",
        "unique_id",
        "name",
        "age",
        "club",
        "nation_id",
        "positions",
        "ca",
        "pa",
        "value",
        "wage",
        "contract_end",
    ])?;
    for player in players {
        let wage = player
            .contract
            .as_ref()
            .and_then(|contract| contract.wage)
            .map(|value| value.to_string())
            .unwrap_or_default();
        let end = player
            .contract
            .as_ref()
            .and_then(|contract| contract.end.clone())
            .unwrap_or_default();
        csv.write_record([
            player.uid.to_string(),
            player
                .unique_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            player.name.clone().unwrap_or_default(),
            player
                .age
                .map(|value| value.to_string())
                .unwrap_or_default(),
            player.club_name.clone().unwrap_or_default(),
            player
                .nation_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            player.natural_positions.join(" / "),
            player.ca.to_string(),
            player.pa.map(|value| value.to_string()).unwrap_or_default(),
            player
                .transfer_value
                .map(|value| value.to_string())
                .unwrap_or_default(),
            wage,
            end,
        ])?;
    }
    csv.flush()?;
    Ok(())
}
