use crate::tui::TuiCommandOutput;
use crate::ui::{error, info, success, warning};
use sw_galaxy_map_core::db::db_status::{DbHealth, DbStatusReport};
use sw_galaxy_map_core::db::db_update::{ChangeKind, DbUpdateReport};
use sw_galaxy_map_core::db::migrate::MigrationReport;

pub(crate) fn print_db_init_report(report: &sw_galaxy_map_core::db::db_init::DbInitReport) {
    println!(
        "Initializing local database at: {}",
        report.out_path.display()
    );
    if report.overwritten_existing {
        info("Existing database overwritten.");
    }
    println!("Downloading data from remote service...");
    info(format!(
        "Downloaded {} features.",
        report.downloaded_features
    ));
    println!("Building SQLite database...");
    println!(
        "FTS5 enabled: {}",
        if report.fts_enabled { "yes" } else { "no" }
    );
    println!("Done.");
}

pub(crate) fn print_db_status_report(report: &DbStatusReport) {
    match report.health {
        DbHealth::Ok => success("Status: OK"),
        DbHealth::Missing => error("Status: MISSING"),
        DbHealth::Invalid => warning("Status: INVALID"),
    }

    info(format!("Database path: {}", report.db_path.display()));
    match report.file_size_bytes {
        Some(bytes) => info(format!("Database size: {} bytes", bytes)),
        None => warning("Database size: unavailable"),
    }

    for line in &report.lines {
        println!("{}", line);
    }
    for msg in &report.warnings {
        warning(msg);
    }
}

pub(crate) fn print_db_update_report(report: &DbUpdateReport) {
    info("Fetching data from remote service...");
    info(format!(
        "Downloaded {} features. Comparing with local database...",
        report.downloaded_features
    ));
    if report.dry_run {
        warning("DRY-RUN mode enabled: no changes will be written");
        if report.prune {
            warning("Prune requested in dry-run: this will be reported as 'would prune'");
        }
    }

    if report.dry_run {
        success("Dry-run completed (no changes written)");
    } else {
        success("Update completed");
    }

    info(format!("inserted: {}", report.summary.inserted));
    info(format!("updated: {}", report.summary.updated));
    info(format!("revived: {}", report.summary.revived));
    info(format!("unchanged: {}", report.summary.unchanged));
    info(format!("marked deleted: {}", report.summary.marked_deleted));

    if report.prune {
        if report.dry_run {
            info(format!("would prune: {}", report.summary.pruned));
        } else {
            info(format!("pruned: {}", report.summary.pruned));
        }
    }

    if report.summary.skipped > 0 {
        warning(format!("skipped invalid rows: {}", report.summary.skipped));
        info(format!(
            "  missing Planet: {}",
            report.summary.skipped_missing_planet
        ));
        info(format!("  missing X: {}", report.summary.skipped_missing_x));
        info(format!("  missing Y: {}", report.summary.skipped_missing_y));
    }

    if let Some(stats) = &report.stats {
        fn kind_label(k: ChangeKind) -> &'static str {
            match k {
                ChangeKind::Inserted => "inserted",
                ChangeKind::Updated => "updated",
                ChangeKind::Revived => "revived",
                ChangeKind::MarkedDeleted => "marked deleted",
            }
        }
        println!();
        info("Stats:");
        for (kind, rows) in [
            (ChangeKind::Inserted, &stats.top_inserted),
            (ChangeKind::Updated, &stats.top_updated),
            (ChangeKind::Revived, &stats.top_revived),
            (ChangeKind::MarkedDeleted, &stats.top_marked_deleted),
        ] {
            info(format!("  Top {} {}:", rows.len(), kind_label(kind)));
            if rows.is_empty() {
                info("    (none)");
            } else {
                for e in rows {
                    if let Some(p) = &e.planet {
                        info(format!("    FID={} | {}", e.fid, p));
                    } else {
                        info(format!("    FID={}", e.fid));
                    }
                }
            }
        }
        info(format!(
            "  First {} changed FIDs:",
            stats.first_changed.len()
        ));
        if stats.first_changed.is_empty() {
            info("    (none)");
        } else {
            for e in &stats.first_changed {
                let planet = e
                    .planet
                    .as_ref()
                    .map(|p| format!(" | {}", p))
                    .unwrap_or_default();
                info(format!(
                    "    FID={} | {}{}",
                    e.fid,
                    kind_label(e.kind),
                    planet
                ));
            }
        }
    }
}

pub(crate) fn print_migration_report(report: &MigrationReport) {
    if report.noop {
        info(format!(
            "Database schema already up-to-date (v{})",
            report.current_version
        ));
        return;
    }
    info(format!(
        "Database schema upgrade required (current: v{}, target: v{})",
        report.current_version, report.target_version
    ));
    if report.dry_run {
        warning("DRY-RUN: no changes will be applied");
    }
    for step in &report.applied {
        info(format!(
            "Applying migration: v{} → v{} ({})",
            step.from, step.to, step.label
        ));
        success(format!("Migration v{} → v{} completed", step.from, step.to));
    }
    if report.dry_run {
        info(format!(
            "Dry-run completed: {} migration(s) would be applied.",
            report.applied.len()
        ));
    } else {
        info(format!(
            "Database schema successfully updated (applied {} migration(s)).",
            report.applied.len()
        ));
    }
}

// ---------------------------------------------------------------------------
// Galaxy statistics rendering (v0.15.0)
// ---------------------------------------------------------------------------

fn pct(n: i64, total: i64) -> String {
    if total > 0 {
        format!("{:5.1}%", n as f64 / total as f64 * 100.0)
    } else {
        "    -".to_string()
    }
}

pub(crate) fn print_galaxy_stats(s: &sw_galaxy_map_core::model::GalaxyStats, top: usize) {
    println!("Galaxy Statistics");
    println!("=================");
    println!();
    println!("Planets: {} total", s.total_planets);
    println!();
    println!("  By status:");
    println!(
        "    active     : {:>6}  ({})",
        s.status_active,
        pct(s.status_active, s.total_planets)
    );
    println!(
        "    inserted   : {:>6}  ({})",
        s.status_inserted,
        pct(s.status_inserted, s.total_planets)
    );
    println!(
        "    modified   : {:>6}  ({})",
        s.status_modified,
        pct(s.status_modified, s.total_planets)
    );
    println!(
        "    skipped    : {:>6}  ({})",
        s.status_skipped,
        pct(s.status_skipped, s.total_planets)
    );
    println!(
        "    deleted    : {:>6}  ({})",
        s.status_deleted,
        pct(s.status_deleted, s.total_planets)
    );
    println!(
        "    invalid    : {:>6}  ({})",
        s.status_invalid,
        pct(s.status_invalid, s.total_planets)
    );
    if s.status_null > 0 {
        println!(
            "    (no status): {:>6}  ({})",
            s.status_null,
            pct(s.status_null, s.total_planets)
        );
    }

    let active_total = s.total_planets - s.status_deleted - s.status_skipped - s.status_invalid;

    println!();
    println!("  Canon / Legends (active planets only):");
    println!(
        "    Canon      : {:>6}  ({})",
        s.canon_count,
        pct(s.canon_count, active_total)
    );
    println!(
        "    Legends    : {:>6}  ({})",
        s.legends_count,
        pct(s.legends_count, active_total)
    );
    println!(
        "    Both       : {:>6}  ({})",
        s.both_count,
        pct(s.both_count, active_total)
    );
    println!(
        "    Neither    : {:>6}  ({})",
        s.neither_count,
        pct(s.neither_count, active_total)
    );

    if !s.top_regions.is_empty() {
        println!();
        println!("  Top {} regions:", top);
        let name_w = s
            .top_regions
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(10)
            .max(10);
        for (i, (name, cnt)) in s.top_regions.iter().enumerate() {
            println!(
                "    {:>2}. {:<name_w$} : {:>5}  ({})",
                i + 1,
                name,
                cnt,
                pct(*cnt, active_total)
            );
        }
    }

    if !s.top_sectors.is_empty() {
        println!();
        println!("  Top {} sectors:", top);
        let name_w = s
            .top_sectors
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(10)
            .max(10);
        for (i, (name, cnt)) in s.top_sectors.iter().enumerate() {
            println!("    {:>2}. {:<name_w$} : {:>5}", i + 1, name, cnt);
        }
    }

    println!();
    println!("  Grid coverage: {} distinct cells", s.distinct_grids);
    if !s.top_grids.is_empty() {
        let top_str: Vec<String> = s
            .top_grids
            .iter()
            .take(5)
            .map(|(g, c)| format!("{} ({})", g, c))
            .collect();
        println!("    Most populated: {}", top_str.join(", "));
    }

    if s.total_routes > 0 {
        println!();
        println!("Routes: {}", s.total_routes);
        println!("  ok       : {}", s.routes_ok);
        println!("  failed   : {}", s.routes_failed);
        if s.total_route_length > 0.0 {
            println!("  Total length : {:.1} parsec", s.total_route_length);
        }
        if s.avg_detours_per_route > 0.0 {
            println!("  Avg detours  : {:.1} per route", s.avg_detours_per_route);
        }
    }
}

pub(crate) fn build_galaxy_stats_tui(
    s: &sw_galaxy_map_core::model::GalaxyStats,
    top: usize,
    out: &mut TuiCommandOutput,
) {
    let l = &mut out.log_lines;

    l.push("Galaxy Statistics".to_string());
    l.push("=================".to_string());
    l.push(String::new());
    l.push(format!("Planets: {} total", s.total_planets));
    l.push(String::new());

    l.push("  By status:".to_string());
    l.push(format!(
        "    active     : {:>6}  ({})",
        s.status_active,
        pct(s.status_active, s.total_planets)
    ));
    l.push(format!(
        "    inserted   : {:>6}  ({})",
        s.status_inserted,
        pct(s.status_inserted, s.total_planets)
    ));
    l.push(format!(
        "    modified   : {:>6}  ({})",
        s.status_modified,
        pct(s.status_modified, s.total_planets)
    ));
    l.push(format!(
        "    skipped    : {:>6}  ({})",
        s.status_skipped,
        pct(s.status_skipped, s.total_planets)
    ));
    l.push(format!(
        "    deleted    : {:>6}  ({})",
        s.status_deleted,
        pct(s.status_deleted, s.total_planets)
    ));
    l.push(format!(
        "    invalid    : {:>6}  ({})",
        s.status_invalid,
        pct(s.status_invalid, s.total_planets)
    ));
    if s.status_null > 0 {
        l.push(format!(
            "    (no status): {:>6}  ({})",
            s.status_null,
            pct(s.status_null, s.total_planets)
        ));
    }

    let active_total = s.total_planets - s.status_deleted - s.status_skipped - s.status_invalid;

    l.push(String::new());
    l.push("  Canon / Legends (active planets only):".to_string());
    l.push(format!(
        "    Canon      : {:>6}  ({})",
        s.canon_count,
        pct(s.canon_count, active_total)
    ));
    l.push(format!(
        "    Legends    : {:>6}  ({})",
        s.legends_count,
        pct(s.legends_count, active_total)
    ));
    l.push(format!(
        "    Both       : {:>6}  ({})",
        s.both_count,
        pct(s.both_count, active_total)
    ));
    l.push(format!(
        "    Neither    : {:>6}  ({})",
        s.neither_count,
        pct(s.neither_count, active_total)
    ));

    if !s.top_regions.is_empty() {
        l.push(String::new());
        l.push(format!("  Top {} regions:", top));
        let name_w = s
            .top_regions
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(10)
            .max(10);
        for (i, (name, cnt)) in s.top_regions.iter().enumerate() {
            l.push(format!(
                "    {:>2}. {:<name_w$} : {:>5}  ({})",
                i + 1,
                name,
                cnt,
                pct(*cnt, active_total)
            ));
        }
    }

    if !s.top_sectors.is_empty() {
        l.push(String::new());
        l.push(format!("  Top {} sectors:", top));
        let name_w = s
            .top_sectors
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(10)
            .max(10);
        for (i, (name, cnt)) in s.top_sectors.iter().enumerate() {
            l.push(format!("    {:>2}. {:<name_w$} : {:>5}", i + 1, name, cnt));
        }
    }

    l.push(String::new());
    l.push(format!(
        "  Grid coverage: {} distinct cells",
        s.distinct_grids
    ));
    if !s.top_grids.is_empty() {
        let top_str: Vec<String> = s
            .top_grids
            .iter()
            .take(5)
            .map(|(g, c)| format!("{} ({})", g, c))
            .collect();
        l.push(format!("    Most populated: {}", top_str.join(", ")));
    }

    if s.total_routes > 0 {
        l.push(String::new());
        l.push(format!("Routes: {}", s.total_routes));
        l.push(format!("  ok       : {}", s.routes_ok));
        l.push(format!("  failed   : {}", s.routes_failed));
        if s.total_route_length > 0.0 {
            l.push(format!(
                "  Total length : {:.1} parsec",
                s.total_route_length
            ));
        }
        if s.avg_detours_per_route > 0.0 {
            l.push(format!(
                "  Avg detours  : {:.1} per route",
                s.avg_detours_per_route
            ));
        }
    }
}
