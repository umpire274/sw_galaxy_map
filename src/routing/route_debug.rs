// src/routing/route_debug.rs

use crate::routing::collision::{Hit, Obstacle};
use crate::routing::geometry::Point;
use crate::routing::router::Route;

/// Debug helper: called when the router cannot find a valid detour.
/// In release builds this is a no-op (keeps API stable).
#[inline]
pub fn debug_failed_detour(
    a: Point,
    b: Point,
    hit: &Hit,
    candidates: &[Point],
    obstacles: &[Obstacle],
) {
    // Debug-only implementation
    #[cfg(debug_assertions)]
    {
        // --- YOUR EXISTING DEBUG IMPLEMENTATION HERE ---
        // Example placeholder:
        eprintln!("DEBUG: failed detour");
        eprintln!(
            "  segment A=({:.3},{:.3}) B=({:.3},{:.3})",
            a.x, a.y, b.x, b.y
        );
        eprintln!(
            "  obstacle id={} radius={:.3}",
            hit.obstacle_id, hit.obstacle_radius
        );
        eprintln!("  candidates={}", candidates.len());
        eprintln!("  obstacles={}", obstacles.len());
    }

    // Release no-op
    #[cfg(not(debug_assertions))]
    {
        let _ = (a, b, hit, candidates, obstacles);
    }
}

/// Debug helper: prints a computed route.
/// In release builds this is a no-op (keeps API stable).
#[inline]
pub fn debug_print_route(route: &Route) {
    #[cfg(debug_assertions)]
    {
        // --- YOUR EXISTING DEBUG IMPLEMENTATION HERE ---
        // Example placeholder:
        eprintln!(
            "DEBUG: route: waypoints={} length={:.3} iters={}",
            route.waypoints.len(),
            route.length,
            route.iterations
        );
    }

    #[cfg(not(debug_assertions))]
    {
        let _ = route;
    }
}
