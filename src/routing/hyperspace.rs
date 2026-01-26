//! Hyperspace travel time estimation.
//!
//! This module implements a simple, configurable model for estimating
//! hyperspace travel time starting from a Euclidean route length in parsecs.
//!
//! Core formula (hours):
//! `time = distance_parsec / compression_factor / hyperdrive_class`
//!
//! Notes:
//! - `compression_factor` models how much a hyperspace route "compresses" real
//!   space distance, and is typically derived from the galactic region.
//! - Detours can further reduce the effective compression factor via a penalty
//!   multiplier.

use crate::model::Planet;

/// Ordered from most internal to most external.
///
/// Values (base compression factors) are user-provided and are intended to be
/// tuned without recompiling (e.g., via config) in future iterations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GalacticRegion {
    DeepCore,
    CoreWorlds,
    Colonies,
    InnerRim,
    ExpansionRegion,
    MidRim,
    HuttSpace,
    OuterRim,
    WildSpace,
    UnknownRegions,
}

impl GalacticRegion {
    /// Base hyperspace compression factor for the region.
    /// Higher means faster travel for the same real-space distance.
    pub fn base_compression_factor(self) -> f64 {
        match self {
            GalacticRegion::DeepCore => 50.0,
            GalacticRegion::CoreWorlds => 45.0,
            GalacticRegion::Colonies => 40.0,
            GalacticRegion::InnerRim => 35.0,
            GalacticRegion::ExpansionRegion => 30.0,
            GalacticRegion::MidRim => 25.0,
            GalacticRegion::HuttSpace => 22.0,
            GalacticRegion::OuterRim => 18.0,
            GalacticRegion::WildSpace => 15.0,
            GalacticRegion::UnknownRegions => 15.0,
        }
    }

    /// Best-effort parsing from strings found in the dataset (case-insensitive).
    ///
    /// This is intentionally permissive (underscores, multiple spaces, etc.)
    /// to accommodate heterogeneous sources.
    pub fn parse(s: &str) -> Option<Self> {
        let norm = normalize_region_name(s);
        match norm.as_str() {
            "deep core" => Some(Self::DeepCore),
            "core worlds" | "core" => Some(Self::CoreWorlds),
            "colonies" => Some(Self::Colonies),
            "inner rim" => Some(Self::InnerRim),
            "expansion region" | "expansion" => Some(Self::ExpansionRegion),
            "mid rim" | "mid-rim" => Some(Self::MidRim),
            "hutt space" => Some(Self::HuttSpace),
            "outer rim" | "outer-rim" => Some(Self::OuterRim),
            "wild space" => Some(Self::WildSpace),
            "unknown regions" | "unknown region" => Some(Self::UnknownRegions),
            _ => None,
        }
    }
}

pub fn extract_galactic_region(p: &Planet) -> Option<GalacticRegion> {
    parse_first_region(&[
        p.c_region.as_deref(),
        p.c_region_li.as_deref(),
        p.region.as_deref(),
    ])
}

pub fn parse_first_region(candidates: &[Option<&str>]) -> Option<GalacticRegion> {
    candidates
        .iter()
        .copied()
        .flatten()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .find_map(GalacticRegion::parse)
}

pub fn weighted_average_compression(segments: &[(f64, f64)]) -> Option<f64> {
    // segments: vec di (segment_length, compression_factor)
    let mut num = 0.0;
    let mut den = 0.0;

    for (len, cf) in segments {
        if *len > 0.0 && *cf > 0.0 {
            num += len * cf;
            den += len;
        }
    }

    if den > 0.0 { Some(num / den) } else { None }
}

fn normalize_region_name(s: &str) -> String {
    // Lowercase + trim + normalize separators.
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.trim().chars() {
        let ch = ch.to_ascii_lowercase();
        let mapped = match ch {
            '_' | '-' => ' ',
            _ => ch,
        };
        if mapped.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(mapped);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

/// Parameters controlling how detours penalize a route.
#[derive(Debug, Clone, Copy)]
pub struct DetourPenaltyParams {
    /// Penalty strength.
    ///
    /// - `0.0` ignores detours completely.
    /// - `1.0` applies a strong penalty for modest detours.
    pub weight: f64,

    /// Cap the detour ratio (route/direct) to avoid extreme outliers dominating.
    pub max_ratio: f64,

    /// Lower bound for the returned multiplier.
    ///
    /// Example: `0.2` means "even with huge detours, don't reduce compression below 20%".
    pub floor: f64,
}

impl Default for DetourPenaltyParams {
    fn default() -> Self {
        Self {
            weight: 0.6,
            max_ratio: 2.5,
            floor: 0.2,
        }
    }
}

/// Compute a multiplier in (0, 1] that reduces the effective compression factor
/// based on how much longer the routed path is compared to the direct segment.
///
/// This models the intuition that detours typically force the ship to leave
/// optimal hyperspace corridors.
pub fn detour_penalty_multiplier(
    direct_distance_parsec: f64,
    route_distance_parsec: f64,
    params: DetourPenaltyParams,
) -> f64 {
    assert!(
        direct_distance_parsec > 0.0,
        "direct_distance_parsec must be > 0"
    );
    assert!(
        route_distance_parsec >= direct_distance_parsec,
        "route must be >= direct"
    );
    assert!(params.weight >= 0.0, "weight must be >= 0");
    assert!(params.max_ratio >= 1.0, "max_ratio must be >= 1");
    assert!(
        params.floor > 0.0 && params.floor <= 1.0,
        "floor must be in (0,1]"
    );

    let ratio = (route_distance_parsec / direct_distance_parsec).min(params.max_ratio);

    // Linear penalty:
    // ratio=1 => 1.0
    // ratio=2 => 1 - weight
    let mult = 1.0 - params.weight * (ratio - 1.0);

    mult.clamp(params.floor, 1.0)
}

/// Estimate hyperspace travel time (in hours) for a given distance.
pub fn estimate_travel_time_hours(
    distance_parsec: f64,
    compression_factor: f64,
    hyperdrive_class: f64,
) -> f64 {
    assert!(compression_factor > 0.0, "compression_factor must be > 0");
    assert!(hyperdrive_class > 0.0, "hyperdrive_class must be > 0");

    (distance_parsec / compression_factor) * hyperdrive_class
}

/// Convenience helper: derive an effective compression factor from a region
/// and detour penalty.
pub fn effective_compression_factor(region: GalacticRegion, detour_multiplier: f64) -> f64 {
    assert!(detour_multiplier > 0.0, "detour_multiplier must be > 0");
    region.base_compression_factor() * detour_multiplier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_parse_is_permissive() {
        assert_eq!(
            GalacticRegion::parse("Outer Rim"),
            Some(GalacticRegion::OuterRim)
        );
        assert_eq!(
            GalacticRegion::parse("outer-rim"),
            Some(GalacticRegion::OuterRim)
        );
        assert_eq!(
            GalacticRegion::parse("  CORE__WORLDS "),
            Some(GalacticRegion::CoreWorlds)
        );
        assert_eq!(
            GalacticRegion::parse("Unknown Region"),
            Some(GalacticRegion::UnknownRegions)
        );
        assert_eq!(GalacticRegion::parse("n/a"), None);
    }

    #[test]
    fn detour_penalty_multiplier_behaves() {
        let p = DetourPenaltyParams {
            weight: 0.6,
            max_ratio: 2.5,
            floor: 0.2,
        };
        let m1 = detour_penalty_multiplier(100.0, 100.0, p);
        assert!((m1 - 1.0).abs() < 1e-9);

        let m2 = detour_penalty_multiplier(100.0, 150.0, p); // ratio=1.5
        // 1 - 0.6*(0.5)=0.7
        assert!((m2 - 0.7).abs() < 1e-9);

        let m3 = detour_penalty_multiplier(100.0, 300.0, p); // ratio=3.0 capped to 2.5
        // 1 - 0.6*(1.5)=0.1 -> clamped to floor 0.2
        assert!((m3 - 0.2).abs() < 1e-9);
    }

    #[test]
    fn estimate_time_is_consistent() {
        // Example: 14757.761 parsec, Outer Rim (18.0), detour multiplier 0.85, class 1.0
        let distance = 14_757.761;
        let region = GalacticRegion::OuterRim;
        let detour_mult = 0.85;
        let cf = effective_compression_factor(region, detour_mult);
        let hours = estimate_travel_time_hours(distance, cf, 1.0);
        // 14757.761 / (18*0.85) ≈ 964.6
        assert!((hours - 964.6).abs() < 1.0);
    }
}
