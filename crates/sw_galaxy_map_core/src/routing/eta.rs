//! Route ETA estimation.
//!
//! This module computes a structured hyperspace ETA estimate for a persisted
//! route (`RouteLoaded`) using:
//!
//! - polyline route length
//! - direct endpoint distance
//! - detour geometry penalty
//! - detour count penalty
//! - detour severity penalty
//! - endpoint galactic regions
//! - region blending policy
//! - hyperdrive class
//!
//! The goal is to keep the ETA model reusable from CLI, TUI and GUI without
//! forcing presentation-specific formatting into the core.

use rusqlite::Connection;

use crate::db::queries;
use crate::model::RouteLoaded;
use crate::routing::geometry::{Point, dist as geom_dist, polyline_length_waypoints_parsec};
use crate::routing::hyperspace::{
    DetourPenaltyParams, GalacticRegion, detour_penalty_multiplier, estimate_travel_time_hours,
    extract_galactic_region,
};

/// Region blending policy used to derive the base compression factor from the
/// two endpoint regions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegionBlend {
    /// Mean of endpoint compression factors.
    Avg,
    /// Slightly favors the destination region.
    Conservative,
    /// Weighted interpolation where `1.0` means "fully from origin" and `0.0`
    /// means "fully from destination".
    Weighted(f64),
}

/// Structured ETA estimate for a persisted route.
#[derive(Debug, Clone)]
pub struct RouteEtaEstimate {
    pub route_length_parsec: f64,
    pub direct_length_parsec: f64,

    pub from_region: GalacticRegion,
    pub to_region: GalacticRegion,
    pub blend: RegionBlend,

    pub hyperdrive_class: f64,

    pub detour_count: usize,
    pub severity_sum: f64,

    pub detour_multiplier_geom: f64,
    pub detour_multiplier_count: f64,
    pub detour_multiplier_severity: f64,
    pub detour_multiplier_total: f64,

    pub base_compression_factor: f64,
    pub effective_compression_factor: f64,

    pub eta_hours: f64,
    pub eta_days: f64,
}

impl RouteEtaEstimate {
    /// Returns a compact human-readable ETA string.
    pub fn format_human(&self) -> String {
        format!("{:.1} h (~{:.1} d)", self.eta_hours, self.eta_days)
    }
}

/// Estimates the ETA of a persisted route.
///
/// Returns `None` when:
/// - the route has fewer than 2 waypoints
/// - invalid numeric inputs are provided
/// - endpoint planets cannot be loaded
/// - route/direct geometry is degenerate
pub fn estimate_route_eta(
    con: &Connection,
    loaded: &RouteLoaded,
    hyperdrive_class: f64,
    blend: RegionBlend,
    detour_count_base: f64,
    severity_k: f64,
) -> Option<RouteEtaEstimate> {
    if loaded.waypoints.len() < 2 {
        return None;
    }

    if hyperdrive_class <= 0.0 || detour_count_base <= 0.0 || severity_k < 0.0 {
        return None;
    }

    let route_length_parsec: f64 =
        polyline_length_waypoints_parsec(&loaded.waypoints, |w| (w.x, w.y));

    let start = loaded.waypoints.first()?;
    let end = loaded.waypoints.last()?;
    let direct_length_parsec = geom_dist(Point::new(start.x, start.y), Point::new(end.x, end.y));

    if route_length_parsec <= 0.0 || direct_length_parsec <= 0.0 {
        return None;
    }

    let detour_params = DetourPenaltyParams::default();
    let detour_multiplier_geom =
        detour_penalty_multiplier(direct_length_parsec, route_length_parsec, detour_params);

    let detour_count = loaded.detours.len();
    let detour_multiplier_count = detour_count_base.powi(detour_count as i32);

    let mut severity_sum: f64 = loaded
        .detours
        .iter()
        .map(|d| {
            let required = d.offset_used.max(1e-9);
            ((required - d.closest_dist) / required).clamp(0.0, 1.0)
        })
        .sum();

    if severity_sum.abs() < 1e-12 {
        severity_sum = 0.0;
    }

    let detour_multiplier_severity = 1.0 / (1.0 + severity_k * severity_sum);

    let detour_multiplier_total =
        (detour_multiplier_geom * detour_multiplier_count * detour_multiplier_severity)
            .clamp(detour_params.floor, 1.0);

    let from_planet = queries::get_planet_by_fid(con, loaded.route.from_planet_fid).ok()??;
    let to_planet = queries::get_planet_by_fid(con, loaded.route.to_planet_fid).ok()??;

    let from_region = extract_galactic_region(&from_planet).unwrap_or(GalacticRegion::OuterRim);
    let to_region = extract_galactic_region(&to_planet).unwrap_or(GalacticRegion::OuterRim);

    let cf_from = from_region.base_compression_factor();
    let cf_to = to_region.base_compression_factor();

    let base_compression_factor = match blend {
        RegionBlend::Avg => (cf_from + cf_to) / 2.0,
        RegionBlend::Conservative => cf_from * 0.4 + cf_to * 0.6,
        RegionBlend::Weighted(w) => {
            let w = w.clamp(0.0, 1.0);
            cf_from * w + cf_to * (1.0 - w)
        }
    };

    let effective_compression_factor = (base_compression_factor * detour_multiplier_total).max(5.0);

    let eta_hours = estimate_travel_time_hours(
        route_length_parsec,
        effective_compression_factor,
        hyperdrive_class,
    );

    Some(RouteEtaEstimate {
        route_length_parsec,
        direct_length_parsec,

        from_region,
        to_region,
        blend,

        hyperdrive_class,

        detour_count,
        severity_sum,

        detour_multiplier_geom,
        detour_multiplier_count,
        detour_multiplier_severity,
        detour_multiplier_total,

        base_compression_factor,
        effective_compression_factor,

        eta_hours,
        eta_days: eta_hours / 24.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_format_is_stable() {
        let eta = RouteEtaEstimate {
            route_length_parsec: 100.0,
            direct_length_parsec: 90.0,
            from_region: GalacticRegion::CoreWorlds,
            to_region: GalacticRegion::CoreWorlds,
            blend: RegionBlend::Avg,
            hyperdrive_class: 1.0,
            detour_count: 0,
            severity_sum: 0.0,
            detour_multiplier_geom: 1.0,
            detour_multiplier_count: 1.0,
            detour_multiplier_severity: 1.0,
            detour_multiplier_total: 1.0,
            base_compression_factor: 45.0,
            effective_compression_factor: 45.0,
            eta_hours: 12.34,
            eta_days: 12.34 / 24.0,
        };

        assert_eq!(eta.format_human(), "12.3 h (~0.5 d)");
    }
}
