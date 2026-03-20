use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;
use std::collections::HashSet;

use crate::db::provision::{
    meta_upsert_public, rebuild_planet_search_public, rebuild_planets_fts_if_enabled,
};
use crate::provision::arcgis;
use crate::utils::normalize::normalize_text;

// ----------------------------
// Stats collection (optional)
// ----------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Inserted,
    Updated,
    Revived,
    MarkedDeleted,
}

#[derive(Debug, Clone)]
pub struct ChangeEvent {
    pub fid: i64,
    pub kind: ChangeKind,
    pub planet: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateSummary {
    pub inserted: i64,
    pub updated: i64,
    pub revived: i64,
    pub unchanged: i64,
    pub marked_deleted: i64,
    pub pruned: i64,
    pub skipped: i64,
    pub skipped_missing_planet: i64,
    pub skipped_missing_x: i64,
    pub skipped_missing_y: i64,
}

#[derive(Debug, Clone)]
pub struct UpdateStatsReport {
    pub top_inserted: Vec<ChangeEvent>,
    pub top_updated: Vec<ChangeEvent>,
    pub top_revived: Vec<ChangeEvent>,
    pub top_marked_deleted: Vec<ChangeEvent>,
    pub first_changed: Vec<ChangeEvent>,
}

#[derive(Debug, Clone)]
pub struct DbUpdateReport {
    pub downloaded_features: usize,
    pub dry_run: bool,
    pub prune: bool,
    pub summary: UpdateSummary,
    pub stats: Option<UpdateStatsReport>,
}

pub struct SkippedPlanetRow {
    pub fid: Option<i64>,
    pub planet: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub reason: String,
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
) -> Result<DbUpdateReport> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to build HTTP client")?;

    let layer = arcgis::fetch_layer_info(&client).context("Failed to fetch ArcGIS layer info")?;

    let page_size = layer.max_record_count.min(2000);
    let features = arcgis::fetch_all_features(&client, page_size)
        .context("Failed to download features from ArcGIS")?;

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
    let mut skipped_rows: Vec<SkippedPlanetRow> = Vec::new();

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
                skipped_rows.push(SkippedPlanetRow {
                    fid: None,
                    planet: planet_name(a),
                    x: get_f(a, "X"),
                    y: get_f(a, "Y"),
                    reason: "missing_fid".to_string(),
                });
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

            let mut reasons = Vec::new();
            if !planet_ok {
                reasons.push("missing_planet".to_string());
            }
            if !x_ok {
                reasons.push("missing_x".to_string());
            }
            if !y_ok {
                reasons.push("missing_y".to_string());
            }
            skipped_rows.push(SkippedPlanetRow {
                fid: Some(fid),
                planet: planet_name(a),
                x: get_f(a, "X"),
                y: get_f(a, "Y"),
                reason: reasons.join(","),
            });

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

        tx.execute("DELETE FROM planets_unknown", [])?;
        let mut stmt = tx.prepare_cached(
            r#"
            INSERT INTO planets_unknown(fid, planet, planet_norm, x, y, reason)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )?;
        for row in &skipped_rows {
            let planet = row
                .planet
                .clone()
                .unwrap_or_else(|| "(unknown)".to_string());
            let planet_norm = normalize_text(&planet);
            stmt.execute(params![
                row.fid,
                planet,
                planet_norm,
                row.x,
                row.y,
                row.reason
            ])?;
        }
        drop(stmt);

        tx.commit().context("Failed to commit db update")?;
    } else {
        // No commit: transaction rolls back automatically on drop
    }

    let summary = UpdateSummary {
        inserted,
        updated,
        revived,
        unchanged,
        marked_deleted,
        pruned,
        skipped,
        skipped_missing_planet,
        skipped_missing_x,
        skipped_missing_y,
    };

    let stats_report = if stats {
        fn collect_kind(
            events: &[ChangeEvent],
            kind: ChangeKind,
            stats_limit: usize,
        ) -> Vec<ChangeEvent> {
            let mut v: Vec<ChangeEvent> =
                events.iter().filter(|e| e.kind == kind).cloned().collect();
            v.sort_by_key(|e| e.fid);
            v.truncate(stats_limit);
            v
        }

        let mut changed: Vec<ChangeEvent> = events.clone();
        changed.sort_by_key(|e| e.fid);
        changed.truncate(stats_limit);

        Some(UpdateStatsReport {
            top_inserted: collect_kind(&events, ChangeKind::Inserted, stats_limit),
            top_updated: collect_kind(&events, ChangeKind::Updated, stats_limit),
            top_revived: collect_kind(&events, ChangeKind::Revived, stats_limit),
            top_marked_deleted: collect_kind(&events, ChangeKind::MarkedDeleted, stats_limit),
            first_changed: changed,
        })
    } else {
        None
    };

    Ok(DbUpdateReport {
        downloaded_features: features.len(),
        dry_run,
        prune,
        summary,
        stats: stats_report,
    })
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
