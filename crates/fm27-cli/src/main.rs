use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use fm27_core::{
    club_editor::{ClubEditRequest, ClubEditor, ClubFinanceEdit, StadiumCapacityEdit},
    container::SaveFile,
    editor::{AttributeEdit, PlayerEditRequest, PlayerEditor},
    inspect_save,
    layout::BUILD,
    metadata::read_metadata,
    parse_save,
    store::Store,
};

#[derive(Parser)]
#[command(
    name = "fm27scout",
    version,
    about = "Football Manager 27 save inspector and experimental editor"
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
        /// Include monthly finances, corporate facilities and stadiums.
        #[arg(long)]
        details: bool,
    },
    /// Parse a save and print the competition index as JSON.
    Competitions {
        save: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Parse a save and print the parse report.
    Validate { save: PathBuf },
    /// Edit one player in a copy of a save. The source save is never modified.
    Edit {
        /// Source .fm save. It is opened read-only.
        save: PathBuf,
        /// Player uid from `fm27scout players` or the SQLite cache.
        #[arg(long)]
        uid: u32,
        /// Current ability (1-200).
        #[arg(long)]
        ca: Option<u16>,
        /// Potential ability (1-200). Negative random-range codes are not supported.
        #[arg(long)]
        pa: Option<i16>,
        /// Repeatable raw attribute (1-100): `crossing=100`, `consistency=100`.
        #[arg(long = "attr", value_name = "KEY=VALUE")]
        attributes: Vec<String>,
        /// Repeatable personality byte (1-20): `professionalism=20`.
        #[arg(long, value_name = "KEY=VALUE")]
        personality: Vec<String>,
        /// Add a known trait bit (0-63).
        #[arg(long = "trait", value_name = "BIT")]
        traits: Vec<u8>,
        /// Remove a trait bit (0-63).
        #[arg(long = "untrait", value_name = "BIT")]
        untraits: Vec<u8>,
        /// Output path. Defaults to `<save stem>.edited.fm` next to the source.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Internal save name stored in the new file, for test copies.
        #[arg(long)]
        save_name: Option<String>,
        /// Overwrite the output file if it already exists.
        #[arg(long)]
        force: bool,
        /// Print the planned changes without writing a save.
        #[arg(long)]
        dry_run: bool,
        /// zstd compression level for the rebuilt game_db (1-22).
        #[arg(long, default_value_t = 3)]
        compression_level: i32,
    },
    /// Edit low-risk club fields in a copy of a save.
    EditClub {
        /// Source .fm save. It is opened read-only.
        save: PathBuf,
        /// Club uid from `fm27scout clubs`.
        #[arg(long)]
        club_uid: u32,
        /// Club reputation (1-10000).
        #[arg(long)]
        reputation: Option<u16>,
        /// Corporate facilities rating (1-20). Only clubs with a finance chain have one.
        #[arg(long)]
        corporate_facilities: Option<u8>,
        /// Training facilities rating (1-20).
        #[arg(long)]
        training_facilities: Option<u8>,
        /// Youth facilities rating (1-20).
        #[arg(long)]
        youth_facilities: Option<u8>,
        /// Junior coaching rating (1-20).
        #[arg(long)]
        junior_coaching: Option<u8>,
        /// Youth recruitment rating (1-20).
        #[arg(long)]
        youth_recruitment: Option<u8>,
        /// Latest-month club balance.
        #[arg(long)]
        balance: Option<i64>,
        /// Latest-month transfer budget remaining.
        #[arg(long)]
        transfer_budget: Option<i64>,
        /// Latest-month weekly wage budget.
        #[arg(long)]
        wage_budget: Option<i64>,
        /// Stadium uid to edit. Must be owned by --club-uid.
        #[arg(long)]
        stadium_uid: Option<u32>,
        /// All-seater capacity for --stadium-uid.
        #[arg(long)]
        all_seater: Option<u32>,
        /// Expansion capacity for --stadium-uid.
        #[arg(long)]
        expansion: Option<u32>,
        /// Output path. Defaults to `<save stem>.edited.fm` next to the source.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Internal save name stored in the new file, for test copies.
        #[arg(long)]
        save_name: Option<String>,
        /// Overwrite the output file if it already exists.
        #[arg(long)]
        force: bool,
        /// Print the planned changes without writing a save.
        #[arg(long)]
        dry_run: bool,
        /// zstd compression level for the rebuilt game_db (1-22).
        #[arg(long, default_value_t = 3)]
        compression_level: i32,
    },
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
        Command::Clubs {
            save,
            output,
            details,
        } => {
            let parsed = parse_save(&save)?;
            let mut writer: Box<dyn Write> = match &output {
                Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
                None => Box::new(BufWriter::new(io::stdout().lock())),
            };
            if details {
                serde_json::to_writer(
                    &mut writer,
                    &serde_json::json!({
                        "clubs": parsed.clubs,
                        "club_finances": parsed.club_finances,
                        "stadiums": parsed.stadiums,
                    }),
                )?;
            } else {
                serde_json::to_writer(&mut writer, &parsed.clubs)?;
            }
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
        Command::Edit {
            save,
            uid,
            ca,
            pa,
            attributes,
            personality,
            traits,
            untraits,
            output,
            save_name,
            force,
            dry_run,
            compression_level,
        } => {
            if !(1..=22).contains(&compression_level) {
                bail!("--compression-level must be between 1 and 22");
            }
            let save_file =
                SaveFile::open(&save).with_context(|| format!("open {}", save.display()))?;
            let metadata = read_metadata(&save_file).context("read save metadata")?;
            if metadata.build != BUILD {
                bail!(
                    "editing supports build {BUILD}; this save was written by {}",
                    metadata.build
                );
            }
            let mut editor =
                PlayerEditor::open(&save_file, &metadata).context("open player editor")?;

            let request = PlayerEditRequest {
                ca,
                pa,
                attributes: parse_assignments(&attributes)?,
                personality: parse_assignments(&personality)?,
                traits,
                untraits,
            };
            if request.is_empty() {
                bail!(
                    "no edits requested; pass --ca, --pa, --attr, --personality, --trait or --untrait"
                );
            }
            let warnings = request.warnings();
            for warning in &warnings {
                eprintln!("warning: {warning}");
            }
            request.apply(&mut editor, uid)?;
            editor.validate()?;

            let output = output.unwrap_or_else(|| default_edit_output(&save));
            if output == save {
                bail!("refusing to overwrite the source save; choose a different --output");
            }
            if output.exists() && !force {
                bail!(
                    "output {} already exists; pass --force to overwrite it",
                    output.display()
                );
            }
            if dry_run {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "uid": uid,
                        "source": save.display().to_string(),
                        "output": output.display().to_string(),
                        "save_name": save_name,
                        "written": false,
                        "warnings": warnings,
                        "changes": editor.changes(),
                    }))?
                );
                return Ok(());
            }

            let report = editor
                .write_as(&save_file, &output, compression_level, save_name.as_deref())
                .context("write edited save")?;
            let verify = verify_edited_save(&output, uid, &request);
            if let Err(error) = verify {
                let _ = std::fs::remove_file(&output);
                return Err(error).context("verify edited save");
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "uid": uid,
                    "source": save.display().to_string(),
                    "output": report.output,
                    "save_name": save_name,
                    "written": true,
                    "verified": true,
                    "warnings": warnings,
                    "file_size": report.file_size,
                    "compressed_delta": report.compressed_delta,
                    "changes": editor.changes(),
                }))?
            );
        }
        Command::EditClub {
            save,
            club_uid,
            reputation,
            corporate_facilities,
            training_facilities,
            youth_facilities,
            junior_coaching,
            youth_recruitment,
            balance,
            transfer_budget,
            wage_budget,
            stadium_uid,
            all_seater,
            expansion,
            output,
            save_name,
            force,
            dry_run,
            compression_level,
        } => {
            if !(1..=22).contains(&compression_level) {
                bail!("--compression-level must be between 1 and 22");
            }
            if stadium_uid.is_none() && (all_seater.is_some() || expansion.is_some()) {
                bail!("--all-seater and --expansion require --stadium-uid");
            }
            let save_file =
                SaveFile::open(&save).with_context(|| format!("open {}", save.display()))?;
            let metadata = read_metadata(&save_file).context("read save metadata")?;
            let mut editor = ClubEditor::open(&save_file, &metadata).context("open club editor")?;
            let finance = ClubFinanceEdit {
                balance,
                transfer_budget_remaining: transfer_budget,
                wage_budget_weekly: wage_budget,
            };
            let request = ClubEditRequest {
                reputation,
                corporate_facilities,
                training_facilities,
                youth_facilities,
                junior_coaching,
                youth_recruitment,
                finances: (!finance.is_empty()).then_some(finance),
                stadiums: stadium_uid
                    .map(|stadium_uid| StadiumCapacityEdit {
                        stadium_uid,
                        all_seater_capacity: all_seater,
                        expansion_capacity: expansion,
                    })
                    .into_iter()
                    .collect(),
            };
            if request.is_empty() {
                bail!(
                    "no edits requested; pass --reputation, --corporate-facilities, --training-facilities, --youth-facilities, --junior-coaching, --youth-recruitment, --balance, --transfer-budget, --wage-budget or --stadium-uid"
                );
            }
            let warnings = request.warnings();
            for warning in &warnings {
                eprintln!("warning: {warning}");
            }
            request.apply(&mut editor, club_uid)?;
            editor.validate()?;

            let output = output.unwrap_or_else(|| default_edit_output(&save));
            if output == save {
                bail!("refusing to overwrite the source save; choose a different --output");
            }
            if output.exists() && !force {
                bail!(
                    "output {} already exists; pass --force to overwrite it",
                    output.display()
                );
            }
            if dry_run {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "club_uid": club_uid,
                        "source": save.display().to_string(),
                        "output": output.display().to_string(),
                        "save_name": save_name,
                        "written": false,
                        "warnings": warnings,
                        "changes": editor.changes(),
                    }))?
                );
                return Ok(());
            }

            let report = editor
                .write_as(&save_file, &output, compression_level, save_name.as_deref())
                .context("write edited save")?;
            let verify = verify_edited_club(&output, club_uid, &request);
            if let Err(error) = verify {
                let _ = std::fs::remove_file(&output);
                return Err(error).context("verify edited save");
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "club_uid": club_uid,
                    "source": save.display().to_string(),
                    "output": report.output,
                    "save_name": save_name,
                    "written": true,
                    "verified": true,
                    "warnings": warnings,
                    "file_size": report.file_size,
                    "compressed_delta": report.compressed_delta,
                    "changes": editor.changes(),
                }))?
            );
        }
    }
    Ok(())
}

fn parse_assignments(values: &[String]) -> Result<Vec<AttributeEdit>> {
    values
        .iter()
        .map(|value| {
            let (key, raw) = value
                .split_once('=')
                .ok_or_else(|| anyhow!("expected KEY=VALUE, got {value:?}"))?;
            let value = raw
                .parse::<u8>()
                .with_context(|| format!("parse value for {key:?} in {value:?}"))?;
            Ok(AttributeEdit {
                key: key.to_owned(),
                value,
            })
        })
        .collect()
}

fn default_edit_output(source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "edited".to_owned());
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}.edited.fm"))
}

fn verify_edited_save(path: &Path, uid: u32, request: &PlayerEditRequest) -> Result<()> {
    let parsed = parse_save(path).context("re-parse edited save")?;
    let player = parsed
        .players
        .iter()
        .find(|player| player.uid == uid)
        .ok_or_else(|| anyhow!("edited save no longer contains player uid {uid}"))?;
    request.verify(player).map_err(Into::into)
}

fn verify_edited_club(path: &Path, club_uid: u32, request: &ClubEditRequest) -> Result<()> {
    let parsed = parse_save(path).context("re-parse edited save")?;
    request.verify(&parsed, club_uid).map_err(Into::into)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repeated_key_value_edits() {
        let values = vec!["crossing=100".to_owned(), "consistency=85".to_owned()];
        assert_eq!(
            parse_assignments(&values).unwrap(),
            vec![
                AttributeEdit {
                    key: "crossing".to_owned(),
                    value: 100
                },
                AttributeEdit {
                    key: "consistency".to_owned(),
                    value: 85
                }
            ]
        );
    }

    #[test]
    fn rejects_a_missing_equals_sign() {
        let error = parse_assignments(&["crossing".to_owned()]).unwrap_err();
        assert!(error.to_string().contains("KEY=VALUE"));
    }

    #[test]
    fn default_output_keeps_the_source_directory() {
        assert_eq!(
            default_edit_output(Path::new("/tmp/save.fm")),
            PathBuf::from("/tmp/save.edited.fm")
        );
    }

    #[test]
    fn warns_about_ca_rebalancing() {
        let request = PlayerEditRequest {
            ca: Some(150),
            ..PlayerEditRequest::default()
        };
        assert!(
            request
                .warnings()
                .iter()
                .any(|warning| warning.contains("recalculates visible attributes"))
        );
    }

    #[test]
    fn warns_about_visible_attributes_without_ca() {
        let request = PlayerEditRequest {
            attributes: vec![AttributeEdit {
                key: "crossing".to_owned(),
                value: 100,
            }],
            ..PlayerEditRequest::default()
        };
        assert!(
            request
                .warnings()
                .iter()
                .any(|warning| warning.contains("without --ca"))
        );
    }

    #[test]
    fn hidden_attribute_edits_do_not_warn() {
        let request = PlayerEditRequest {
            attributes: vec![AttributeEdit {
                key: "consistency".to_owned(),
                value: 100,
            }],
            ..PlayerEditRequest::default()
        };
        assert!(request.warnings().is_empty());
    }

    #[test]
    fn determination_edits_do_not_warn() {
        let request = PlayerEditRequest {
            attributes: vec![AttributeEdit {
                key: "determination".to_owned(),
                value: 100,
            }],
            ..PlayerEditRequest::default()
        };
        assert!(request.warnings().is_empty());
    }

    #[test]
    fn natural_fitness_edits_do_not_warn() {
        let request = PlayerEditRequest {
            attributes: vec![AttributeEdit {
                key: "natural_fitness".to_owned(),
                value: 100,
            }],
            ..PlayerEditRequest::default()
        };
        assert!(request.warnings().is_empty());
    }

    #[test]
    fn flair_and_aggression_edits_do_not_warn() {
        for key in ["flair", "aggression"] {
            let request = PlayerEditRequest {
                attributes: vec![AttributeEdit {
                    key: key.to_owned(),
                    value: 100,
                }],
                ..PlayerEditRequest::default()
            };
            assert!(request.warnings().is_empty(), "{key} should be exempt");
        }
    }
}
