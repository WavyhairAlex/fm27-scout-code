//! CSV and JSON export of the current player query.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::layout::{STAFF_ATTRIBUTE_KEYS, STAFF_ROLE_KEYS};
use crate::store::{PlayerRow, StaffRow};

#[derive(Debug, Clone, Serialize)]
pub struct ExportReport {
    pub path: String,
    pub format: String,
    pub rows: usize,
}

/// Streaming CSV/JSON writer. Rows are written as they are read from SQLite, so
/// exports do not need to materialize the complete result set in memory.
pub struct ExportWriter {
    path: String,
    format: String,
    rows: usize,
    destination: Destination,
}

#[allow(clippy::large_enum_variant)]
enum Destination {
    Csv(csv::Writer<BufWriter<File>>),
    Json(BufWriter<File>),
}

impl ExportWriter {
    pub fn create(path: &Path, format: &str) -> Result<Self> {
        let format = format.to_ascii_lowercase();
        let destination = match format.as_str() {
            "csv" => {
                let mut writer = csv::Writer::from_writer(BufWriter::new(File::create(path)?));
                writer.write_record(csv_header())?;
                Destination::Csv(writer)
            }
            "json" => {
                let mut writer = BufWriter::new(File::create(path)?);
                writer.write_all(b"[")?;
                Destination::Json(writer)
            }
            other => {
                return Err(Error::parse(format!(
                    "unknown export format {other:?}; expected csv or json"
                )));
            }
        };
        Ok(Self {
            path: path.display().to_string(),
            format,
            rows: 0,
            destination,
        })
    }

    pub fn write_row(&mut self, row: &PlayerRow) -> Result<()> {
        match &mut self.destination {
            Destination::Csv(writer) => writer.write_record(csv_record(row))?,
            Destination::Json(writer) => {
                if self.rows == 0 {
                    writer.write_all(b"\n")?;
                } else {
                    writer.write_all(b",\n")?;
                }
                write_indented_json(writer, row)?;
            }
        }
        self.rows += 1;
        Ok(())
    }

    pub fn finish(mut self) -> Result<ExportReport> {
        match &mut self.destination {
            Destination::Csv(writer) => writer.flush()?,
            Destination::Json(writer) => {
                if self.rows == 0 {
                    writer.write_all(b"]")?;
                } else {
                    writer.write_all(b"\n]")?;
                }
                writer.flush()?;
            }
        }
        Ok(ExportReport {
            path: self.path,
            format: self.format,
            rows: self.rows,
        })
    }
}

pub fn write_export(path: &Path, format: &str, rows: &[PlayerRow]) -> Result<ExportReport> {
    let mut writer = ExportWriter::create(path, format)?;
    for row in rows {
        writer.write_row(row)?;
    }
    writer.finish()
}

/// Streaming CSV/JSON writer for the staff directory.
pub struct StaffExportWriter {
    path: String,
    format: String,
    rows: usize,
    destination: Destination,
}

impl StaffExportWriter {
    pub fn create(path: &Path, format: &str) -> Result<Self> {
        let format = format.to_ascii_lowercase();
        let destination = match format.as_str() {
            "csv" => {
                let mut writer = csv::Writer::from_writer(BufWriter::new(File::create(path)?));
                writer.write_record(staff_csv_header())?;
                Destination::Csv(writer)
            }
            "json" => {
                let mut writer = BufWriter::new(File::create(path)?);
                writer.write_all(b"[")?;
                Destination::Json(writer)
            }
            other => {
                return Err(Error::parse(format!(
                    "unknown export format {other:?}; expected csv or json"
                )));
            }
        };
        Ok(Self {
            path: path.display().to_string(),
            format,
            rows: 0,
            destination,
        })
    }

    pub fn write_row(&mut self, row: &StaffRow) -> Result<()> {
        match &mut self.destination {
            Destination::Csv(writer) => writer.write_record(staff_csv_record(row))?,
            Destination::Json(writer) => {
                if self.rows == 0 {
                    writer.write_all(b"\n")?;
                } else {
                    writer.write_all(b",\n")?;
                }
                write_indented_json(writer, row)?;
            }
        }
        self.rows += 1;
        Ok(())
    }

    pub fn finish(mut self) -> Result<ExportReport> {
        match &mut self.destination {
            Destination::Csv(writer) => writer.flush()?,
            Destination::Json(writer) => {
                if self.rows == 0 {
                    writer.write_all(b"]")?;
                } else {
                    writer.write_all(b"\n]")?;
                }
                writer.flush()?;
            }
        }
        Ok(ExportReport {
            path: self.path,
            format: self.format,
            rows: self.rows,
        })
    }
}

fn write_indented_json<T: Serialize>(writer: &mut BufWriter<File>, row: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(row)?;
    for (index, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        if index > 0 {
            writer.write_all(b"\n")?;
        }
        writer.write_all(b"  ")?;
        writer.write_all(line)?;
    }
    Ok(())
}

fn csv_header() -> Vec<String> {
    let mut header = vec![
        "uid".to_owned(),
        "generated".to_owned(),
        "name".to_owned(),
        "age".to_owned(),
        "club".to_owned(),
        "nation_id".to_owned(),
        "nation_name".to_owned(),
        "gender".to_owned(),
        "positions".to_owned(),
        "best_role".to_owned(),
        "best_role_score".to_owned(),
        "ca".to_owned(),
        "pa".to_owned(),
        "value".to_owned(),
        "wage".to_owned(),
        "contract_end".to_owned(),
        "height_cm".to_owned(),
        "left_foot".to_owned(),
        "right_foot".to_owned(),
        "on_loan".to_owned(),
        "transfer_listed".to_owned(),
        "loan_listed".to_owned(),
    ];
    for key in crate::layout::ATTRIBUTE_KEYS {
        header.push(key.to_owned());
    }
    header
}

fn csv_record(row: &PlayerRow) -> Vec<String> {
    let mut record = vec![
        row.uid.to_string(),
        if row.generated { "yes" } else { "no" }.to_owned(),
        row.name.clone().unwrap_or_default(),
        row.age.map(|value| value.to_string()).unwrap_or_default(),
        row.club_name.clone().unwrap_or_default(),
        row.nation_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.nation_name.clone().unwrap_or_default(),
        row.gender
            .map(|value| if value == 1 { "women" } else { "men" }.to_owned())
            .unwrap_or_default(),
        row.positions_text.clone().unwrap_or_default(),
        row.best_role_name.clone().unwrap_or_default(),
        row.best_role_score
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.ca.to_string(),
        row.pa.map(|value| value.to_string()).unwrap_or_default(),
        row.transfer_value
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.wage.map(|value| value.to_string()).unwrap_or_default(),
        row.contract_end.clone().unwrap_or_default(),
        row.height_cm
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.left_foot
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.right_foot
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.on_loan
            .map(|value| if value { "yes" } else { "no" })
            .unwrap_or_default()
            .to_owned(),
        if row.transfer_listed { "yes" } else { "no" }.to_owned(),
        if row.loan_listed { "yes" } else { "no" }.to_owned(),
    ];
    for value in row.attributes.values {
        record.push(value.to_string());
    }
    record
}

fn staff_csv_header() -> Vec<String> {
    let mut header = vec![
        "uid".to_owned(),
        "unique_id".to_owned(),
        "name".to_owned(),
        "age".to_owned(),
        "club".to_owned(),
        "nation_id".to_owned(),
        "nation_name".to_owned(),
        "job".to_owned(),
        "ca".to_owned(),
        "pa".to_owned(),
        "wage".to_owned(),
        "contract_start".to_owned(),
        "contract_end".to_owned(),
        "has_contract".to_owned(),
        "best_role".to_owned(),
        "best_role_score".to_owned(),
    ];
    for key in STAFF_ATTRIBUTE_KEYS {
        header.push(key.to_owned());
    }
    for key in STAFF_ROLE_KEYS {
        header.push(key.to_owned());
    }
    header
}

fn staff_csv_record(row: &StaffRow) -> Vec<String> {
    let mut record = vec![
        row.uid.to_string(),
        row.unique_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.name.clone().unwrap_or_default(),
        row.age.map(|value| value.to_string()).unwrap_or_default(),
        row.club_name.clone().unwrap_or_default(),
        row.nation_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.nation_name.clone().unwrap_or_default(),
        row.job_name.clone().unwrap_or_default(),
        row.ca.map(|value| value.to_string()).unwrap_or_default(),
        row.pa.map(|value| value.to_string()).unwrap_or_default(),
        row.wage.map(|value| value.to_string()).unwrap_or_default(),
        row.contract_start.clone().unwrap_or_default(),
        row.contract_end.clone().unwrap_or_default(),
        if row.has_contract { "yes" } else { "no" }.to_owned(),
        row.best_role_name.clone().unwrap_or_default(),
        row.best_role_score
            .map(|value| value.to_string())
            .unwrap_or_default(),
    ];
    for key in STAFF_ATTRIBUTE_KEYS {
        record.push(
            row.attributes
                .as_ref()
                .and_then(|attributes| attributes.get(key))
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }
    for (index, _) in STAFF_ROLE_KEYS.iter().enumerate() {
        record.push(
            row.roles
                .as_ref()
                .and_then(|roles| roles.get(index))
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }
    record
}

/// Streaming CSV/JSON writer for the stats-based moneyball rows.
pub struct StatsExportWriter {
    path: String,
    format: String,
    rows: usize,
    destination: Destination,
}

impl StatsExportWriter {
    pub fn create(path: &Path, format: &str) -> Result<Self> {
        let format = format.to_ascii_lowercase();
        let destination = match format.as_str() {
            "csv" => {
                let mut writer = csv::Writer::from_writer(BufWriter::new(File::create(path)?));
                writer.write_record(stats_csv_header())?;
                Destination::Csv(writer)
            }
            "json" => {
                let mut writer = BufWriter::new(File::create(path)?);
                writer.write_all(b"[")?;
                Destination::Json(writer)
            }
            other => {
                return Err(Error::parse(format!(
                    "unknown export format {other:?}; expected csv or json"
                )));
            }
        };
        Ok(Self {
            path: path.display().to_string(),
            format,
            rows: 0,
            destination,
        })
    }

    pub fn write_row(&mut self, row: &crate::stats_moneyball::StatsMoneyballRow) -> Result<()> {
        match &mut self.destination {
            Destination::Csv(writer) => writer.write_record(stats_csv_record(row))?,
            Destination::Json(writer) => {
                if self.rows == 0 {
                    writer.write_all(b"\n")?;
                } else {
                    writer.write_all(b",\n")?;
                }
                write_indented_json(writer, row)?;
            }
        }
        self.rows += 1;
        Ok(())
    }

    pub fn finish(mut self) -> Result<ExportReport> {
        match &mut self.destination {
            Destination::Csv(writer) => writer.flush()?,
            Destination::Json(writer) => {
                if self.rows == 0 {
                    writer.write_all(b"]")?;
                } else {
                    writer.write_all(b"\n]")?;
                }
                writer.flush()?;
            }
        }
        Ok(ExportReport {
            path: self.path,
            format: self.format,
            rows: self.rows,
        })
    }
}

pub fn write_stats_export(
    path: &Path,
    format: &str,
    rows: &[crate::stats_moneyball::StatsMoneyballRow],
) -> Result<ExportReport> {
    let mut writer = StatsExportWriter::create(path, format)?;
    for row in rows {
        writer.write_row(row)?;
    }
    writer.finish()
}

fn stats_csv_header() -> Vec<String> {
    [
        "uid",
        "name",
        "age",
        "club",
        "league",
        "nation",
        "position_group",
        "minutes",
        "starts",
        "substitute_appearances",
        "ca",
        "pa",
        "quality",
        "quality_low",
        "production_global",
        "fee",
        "cost",
        "fair_price",
        "value_gap_pct",
        "bargain_score",
        "confidence",
        "goals",
        "assists",
        "expected_goals",
        "non_penalty_expected_goals",
        "expected_assists",
        "shots",
        "shots_on_target",
        "passes_attempted",
        "passes_completed",
        "pass_completion_percent",
        "tackles_attempted",
        "tackles_completed",
        "interceptions",
        "key_passes",
        "progressive_passes",
        "headers_won",
        "aerial_challenges_attempted",
        "clearances",
        "blocks",
        "crosses_completed",
        "saves_held",
        "shots_on_target_faced",
        "save_percent",
        "expected_goals_prevented",
        "transfer_value",
        "transfer_value_state",
        "wage",
        "contract_end",
        "advanced_available",
        "transfer_listed",
        "loan_listed",
        "injury_status",
        "recent_injury",
        "stats_from_other_club",
        "flags",
        "drivers",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn stats_csv_record(row: &crate::stats_moneyball::StatsMoneyballRow) -> Vec<String> {
    let opt_u32 = |value: Option<u32>| value.map(|value| value.to_string()).unwrap_or_default();
    let opt_f64 = |value: Option<f64>| value.map(|value| value.to_string()).unwrap_or_default();
    let yes_no = |value: bool| if value { "yes" } else { "no" }.to_owned();
    let drivers = row
        .drivers
        .iter()
        .map(|driver| format!("{}:{:.1}", driver.key, driver.percentile))
        .collect::<Vec<_>>()
        .join("|");
    vec![
        row.uid.to_string(),
        row.name.clone().unwrap_or_default(),
        row.age.map(|value| value.to_string()).unwrap_or_default(),
        row.club_name.clone().unwrap_or_default(),
        row.league_name.clone().unwrap_or_default(),
        row.nation_name.clone().unwrap_or_default(),
        row.position_group.clone(),
        row.minutes.to_string(),
        row.starts.to_string(),
        row.substitute_appearances.to_string(),
        row.ca.to_string(),
        row.pa.map(|value| value.to_string()).unwrap_or_default(),
        opt_f64(row.score),
        opt_f64(row.score_low),
        opt_f64(row.production_global),
        row.fee.map(|value| value.to_string()).unwrap_or_default(),
        row.cost.to_string(),
        opt_f64(row.fair_price),
        opt_f64(row.value_gap_pct),
        opt_f64(row.bargain_score),
        row.confidence.to_string(),
        row.goals.to_string(),
        row.assists.to_string(),
        row.expected_goals.to_string(),
        row.non_penalty_expected_goals.to_string(),
        row.expected_assists.to_string(),
        row.shots.to_string(),
        row.shots_on_target.to_string(),
        row.passes_attempted.to_string(),
        row.passes_completed.to_string(),
        opt_f64(row.pass_completion_percent),
        row.tackles_attempted.to_string(),
        row.tackles_completed.to_string(),
        opt_u32(row.interceptions),
        opt_u32(row.key_passes),
        opt_u32(row.progressive_passes),
        opt_u32(row.headers_won),
        opt_u32(row.aerial_challenges_attempted),
        opt_u32(row.clearances),
        opt_u32(row.blocks),
        opt_u32(row.crosses_completed),
        opt_u32(row.saves_held),
        opt_u32(row.shots_on_target_faced),
        opt_f64(row.save_percent),
        opt_f64(row.expected_goals_prevented),
        row.transfer_value
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.transfer_value_state.clone(),
        row.wage.map(|value| value.to_string()).unwrap_or_default(),
        row.contract_end.clone().unwrap_or_default(),
        yes_no(row.advanced_available),
        yes_no(row.transfer_listed),
        yes_no(row.loan_listed),
        row.injury_status.clone(),
        row.recent_injury.clone().unwrap_or_default(),
        yes_no(row.stats_from_other_club),
        row.flags.join("|"),
        drivers,
    ]
}
