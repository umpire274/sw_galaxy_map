use sw_galaxy_map::routing::geometry::Point;
use sw_galaxy_map::routing::router::{RouteOptions, compute_route};

#[test]
fn direct_route_without_obstacles() {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(10.0, 0.0);

    let route =
        compute_route(start, end, &[], RouteOptions::default()).expect("route computation failed");

    assert_eq!(route.waypoints.len(), 2);
    assert_eq!(route.waypoints[0], start);
    assert_eq!(route.waypoints[1], end);
    assert!(route.length > 0.0);
}
