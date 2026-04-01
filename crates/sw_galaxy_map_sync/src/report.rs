use crate::models::{ReportKind, ReportRow, SyncReport};
use anyhow::Result;
use rust_xlsxwriter::Workbook;

/// Push a row into the correct report bucket.
pub fn push_report_row(report: &mut SyncReport, row: ReportRow, kind: ReportKind) {
    match kind {
        ReportKind::Inserted => report.inserted.push(row),
        ReportKind::Modified => report.modified.push(row),
        ReportKind::Active => report.active.push(row),
        ReportKind::Invalid => report.invalid.push(row),
        ReportKind::Skipped => report.skipped.push(row),
        ReportKind::Deleted => report.deleted.push(row),
    }
}

fn write_sheet(workbook: &mut Workbook, name: &str, rows: &[ReportRow]) -> Result<()> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(name)?;

    // --- HEADER ---
    worksheet.write_string(0, 0, "FID")?;
    worksheet.write_string(0, 1, "Planet")?;
    worksheet.write_string(0, 2, "Sector")?;
    worksheet.write_string(0, 3, "Region")?;
    worksheet.write_string(0, 4, "Grid")?;

    // --- FREEZE HEADER ---
    worksheet.set_freeze_panes(1, 0)?;

    // --- DATA ---
    for (i, row) in rows.iter().enumerate() {
        let r = (i + 1) as u32;

        if let Some(fid) = row.fid {
            worksheet.write_number(r, 0, fid as f64)?;
        }

        worksheet.write_string(r, 1, &row.planet)?;
        worksheet.write_string(r, 2, &row.sector)?;
        worksheet.write_string(r, 3, &row.region)?;
        worksheet.write_string(r, 4, &row.grid)?;
    }

    // --- AUTOFILTER ---
    if !rows.is_empty() {
        let last_row = rows.len() as u32;
        worksheet.autofilter(0, 0, last_row, 4)?;
    }

    Ok(())
}

/// Write the synchronization report to an XLSX file.
pub fn write_report(path: &str, report: &SyncReport) -> anyhow::Result<()> {
    let mut workbook = Workbook::new();

    write_sheet(&mut workbook, "inserted", &report.inserted)?;
    write_sheet(&mut workbook, "modified", &report.modified)?;
    write_sheet(&mut workbook, "active", &report.active)?;
    write_sheet(&mut workbook, "invalid", &report.invalid)?;
    write_sheet(&mut workbook, "skipped", &report.skipped)?;
    write_sheet(&mut workbook, "deleted", &report.deleted)?;

    workbook.save(path)?;
    Ok(())
}
