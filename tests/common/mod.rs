use sw_galaxy_map::routing::collision::Obstacle;
use sw_galaxy_map::routing::collision::first_collision_on_segment;
use sw_galaxy_map::routing::router::Route;

pub fn assert_collision_free(route: &Route, obstacles: &[Obstacle]) {
    for seg in route.waypoints.windows(2) {
        let a = seg[0];
        let b = seg[1];

        let hit = first_collision_on_segment(a, b, obstacles);
        assert!(
            hit.is_none(),
            "Collision left in route: {:?} on segment A={:?} B={:?}",
            hit,
            a,
            b
        );
    }
}
