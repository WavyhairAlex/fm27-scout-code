use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use fm27_core::{
    container::SaveFile, editor::PlayerEditor, inspect_save, layout::BUILD,
    metadata::read_metadata, parse_save, store::Store,
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

            let request = EditRequest {
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
                    "file_size": report.file_size,
                    "compressed_delta": report.compressed_delta,
                    "changes": editor.changes(),
                }))?
            );
        }
    }
    Ok(())
}

fn parse_assignments(values: &[String]) -> Result<Vec<(String, u8)>> {
    values
        .iter()
        .map(|value| {
            let (key, raw) = value
                .split_once('=')
                .ok_or_else(|| anyhow!("expected KEY=VALUE, got {value:?}"))?;
            let parsed = raw
                .parse::<u8>()
                .with_context(|| format!("parse value for {key:?} in {value:?}"))?;
            Ok((key.to_owned(), parsed))
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

#[derive(Debug, Default)]
struct EditRequest {
    ca: Option<u16>,
    pa: Option<i16>,
    attributes: Vec<(String, u8)>,
    personality: Vec<(String, u8)>,
    traits: Vec<u8>,
    untraits: Vec<u8>,
}

impl EditRequest {
    fn is_empty(&self) -> bool {
        self.ca.is_none()
            && self.pa.is_none()
            && self.attributes.is_empty()
            && self.personality.is_empty()
            && self.traits.is_empty()
            && self.untraits.is_empty()
    }

    fn apply(&self, editor: &mut PlayerEditor, uid: u32) -> Result<()> {
        if let Some(value) = self.ca {
            editor.set_ca(uid, value)?;
        }
        if let Some(value) = self.pa {
            editor.set_pa(uid, value)?;
        }
        for (key, value) in &self.attributes {
            editor.set_attribute(uid, key, *value)?;
        }
        for (key, value) in &self.personality {
            editor.set_personality(uid, key, *value)?;
        }
        for bit in &self.traits {
            editor.set_trait(uid, *bit, true)?;
        }
        for bit in &self.untraits {
            editor.set_trait(uid, *bit, false)?;
        }
        Ok(())
    }

    fn verify(&self, player: &fm27_core::Player) -> Result<()> {
        if let Some(expected) = self.ca {
            if player.ca != expected {
                bail!(
                    "CA verification failed: expected {expected}, got {}",
                    player.ca
                );
            }
        }
        if let Some(expected) = self.pa {
            if player.pa != Some(expected) {
                bail!(
                    "PA verification failed: expected {expected}, got {:?}",
                    player.pa
                );
            }
        }
        for (key, expected) in &self.attributes {
            let actual = player.raw_attributes.get(key);
            if actual != Some(*expected) {
                bail!("attribute {key} verification failed: expected {expected}, got {actual:?}");
            }
        }
        for (key, expected) in &self.personality {
            let actual = personality_value(player, key);
            if actual != Some(*expected) {
                bail!("personality {key} verification failed: expected {expected}, got {actual:?}");
            }
        }
        for bit in &self.traits {
            if player
                .trait_bits
                .is_none_or(|bits| bits & (1u64 << *bit) == 0)
            {
                bail!("trait {bit} verification failed: it is not set");
            }
        }
        for bit in &self.untraits {
            if player
                .trait_bits
                .is_some_and(|bits| bits & (1u64 << *bit) != 0)
            {
                bail!("trait {bit} verification failed: it is still set");
            }
        }
        Ok(())
    }
}

fn verify_edited_save(path: &Path, uid: u32, request: &EditRequest) -> Result<()> {
    let parsed = parse_save(path).context("re-parse edited save")?;
    let player = parsed
        .players
        .iter()
        .find(|player| player.uid == uid)
        .ok_or_else(|| anyhow!("edited save no longer contains player uid {uid}"))?;
    request.verify(player)
}

fn personality_value(player: &fm27_core::Player, key: &str) -> Option<u8> {
    let personality = player.personality?;
    match key {
        "adaptability" => Some(personality.adaptability),
        "ambition" => Some(personality.ambition),
        "loyalty" => Some(personality.loyalty),
        "pressure" => Some(personality.pressure),
        "professionalism" => Some(personality.professionalism),
        "sportsmanship" => Some(personality.sportsmanship),
        "temperament" => Some(personality.temperament),
        "controversy" => Some(personality.controversy),
        _ => None,
    }
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
            vec![("crossing".to_owned(), 100), ("consistency".to_owned(), 85)]
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
}
