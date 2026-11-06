//! CSV and JSON export of the current player query.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::store::PlayerRow;

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

fn write_indented_json(writer: &mut BufWriter<File>, row: &PlayerRow) -> Result<()> {
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
