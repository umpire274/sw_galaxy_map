use crate::routing::collision::{Hit, Obstacle, first_collision_on_segment};
use crate::routing::geometry::Point;
use crate::routing::router::Route;

#[cfg(debug_assertions)]
pub fn debug_failed_detour(
    a: Point,
    b: Point,
    hit: &Hit,
    candidates: &[Point],
    obstacles: &[Obstacle],
) {
    eprintln!(
        "DET0UR FAIL obstacle={} center=({:.4},{:.4}) r={:.4} d={:.4} A=({:.4},{:.4}) B=({:.4},{:.4})",
        hit.obstacle_id,
        hit.obstacle_center.x,
        hit.obstacle_center.y,
        hit.obstacle_radius,
        hit.closest.dist,
        a.x,
        a.y,
        b.x,
        b.y
    );

    for (i, w) in candidates.iter().enumerate() {
        let col1 = first_collision_on_segment(a, *w, obstacles).map(|h| h.obstacle_id);
        let col2 = first_collision_on_segment(*w, b, obstacles).map(|h| h.obstacle_id);

        eprintln!(
            "  cand #{:02} W=({:.4},{:.4}) col(A-W)={:?} col(W-B)={:?}",
            i, w.x, w.y, col1, col2
        );
    }
}

#[cfg(debug_assertions)]
pub fn debug_print_route(route: &Route) {
    use std::fmt::Write;

    let mut out = String::new();

    let _ = writeln!(&mut out, "================ ROUTE DEBUG ================");
    let _ = writeln!(
        &mut out,
        "Waypoints: {} | Length: {:.3} | Iterations: {} | Detours: {}",
        route.waypoints.len(),
        route.length,
        route.iterations,
        route.detours.len()
    );

    let _ = writeln!(&mut out, "Waypoints (polyline):");
    for (i, p) in route.waypoints.iter().enumerate() {
        let _ = writeln!(&mut out, "  {:>2}: ({:.6}, {:.6})", i, p.x, p.y);
    }

    if route.detours.is_empty() {
        let _ = writeln!(&mut out, "Detours: -");
        let _ = writeln!(&mut out, "================================================");
        eprintln!("{out}");
        return;
    }

    let _ = writeln!(&mut out, "Detours (decisions):");
    for d in &route.detours {
        let total = d.score.total();

        let _ = writeln!(
            &mut out,
            "  - detour #{:>2} | seg_idx={} | obstacle_id={} | offset={:.3}",
            d.iteration, d.segment_index, d.obstacle_id, d.offset_used
        );

        let _ = writeln!(
            &mut out,
            "    obstacle: center=({:.6},{:.6}) radius={:.3}",
            d.obstacle_center.x, d.obstacle_center.y, d.obstacle_radius
        );

        let _ = writeln!(
            &mut out,
            "    collision: t={:.4} Q=({:.6},{:.6}) dist={:.6}",
            d.closest_t, d.closest_q.x, d.closest_q.y, d.closest_dist
        );

        let _ = writeln!(
            &mut out,
            "    chosen wp: ({:.6},{:.6})",
            d.waypoint.x, d.waypoint.y
        );

        let _ = writeln!(
            &mut out,
            "    score: total={:.6} | base={:.6} turn={:.6} back={:.6} prox={:.6}",
            total, d.score.base, d.score.turn, d.score.back, d.score.proximity
        );
    }

    let _ = writeln!(&mut out, "================================================");

    eprintln!("{out}");
}
