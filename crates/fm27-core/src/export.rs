//! CSV and JSON export of the current player query.

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

pub fn write_export(path: &Path, format: &str, rows: &[PlayerRow]) -> Result<ExportReport> {
    match format.to_ascii_lowercase().as_str() {
        "csv" => write_csv(path, rows)?,
        "json" => write_json(path, rows)?,
        other => {
            return Err(Error::parse(format!(
                "unknown export format {other:?}; expected csv or json"
            )));
        }
    }
    Ok(ExportReport {
        path: path.display().to_string(),
        format: format.to_ascii_lowercase(),
        rows: rows.len(),
    })
}

fn write_csv(path: &Path, rows: &[PlayerRow]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    let mut header = vec![
        "uid".to_owned(),
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
    writer.write_record(&header)?;
    for row in rows {
        let mut record = vec![
            row.uid.to_string(),
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
        for key in crate::layout::ATTRIBUTE_KEYS {
            record.push(
                row.attributes
                    .get(key)
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            );
        }
        writer.write_record(&record)?;
    }
    writer.flush()?;
    Ok(())
}

fn write_json(path: &Path, rows: &[PlayerRow]) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, rows)?;
    Ok(())
}
