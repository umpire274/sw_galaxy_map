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
            bail!("--fid is required with --unknown");
        }

        if planet.is_some() || x.is_some() || y.is_some() {
            bail!("When using --unknown, do not specify a planet name or --x/--y coordinates.");
        }

        return Ok(());
    }

    if fid.is_some() {
        bail!("--fid can only be used with --unknown.");
    }

    if planet.is_some() {
        if x.is_some() || y.is_some() {
            bail!("Specify either a planet name or --x/--y coordinates, not both.");
        }

        return Ok(());
    }

    if x.is_some() && y.is_some() {
        return Ok(());
    }

    if x.is_some() || y.is_some() {
        bail!("You must specify both --x and --y coordinates.\n\n{TIP_NEGATIVE_COORDS}");
    }

    bail!(
        "You must specify either:\n\
         \n\
         - <PLANET_NAME>\n\
         \n\
         OR\n\
         \n\
         - --x=<VALUE> --y=<VALUE>\n\
         \n\
         OR\n\
         \n\
         - --unknown --fid <FID>\n\
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

pub fn validate_route_id(route_id: i64, ctx: &str) -> Result<()> {
    if route_id <= 0 {
        bail!("Invalid route id for {ctx}: {route_id} (must be > 0)");
    }
    Ok(())
}

pub fn validate_route_compute(from: &str, to: &str) -> Result<()> {
    let f = from.trim();
    let t = to.trim();

    if f.is_empty() || t.is_empty() {
        bail!("FROM and TO must be non-empty");
    }
    if f.eq_ignore_ascii_case(t) {
        bail!("FROM and TO must be different");
    }
    Ok(())
}

pub fn validate_route_planets(planets: &[String]) -> Result<()> {
    if planets.len() < 2 {
        bail!("Route compute requires at least two planets.");
    }
    for (idx, planet) in planets.iter().enumerate() {
        if planet.trim().is_empty() {
            bail!("Planet {} cannot be empty.", idx + 1);
        }
    }
    for window in planets.windows(2) {
        let from = window[0].trim();
        let to = window[1].trim();
        if from.eq_ignore_ascii_case(to) {
            bail!("Adjacent planets must be different ({} → {}).", from, to);
        }
    }
    Ok(())
}

pub fn validate_limit(limit: i64, ctx: &str) -> Result<()> {
    if limit <= 0 {
        bail!("Invalid limit for {ctx}: {limit} (must be > 0)");
    }
    Ok(())
}
