// src/cli/validate.rs
use anyhow::{Result, bail};

pub const TIP_NEGATIVE_COORDS: &str =
    "Note: for negative coordinates, use the '=' form, e.g.:\n  near --r 50 --x=7100 --y=-190";

pub fn validate_near(
    unknown: bool,
    fid: &Option<i64>,
    planet: &Option<String>,
    x: &Option<f64>,
    y: &Option<f64>,
) -> Result<()> {
    if unknown {
        if fid.is_none() {
            bail!("--fid is required when using --unknown.");
        }
        if planet.is_some() || x.is_some() || y.is_some() {
            bail!("--unknown cannot be combined with --planet, --x, or --y.");
        }
        return Ok(());
    }

    if fid.is_some() {
        bail!("--fid can only be used with --unknown.");
    }

    if planet.is_some() {
        return Ok(());
    }
    if x.is_some() && y.is_some() {
        return Ok(());
    }

    bail!(
        "You must specify either:\n\
         \n\
         - --planet <NAME>\n\
         \n\
         OR\n\
         \n\
         - --x=<VALUE> --y=<VALUE>\n\
         \n\
         {TIP_NEGATIVE_COORDS}"
    )
}

pub fn validate_search(query: &str, limit: i64) -> Result<()> {
    if query.trim().is_empty() {
        bail!("Search query cannot be empty.");
    }
    if limit <= 0 {
        bail!("--limit must be > 0.");
    }
    Ok(())
}

pub fn validate_route_id(route_id: i64, ctx: &str) -> anyhow::Result<()> {
    if route_id <= 0 {
        anyhow::bail!("Invalid route id for {ctx}: {route_id} (must be > 0)");
    }
    Ok(())
}

pub fn validate_route_compute(from: &str, to: &str) -> anyhow::Result<()> {
    let f = from.trim();
    let t = to.trim();

    if f.is_empty() || t.is_empty() {
        anyhow::bail!("FROM and TO must be non-empty");
    }
    if f.eq_ignore_ascii_case(t) {
        anyhow::bail!("FROM and TO must be different");
    }
    Ok(())
}

pub fn validate_route_planets(planets: &[String]) -> anyhow::Result<()> {
    if planets.len() < 2 {
        anyhow::bail!("Route compute requires at least two planets.");
    }
    for (idx, planet) in planets.iter().enumerate() {
        if planet.trim().is_empty() {
            anyhow::bail!("Planet {} cannot be empty.", idx + 1);
        }
    }
    for window in planets.windows(2) {
        let from = window[0].trim();
        let to = window[1].trim();
        if from.eq_ignore_ascii_case(to) {
            anyhow::bail!("Adjacent planets must be different ({} → {}).", from, to);
        }
    }
    Ok(())
}

pub fn validate_limit(limit: i64, ctx: &str) -> anyhow::Result<()> {
    if limit <= 0 {
        anyhow::bail!("Invalid limit for {ctx}: {limit} (must be > 0)");
    }
    Ok(())
}
