//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::types::portal_half_extent;

fn floor(pos: Vec2) -> PlacedPortal {
    PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Indexed(0)),
        pos,
        Vec2::new(0.0, -1.0),
        portal_half_extent(Vec2::new(0.0, -1.0)),
    )
}

fn app() -> App {
    let mut app = App::new();
    app.add_systems(
        Update,
        (resolve_portal_links, equalize_pair_apertures).chain(),
    );
    app
}

#[test]
fn same_link_pairs_by_position_distinct_partners() {
    let mut app = app();
    let a = app
        .world_mut()
        .spawn((
            PortalLink(link_hash("door")),
            floor(Vec2::new(100.0, 300.0)),
        ))
        .id();
    let b = app
        .world_mut()
        .spawn((
            PortalLink(link_hash("door")),
            floor(Vec2::new(500.0, 300.0)),
        ))
        .id();
    app.update();
    let ca = app.world().get::<PlacedPortal>(a).unwrap().channel;
    let cb = app.world().get::<PlacedPortal>(b).unwrap().channel;
    assert_ne!(ca, cb, "the two ends get distinct channels");
    assert_eq!(ca.partner(), cb, "and they are each other's partners");
    assert_eq!(cb.partner(), ca);
}

#[test]
fn wrong_arity_link_is_closed() {
    let mut app = app();
    // THREE portals share a link → all closed (no member has a partner).
    let es: Vec<_> = [100.0, 300.0, 500.0]
        .iter()
        .map(|x| {
            app.world_mut()
                .spawn((PortalLink(link_hash("trio")), floor(Vec2::new(*x, 300.0))))
                .id()
        })
        .collect();
    // A lone portal on another link → also closed.
    let lone = app
        .world_mut()
        .spawn((
            PortalLink(link_hash("solo")),
            floor(Vec2::new(700.0, 300.0)),
        ))
        .id();
    app.update();
    let all: Vec<PlacedPortal> = {
        let mut q = app.world_mut().query::<&PlacedPortal>();
        q.iter(app.world()).cloned().collect()
    };
    for e in es.iter().chain(std::iter::once(&lone)) {
        let c = app.world().get::<PlacedPortal>(*e).unwrap().channel;
        assert!(
            !all.iter().any(|p| p.channel == c.partner()),
            "closed: no partner exists for {c:?}"
        );
    }
}

#[test]
fn aperture_shrinks_to_the_pair_minimum() {
    let mut app = app();
    let mut big = floor(Vec2::new(100.0, 300.0));
    big.half_extent = portal_half_extent_with_length(big.normal, 80.0); // wide
    let mut small = floor(Vec2::new(500.0, 300.0));
    small.half_extent = portal_half_extent_with_length(small.normal, 30.0); // narrow
    let a = app
        .world_mut()
        .spawn((PortalLink(link_hash("d")), big))
        .id();
    let b = app
        .world_mut()
        .spawn((PortalLink(link_hash("d")), small))
        .id();
    app.update();
    // Both ends now open the minimum (30), centered (pos unchanged).
    for e in [a, b] {
        let p = app.world().get::<PlacedPortal>(e).unwrap();
        assert!(
            (portal_opening_half(p.normal, p.half_extent) - 30.0).abs() < 1e-3,
            "opening shrank to min: {:?}",
            p.half_extent
        );
    }
}
