//! Sublight travel time estimation.
//!
//! This module provides simple helpers to estimate sublight travel time
//! from a distance expressed in parsecs.
//!
//! Star Wars is inconsistent about sublight speeds; this intentionally keeps
//! the model explicit (you must provide a speed).

/// Kilometers in one parsec.
///
/// 1 pc = 3.0856775814913673e13 km
pub const PARSEC_KM: f64 = 3.085_677_581_491_367e13;

/// Convert parsecs to kilometers.
#[inline]
pub fn parsec_to_km(parsec: f64) -> f64 {
    parsec * PARSEC_KM
}

/// Estimate sublight travel time (in hours) for a given distance.
///
/// # Panics
/// Panics if `speed_km_s <= 0.0`.
pub fn estimate_sublight_time_hours(distance_parsec: f64, speed_km_s: f64) -> f64 {
    assert!(speed_km_s > 0.0, "speed_km_s must be > 0");
    let km = parsec_to_km(distance_parsec);
    let seconds = km / speed_km_s;
    seconds / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_mars_order_of_magnitude() {
        // ~360 million km ~= 1.166e-5 pc
        let pc = 360_000_000.0 / PARSEC_KM;
        let h = estimate_sublight_time_hours(pc, 2000.0);
        // 360e6 km / 2000 km/s = 180_000 s = 50 h
        assert!((h - 50.0).abs() < 0.5);
    }
}
