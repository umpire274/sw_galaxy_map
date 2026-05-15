use anyhow::{Context, Result, bail};
use csv::{ReaderBuilder, StringRecord};
use std::path::Path;

use crate::models::{CsvOverlayFormat, CsvOverlayRow};
use crate::utils::normalize_text;

/// Load and normalize CSV overlay rows.
pub fn load_overlay_csv(
    path: &Path,
    format: Option<CsvOverlayFormat>,
    delimiter: Option<u8>,
) -> Result<Vec<CsvOverlayRow>> {
    let requested_format = format.unwrap_or(CsvOverlayFormat::Auto);

    let delimiter = match delimiter {
        Some(value) => value,
        None => detect_delimiter(path, requested_format)?,
    };

    let mut rdr = ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("Unable to open CSV overlay file: {}", path.display()))?;

    let headers = rdr.headers()?.clone();
    let detected_format = detect_format(&headers, requested_format)?;
    let mapping = CsvColumnMapping::from_headers(&headers, detected_format)?;

    let mut rows = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let row = mapping.row_from_record(&record)?;

        if row.system.trim().is_empty() {
            continue;
        }

        rows.push(row);
    }

    Ok(rows)
}

fn detect_delimiter(path: &Path, format: CsvOverlayFormat) -> Result<u8> {
    if format == CsvOverlayFormat::Official {
        return Ok(b';');
    }

    if format == CsvOverlayFormat::Full {
        return Ok(b',');
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Unable to inspect CSV delimiter: {}", path.display()))?;
    let first_line = content.lines().next().unwrap_or_default();

    let semicolon_count = first_line.matches(';').count();
    let comma_count = first_line.matches(',').count();

    if semicolon_count > comma_count {
        Ok(b';')
    } else {
        Ok(b',')
    }
}

fn detect_format(headers: &StringRecord, requested: CsvOverlayFormat) -> Result<CsvOverlayFormat> {
    if requested != CsvOverlayFormat::Auto {
        return Ok(requested);
    }

    let normalized = headers
        .iter()
        .map(|header| header.trim().to_lowercase())
        .collect::<Vec<_>>();

    let has_official_columns = ["system", "sector", "region", "grid"]
        .iter()
        .all(|column| normalized.iter().any(|header| header == column));

    let has_full_columns = ["planet", "region", "sector", "system", "grid"]
        .iter()
        .all(|column| normalized.iter().any(|header| header == column));

    if has_full_columns && normalized.iter().any(|header| header == "fid") {
        Ok(CsvOverlayFormat::Full)
    } else if has_official_columns {
        Ok(CsvOverlayFormat::Official)
    } else {
        bail!("Unable to detect CSV overlay format from headers: {headers:?}")
    }
}

#[derive(Debug, Clone)]
struct CsvColumnMapping {
    system: usize,
    sector: Option<usize>,
    region: Option<usize>,
    grid: Option<usize>,
}

impl CsvColumnMapping {
    fn from_headers(headers: &StringRecord, format: CsvOverlayFormat) -> Result<Self> {
        let system_name = match format {
            CsvOverlayFormat::Auto => unreachable!("format must be detected before mapping"),
            CsvOverlayFormat::Official => "system",
            CsvOverlayFormat::Full => "System",
        };

        Ok(Self {
            system: find_required_header(headers, system_name)?,
            sector: find_optional_header(headers, "sector"),
            region: find_optional_header(headers, "region"),
            grid: find_optional_header(headers, "grid"),
        })
    }

    fn row_from_record(&self, record: &StringRecord) -> Result<CsvOverlayRow> {
        Ok(CsvOverlayRow {
            system: normalize_text(record.get(self.system).unwrap_or_default()),
            sector: normalize_text(optional_value(record, self.sector)),
            region: normalize_text(optional_value(record, self.region)),
            grid: normalize_text(optional_value(record, self.grid)),
        })
    }
}

fn find_required_header(headers: &StringRecord, name: &str) -> Result<usize> {
    find_optional_header(headers, name).with_context(|| format!("Missing CSV header '{name}'"))
}

fn find_optional_header(headers: &StringRecord, name: &str) -> Option<usize> {
    headers
        .iter()
        .position(|header| header.trim().eq_ignore_ascii_case(name))
}

fn optional_value(record: &StringRecord, index: Option<usize>) -> &str {
    index.and_then(|idx| record.get(idx)).unwrap_or_default()
}
