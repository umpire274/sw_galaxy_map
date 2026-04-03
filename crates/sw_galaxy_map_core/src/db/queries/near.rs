use crate::model::NearHit;
use anyhow::Result;
use rusqlite::{Connection, params};

/// Returns planets near the given coordinates within the specified radius.
pub fn near_planets(con: &Connection, x: f64, y: f64, r: f64, limit: i64) -> Result<Vec<NearHit>> {
    if !x.is_finite() || !y.is_finite() {
        anyhow::bail!("Center coordinates must be finite numbers");
    }
    if !r.is_finite() || r < 0.0 {
        anyhow::bail!("Radius must be a finite number >= 0");
    }
    if limit <= 0 {
        return Ok(Vec::new());
    }

    let r2 = r * r;

    let mut stmt = con.prepare(
        r#"
        SELECT FID, Planet, X, Y,
               ((X - ?1)*(X - ?1) + (Y - ?2)*(Y - ?2)) AS d2
        FROM planets
        WHERE ((X - ?1)*(X - ?1) + (Y - ?2)*(Y - ?2)) <= ?3
        ORDER BY d2 ASC
        LIMIT ?4
        "#,
    )?;

    let rows = stmt
        .query_map(params![x, y, r2, limit], |r| {
            let fid: i64 = r.get(0)?;
            let planet: String = r.get(1)?;
            let px: f64 = r.get(2)?;
            let py: f64 = r.get(3)?;
            let d2: f64 = r.get(4)?;
            Ok(NearHit {
                fid,
                planet,
                x: px,
                y: py,
                distance: d2.sqrt(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Returns planets near the given coordinates, excluding the center planet FID.
pub fn near_planets_excluding_fid(
    con: &Connection,
    center_fid: i64,
    x: f64,
    y: f64,
    r: f64,
    limit: i64,
) -> Result<Vec<NearHit>> {
    if !x.is_finite() || !y.is_finite() {
        anyhow::bail!("Center coordinates must be finite numbers");
    }
    if !r.is_finite() || r < 0.0 {
        anyhow::bail!("Radius must be a finite number >= 0");
    }
    if limit <= 0 {
        return Ok(Vec::new());
    }

    let r2 = r * r;

    let mut stmt = con.prepare(
        r#"
        SELECT FID, Planet, X, Y,
               ((X - ?2)*(X - ?2) + (Y - ?3)*(Y - ?3)) AS d2
        FROM planets
        WHERE FID != ?1
          AND ((X - ?2)*(X - ?2) + (Y - ?3)*(Y - ?3)) <= ?4
        ORDER BY d2 ASC
        LIMIT ?5
        "#,
    )?;

    let rows = stmt
        .query_map(params![center_fid, x, y, r2, limit], |r| {
            let fid: i64 = r.get(0)?;
            let planet: String = r.get(1)?;
            let px: f64 = r.get(2)?;
            let py: f64 = r.get(3)?;
            let d2: f64 = r.get(4)?;
            Ok(NearHit {
                fid,
                planet,
                x: px,
                y: py,
                distance: d2.sqrt(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}
