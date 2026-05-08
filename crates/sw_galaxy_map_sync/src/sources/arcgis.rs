use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;

use crate::models::{ArcgisDataset, ArcgisRecord, NormalizedPlanet};
use crate::utils::{build_planet_norm, hash_source, normalize_text};

/// Fetch raw ArcGIS attributes through the core provider.
pub fn fetch_raw_features(page_size: i64) -> Result<Vec<Value>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("Unable to create HTTP client")?;

    let layer_info = sw_galaxy_map_core::provision::arcgis::fetch_layer_info(&client)
        .context("Unable to fetch ArcGIS layer info")?;

    let effective_page_size = if page_size <= 0 {
        layer_info.max_record_count
    } else {
        page_size
    };

    sw_galaxy_map_core::provision::arcgis::fetch_all_features(&client, effective_page_size)
        .context("Unable to fetch ArcGIS features")
}

/// Fetch and classify ArcGIS features.
pub fn fetch_arcgis_records(page_size: i64) -> Result<Vec<ArcgisRecord>> {
    let raw = fetch_raw_features(page_size)?;

    raw.into_iter()
        .map(normalize_arcgis_feature)
        .collect::<Result<Vec<_>>>()
}

/// Fetch and normalize ArcGIS features, split into known and unknown records.
pub fn fetch_arcgis_dataset(page_size: i64) -> Result<ArcgisDataset> {
    let records = fetch_arcgis_records(page_size)?;
    let mut dataset = ArcgisDataset::default();

    for record in records {
        match record {
            ArcgisRecord::Known(planet) => dataset.known.push(planet),
            ArcgisRecord::Unknown(planet) => dataset.unknown.push(planet),
        }
    }

    Ok(dataset)
}

/// Convert one ArcGIS attributes object into a classified normalized record.
pub fn normalize_arcgis_feature(raw: Value) -> Result<ArcgisRecord> {
    let fid = get_i64(&raw, &["FID", "OBJECTID", "ObjectId", "objectid"])
        .context("ArcGIS feature has no FID/OBJECTID")?;

    let planet = get_string(&raw, &["Planet", "PLANET", "Name", "NAME", "planet"])
        .unwrap_or_default();

    let region = get_string(&raw, &["Region", "REGION", "region"]).unwrap_or_default();
    let sector = get_string(&raw, &["Sector", "SECTOR", "sector"]).unwrap_or_default();
    let system = get_string(&raw, &["System", "SYSTEM", "system"]).unwrap_or_default();
    let grid = get_string(&raw, &["Grid", "GRID", "grid"]).unwrap_or_default();

    let x = get_f64(&raw, &["X", "x"]);
    let y = get_f64(&raw, &["Y", "y"]);

    let canon = get_boolish_i64(&raw, &["Canon", "CANON", "canon"]).unwrap_or(1);
    let legends = get_boolish_i64(&raw, &["Legends", "LEGENDS", "legends"]).unwrap_or(0);

    let planet = normalize_text(&planet);
    let planet_norm = build_planet_norm(&planet);

    let normalized = NormalizedPlanet {
        fid,
        planet_norm,
        planet,
        region: normalize_text(&region),
        sector: normalize_text(&sector),
        system: normalize_text(&system),
        grid: normalize_text(&grid),
        x,
        y,
        arcgis_hash: hash_source(&raw),
        canon,
        legends,
        raw,
    };

    if normalized.planet.trim().is_empty() {
        Ok(ArcgisRecord::Unknown(normalized))
    } else {
        Ok(ArcgisRecord::Known(normalized))
    }
}

fn get_value<'a>(raw: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| raw.get(*key))
}

fn get_string(raw: &Value, keys: &[&str]) -> Option<String> {
    let value = get_value(raw, keys)?;

    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(v) => Some(if *v { "1" } else { "0" }.to_string()),
        _ => None,
    }
}

fn get_i64(raw: &Value, keys: &[&str]) -> Option<i64> {
    let value = get_value(raw, keys)?;

    match value {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn get_f64(raw: &Value, keys: &[&str]) -> Option<f64> {
    let value = get_value(raw, keys)?;

    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn get_boolish_i64(raw: &Value, keys: &[&str]) -> Option<i64> {
    let value = get_value(raw, keys)?;

    match value {
        Value::Bool(v) => Some(i64::from(*v)),
        Value::Number(n) => n.as_i64(),
        Value::String(s) => match s.trim().to_lowercase().as_str() {
            "true" | "yes" | "y" | "1" => Some(1),
            "false" | "no" | "n" | "0" => Some(0),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_arcgis_feature_reads_pascal_case_fields_as_known() {
        let raw = json!({
            "FID": 42,
            "Planet": "  Coruscant ",
            "Region": "Core Worlds",
            "Sector": "Corusca Sector",
            "System": "Coruscant system",
            "Grid": "L-9",
            "X": 10.5,
            "Y": -20.25,
            "Canon": 1,
            "Legends": 0
        });

        let record = normalize_arcgis_feature(raw).expect("valid feature");

        match record {
            ArcgisRecord::Known(planet) => {
                assert_eq!(planet.fid, 42);
                assert_eq!(planet.planet, "Coruscant");
                assert_eq!(planet.planet_norm, "coruscant");
                assert_eq!(planet.grid, "L-9");
                assert_eq!(planet.x, Some(10.5));
                assert_eq!(planet.y, Some(-20.25));
            }
            ArcgisRecord::Unknown(_) => panic!("expected known record"),
        }
    }

    #[test]
    fn normalize_arcgis_feature_preserves_unnamed_records_as_unknown() {
        let raw = json!({
            "FID": 372,
            "Planet": "",
            "Region": "Unknown Regions",
            "Grid": "A-1",
            "X": 1.0,
            "Y": 2.0
        });

        let record = normalize_arcgis_feature(raw).expect("valid feature");

        match record {
            ArcgisRecord::Unknown(planet) => {
                assert_eq!(planet.fid, 372);
                assert_eq!(planet.planet, "");
                assert_eq!(planet.planet_norm, "");
                assert_eq!(planet.grid, "A-1");
            }
            ArcgisRecord::Known(_) => panic!("expected unknown record"),
        }
    }
}
