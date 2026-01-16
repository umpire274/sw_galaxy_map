use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;
use std::collections::HashSet;

use crate::db::provision::{
    meta_upsert_public, rebuild_planet_search_public, rebuild_planets_fts_if_enabled,
};
use crate::normalize::normalize_text;
use crate::provision::arcgis;
use crate::ui;

// ----------------------------
// Stats collection (optional)
// ----------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeKind {
    Inserted,
    Updated,
    Revived,
    MarkedDeleted,
}

#[derive(Debug, Clone)]
struct ChangeEvent {
    fid: i64,
    kind: ChangeKind,
    planet: Option<String>,
}

fn compute_arcgis_hash(a: &Value) -> String {
    // Must match the one used in provision (keep in sync)
    let keys = [
        "FID",
        "Planet",
        "Region",
        "Sector",
        "System",
        "Grid",
        "X",
        "Y",
        "Canon",
        "Legends",
        "zm",
        "name0",
        "name1",
        "name2",
        "lat",
        "long",
        "ref",
        "status",
        "CRegion",
        "CRegion_li",
    ];

    let mut s = String::new();
    for k in keys {
        s.push_str(k);
        s.push('=');

        match a.get(k) {
            None | Some(Value::Null) => {}
            Some(Value::String(v)) => s.push_str(v.trim()),
            Some(Value::Number(n)) => s.push_str(&n.to_string()),
            Some(Value::Bool(b)) => s.push_str(if *b { "1" } else { "0" }),
            Some(other) => s.push_str(&other.to_string()),
        }

        s.push('\n');
    }

    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

fn get_i(a: &Value, k: &str) -> Option<i64> {
    a.get(k).and_then(|v| v.as_i64())
}
fn get_f(a: &Value, k: &str) -> Option<f64> {
    a.get(k).and_then(|v| v.as_f64())
}
fn get_s(a: &Value, k: &str) -> Option<String> {
    a.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
}

fn upsert_planet(tx: &Transaction<'_>, a: &Value) -> Result<()> {
    let fid = get_i(a, "FID").context("Missing FID")?;
    let planet = get_s(a, "Planet").unwrap_or_default();
    let x = get_f(a, "X").context("Missing X")?;
    let y = get_f(a, "Y").context("Missing Y")?;

    if planet.is_empty() {
        // Skip invalid rows (policy consistent with init)
        return Ok(());
    }

    let planet_norm = normalize_text(&planet);
    let arcgis_hash = compute_arcgis_hash(a);

    // DELETE + INSERT strategy (ensures aliases cascade cleanly)
    tx.execute("DELETE FROM planets WHERE FID = ?1", params![fid])?;

    tx.execute(
        r#"
        INSERT INTO planets(
            FID, Planet, planet_norm, Region, Sector, System, Grid,
            X, Y,
            arcgis_hash, deleted,
            Canon, Legends, zm,
            name0, name1, name2,
            lat, long, ref, status, CRegion, CRegion_li
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7,
            ?8, ?9,
            ?10, 0,
            ?11, ?12, ?13,
            ?14, ?15, ?16,
            ?17, ?18, ?19, ?20, ?21, ?22
        )
        "#,
        params![
            fid,
            planet,
            planet_norm,
            get_s(a, "Region"),
            get_s(a, "Sector"),
            get_s(a, "System"),
            get_s(a, "Grid"),
            x,
            y,
            arcgis_hash,
            get_i(a, "Canon"),
            get_i(a, "Legends"),
            get_i(a, "zm"),
            get_s(a, "name0"),
            get_s(a, "name1"),
            get_s(a, "name2"),
            get_f(a, "lat"),
            get_f(a, "long"),
            get_s(a, "ref"),
            get_s(a, "status"),
            get_s(a, "CRegion"),
            get_s(a, "CRegion_li"),
        ],
    )?;

    // Insert aliases from name0/name1/name2
    let mut stmt_alias = tx.prepare_cached(
        r#"
        INSERT OR IGNORE INTO planet_aliases(planet_fid, alias, alias_norm, source)
        VALUES (?1, ?2, ?3, ?4)
        "#,
    )?;

    for (src, key) in [("name0", "name0"), ("name1", "name1"), ("name2", "name2")] {
        if let Some(val) = a.get(key).and_then(|v| v.as_str()) {
            let val = val.trim();
            if !val.is_empty() {
                stmt_alias.execute(params![fid, val, normalize_text(val), src])?;
            }
        }
    }

    Ok(())
}

fn db_get_hash_and_deleted(tx: &Transaction<'_>, fid: i64) -> Result<Option<(String, i64)>> {
    tx.query_row(
        "SELECT arcgis_hash, deleted FROM planets WHERE FID = ?1",
        [fid],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
    )
    .optional()
    .map_err(Into::into)
}

fn mark_deleted_missing(tx: &Transaction<'_>, keep_fids: &HashSet<i64>) -> Result<i64> {
    // Mark planets not in remote feed as deleted=1
    // For SQLite, best approach: create temp table and join.
    tx.execute_batch(
        "DROP TABLE IF EXISTS __keep_fids; CREATE TEMP TABLE __keep_fids(fid INTEGER PRIMARY KEY);",
    )?;

    {
        let mut ins = tx.prepare("INSERT OR IGNORE INTO __keep_fids(fid) VALUES (?1)")?;
        for fid in keep_fids {
            ins.execute([fid])?;
        }
    }

    let changed = tx.execute(
        r#"
        UPDATE planets
        SET deleted = 1
        WHERE deleted = 0
          AND FID NOT IN (SELECT fid FROM __keep_fids)
        "#,
        [],
    )? as i64;

    tx.execute_batch("DROP TABLE IF EXISTS __keep_fids;")?;
    Ok(changed)
}

fn prune_deleted(tx: &Transaction<'_>) -> Result<i64> {
    // FK cascades will remove aliases/search automatically (where linked).
    let n = tx.execute("DELETE FROM planets WHERE deleted = 1", [])? as i64;
    Ok(n)
}

pub fn run(
    con: &mut Connection,
    prune: bool,
    dry_run: bool,
    stats: bool,
    stats_limit: usize,
) -> Result<()> {
    ui::info("Fetching data from remote service...");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to build HTTP client")?;

    let layer = arcgis::fetch_layer_info(&client).context("Failed to fetch ArcGIS layer info")?;

    let page_size = layer.max_record_count.min(2000);
    let features = arcgis::fetch_all_features(&client, page_size)
        .context("Failed to download features from ArcGIS")?;

    ui::info(format!(
        "Downloaded {} features. Comparing with local database...",
        features.len()
    ));

    if dry_run {
        ui::warning("DRY-RUN mode enabled: no changes will be written");
        if prune {
            ui::warning("Prune requested in dry-run: this will be reported as 'would prune'");
        }
    }

    // Start transaction: gives consistent view and allows temp tables.
    // In dry-run we will NOT commit -> changes (if any) won't persist.
    let tx = con
        .transaction()
        .context("Failed to start update transaction")?;

    // Write meta only in real mode
    if !dry_run {
        meta_upsert_public(&tx, "source_serviceItemId", &layer.service_item_id)?;

        meta_upsert_public(
            &tx,
            "source_maxRecordCount",
            &layer.max_record_count.to_string(),
        )?;

        if let Some(v) = layer.current_version {
            meta_upsert_public(&tx, "source_currentVersion", &v.to_string())?;
        }

        if let Some(ms) = layer.editing_info.as_ref().and_then(|e| e.last_edit_date) {
            meta_upsert_public(&tx, "source_lastEditDate", &ms.to_string())?;
        }

        if let Some(ms) = layer
            .editing_info
            .as_ref()
            .and_then(|e| e.schema_last_edit_date)
        {
            meta_upsert_public(&tx, "source_schemaLastEditDate", &ms.to_string())?;
        }

        if let Some(ms) = layer
            .editing_info
            .as_ref()
            .and_then(|e| e.data_last_edit_date)
        {
            meta_upsert_public(&tx, "source_dataLastEditDate", &ms.to_string())?;
        }
    }

    let mut events: Vec<ChangeEvent> = Vec::new();

    // Keep set of FIDs present in remote feed
    let mut keep = HashSet::<i64>::with_capacity(features.len());

    // Counters
    let mut inserted: i64 = 0;
    let mut updated: i64 = 0;
    let mut unchanged: i64 = 0;
    let mut revived: i64 = 0;

    let mut skipped: i64 = 0;
    let mut skipped_missing_planet: i64 = 0;
    let mut skipped_missing_x: i64 = 0;
    let mut skipped_missing_y: i64 = 0;

    // Helper to capture best-effort planet name
    let planet_name = |a: &Value| -> Option<String> {
        a.get("Planet")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    // 1) Per-feature compare (and apply only if !dry_run)
    for a in &features {
        let fid = match get_i(a, "FID") {
            Some(v) => v,
            None => {
                skipped += 1;
                continue;
            }
        };
        keep.insert(fid);

        // Skip invalid rows (Planet empty or X/Y missing) consistent with init
        let planet_ok = a
            .get("Planet")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        let x_ok = get_f(a, "X").is_some();
        let y_ok = get_f(a, "Y").is_some();

        if !planet_ok || !x_ok || !y_ok {
            skipped += 1;

            if !planet_ok {
                skipped_missing_planet += 1;
            }
            if !x_ok {
                skipped_missing_x += 1;
            }
            if !y_ok {
                skipped_missing_y += 1;
            }

            continue;
        }

        let new_hash = compute_arcgis_hash(a);

        match db_get_hash_and_deleted(&tx, fid)? {
            None => {
                inserted += 1;
                if stats && (events.len() < stats_limit * 50) {
                    events.push(ChangeEvent {
                        fid,
                        kind: ChangeKind::Inserted,
                        planet: planet_name(a),
                    });
                }
                if !dry_run {
                    upsert_planet(&tx, a)?;
                }
            }
            Some((old_hash, old_deleted)) => {
                if old_deleted == 1 {
                    revived += 1;
                    if stats && (events.len() < stats_limit * 50) {
                        events.push(ChangeEvent {
                            fid,
                            kind: ChangeKind::Revived,
                            planet: planet_name(a),
                        });
                    }
                    if !dry_run {
                        // revive by forcing rewrite
                        upsert_planet(&tx, a)?;
                    }
                } else if old_hash != new_hash {
                    updated += 1;
                    if stats && (events.len() < stats_limit * 50) {
                        events.push(ChangeEvent {
                            fid,
                            kind: ChangeKind::Updated,
                            planet: planet_name(a),
                        });
                    }
                    if !dry_run {
                        upsert_planet(&tx, a)?;
                    }
                } else {
                    unchanged += 1;
                }
            }
        }
    }

    // 2) Soft-delete missing (real) OR compute count (dry-run)
    // If stats enabled, capture a preview of top missing (FID, Planet) before actually updating.
    let mut deleted_preview: Vec<(i64, String)> = Vec::new();
    if stats {
        deleted_preview = select_missing_active_planets(&tx, &keep, stats_limit)
            .context("Failed to compute missing planets preview for --stats")?;
    }

    let marked_deleted: i64 = if dry_run {
        count_missing_active_planets(&tx, &keep)?
    } else {
        mark_deleted_missing(&tx, &keep)?
    };

    if stats {
        for (fid, planet) in deleted_preview {
            events.push(ChangeEvent {
                fid,
                kind: ChangeKind::MarkedDeleted,
                planet: Some(planet),
            });
        }
    }

    // 3) Prune (real) OR compute would-prune (dry-run)
    let pruned: i64 = if prune {
        if dry_run {
            // would prune: already deleted + would-be-marked-deleted
            let already_deleted: i64 = tx
                .query_row("SELECT COUNT(*) FROM planets WHERE deleted = 1", [], |r| {
                    r.get(0)
                })
                .context("Failed to count already deleted planets")?;
            already_deleted + marked_deleted
        } else {
            ui::warning("Prune enabled: permanently removing deleted planets");
            prune_deleted(&tx)?
        }
    } else {
        0
    };

    if !dry_run {
        // Rebuild derived tables
        rebuild_planet_search_public(&tx)?;
        rebuild_planets_fts_if_enabled(&tx)?;

        // Update meta
        meta_upsert_public(&tx, "last_update_utc", &crate::utils::time::now_utc_iso())?;
        meta_upsert_public(&tx, "update_mode", "incremental")?;
        meta_upsert_public(&tx, "prune_used", if prune { "1" } else { "0" })?;

        tx.commit().context("Failed to commit db update")?;

        ui::success("Update completed");
    } else {
        // No commit: transaction rolls back automatically on drop
        ui::success("Dry-run completed (no changes written)");
    }

    ui::info(format!("inserted: {}", inserted));
    ui::info(format!("updated: {}", updated));
    ui::info(format!("revived: {}", revived));
    ui::info(format!("unchanged: {}", unchanged));
    ui::info(format!("marked deleted: {}", marked_deleted));

    if prune {
        if dry_run {
            ui::info(format!("would prune: {}", pruned));
        } else {
            ui::info(format!("pruned: {}", pruned));
        }
    }

    if skipped > 0 {
        ui::warning(format!("skipped invalid rows: {}", skipped));
        ui::info(format!("  missing Planet: {}", skipped_missing_planet));
        ui::info(format!("  missing X: {}", skipped_missing_x));
        ui::info(format!("  missing Y: {}", skipped_missing_y));
    }

    // ----------------------------
    // Print --stats section
    // ----------------------------
    if stats {
        fn kind_label(k: ChangeKind) -> &'static str {
            match k {
                ChangeKind::Inserted => "inserted",
                ChangeKind::Updated => "updated",
                ChangeKind::Revived => "revived",
                ChangeKind::MarkedDeleted => "marked deleted",
            }
        }

        println!();
        ui::info("Stats:");

        // Top N per category (sorted by FID)
        for k in [
            ChangeKind::Inserted,
            ChangeKind::Updated,
            ChangeKind::Revived,
            ChangeKind::MarkedDeleted,
        ] {
            let mut v: Vec<&ChangeEvent> = events.iter().filter(|e| e.kind == k).collect();
            v.sort_by_key(|e| e.fid);

            ui::info(format!("  Top {} {}:", stats_limit, kind_label(k)));
            if v.is_empty() {
                ui::info("    (none)");
            } else {
                for e in v.into_iter().take(stats_limit) {
                    if let Some(p) = &e.planet {
                        ui::info(format!("    FID={} | {}", e.fid, p));
                    } else {
                        ui::info(format!("    FID={}", e.fid));
                    }
                }
            }
        }

        // First N changed FIDs overall
        let mut changed: Vec<&ChangeEvent> = events.iter().collect();
        changed.sort_by_key(|e| e.fid);

        ui::info(format!("  First {} changed FIDs:", stats_limit));
        if changed.is_empty() {
            ui::info("    (none)");
        } else {
            for e in changed.into_iter().take(stats_limit) {
                let planet = e
                    .planet
                    .as_ref()
                    .map(|p| format!(" | {}", p))
                    .unwrap_or_default();
                ui::info(format!(
                    "    FID={} | {}{}",
                    e.fid,
                    kind_label(e.kind),
                    planet
                ));
            }
        }
    }

    Ok(())
}

/// Returns top N missing active planets (deleted=0 and not in keep_fids), ordered by FID.
/// Used for --stats preview (both dry-run and real).
fn select_missing_active_planets(
    tx: &Transaction<'_>,
    keep_fids: &HashSet<i64>,
    limit: usize,
) -> Result<Vec<(i64, String)>> {
    tx.execute_batch(
        "DROP TABLE IF EXISTS __keep_fids; CREATE TEMP TABLE __keep_fids(fid INTEGER PRIMARY KEY);",
    )?;

    {
        let mut ins = tx.prepare("INSERT OR IGNORE INTO __keep_fids(fid) VALUES (?1)")?;
        for fid in keep_fids {
            ins.execute([fid])?;
        }
    }

    let mut stmt = tx.prepare(
        r#"
        SELECT FID, Planet
        FROM planets
        WHERE deleted = 0
          AND FID NOT IN (SELECT fid FROM __keep_fids)
        ORDER BY FID
        LIMIT ?1
        "#,
    )?;

    let rows = stmt.query_map([limit as i64], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }

    tx.execute_batch("DROP TABLE IF EXISTS __keep_fids;")?;
    Ok(out)
}

/// Dry-run helper: counts how many active (deleted=0) planets would be marked deleted
/// given the keep_fids set, WITHOUT performing UPDATEs.
fn count_missing_active_planets(tx: &Transaction<'_>, keep_fids: &HashSet<i64>) -> Result<i64> {
    // Use the same temp-table strategy as mark_deleted_missing(), but do a SELECT COUNT(*)
    tx.execute_batch(
        "DROP TABLE IF EXISTS __keep_fids; CREATE TEMP TABLE __keep_fids(fid INTEGER PRIMARY KEY);",
    )?;

    {
        let mut ins = tx.prepare("INSERT OR IGNORE INTO __keep_fids(fid) VALUES (?1)")?;
        for fid in keep_fids {
            ins.execute([fid])?;
        }
    }

    let n: i64 = tx.query_row(
        r#"
        SELECT COUNT(*)
        FROM planets
        WHERE deleted = 0
          AND FID NOT IN (SELECT fid FROM __keep_fids)
        "#,
        [],
        |r| r.get(0),
    )?;

    tx.execute_batch("DROP TABLE IF EXISTS __keep_fids;")?;
    Ok(n)
}
