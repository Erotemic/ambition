use super::*;

#[derive(Clone)]
struct Object {
    id: usize,
    bounds: Aabb,
    category: u8,
}
impl Spatial for Object {
    fn spatial_bounds(&self) -> Aabb {
        self.bounds
    }
}
struct Category(u8);
impl SpatialFilter<Object, u8> for Category {
    fn accepts(&mut self, item: &Object) -> bool {
        item.category & self.0 != 0
    }
    fn may_match(&mut self, summary: &u8) -> bool {
        summary & self.0 != 0
    }
}
fn distance(point: Vec2, bounds: Aabb) -> f32 {
    let dx = (bounds.min.x - point.x)
        .max(point.x - bounds.max.x)
        .max(0.0);
    let dy = (bounds.min.y - point.y)
        .max(point.y - bounds.max.y)
        .max(0.0);
    dx * dx + dy * dy
}

#[test]
fn moving_bounds_and_filtered_nearest_match_exhaustive_queries() {
    let mut tree = SpatialTree::default();
    for tick in 0..24 {
        let items: Vec<_> = (0..(tick * 17 % 113))
            .map(|id| {
                let center = Vec2::new(
                    ((id * 71 + tick * 19) % 997) as f32 - 500.0,
                    ((id * 137 + tick * 31) % 863) as f32 - 400.0,
                );
                Object {
                    id,
                    bounds: Aabb::new(center, Vec2::splat((id % 4) as f32 * 3.0)),
                    category: 1 << (id % 3),
                }
            })
            .collect();
        tree.rebuild_with_summary(items.clone(), |o| o.category, |all, other| *all | other)
            .unwrap();
        for query in 0..31 {
            let point = Vec2::new(query as f32 * 35.0 - 450.0, tick as f32 * 17.0 - 200.0);
            let mask = 1 << (query % 3);
            let expected = items
                .iter()
                .enumerate()
                .filter(|(_, o)| o.category & mask != 0)
                .map(|(index, o)| (index, distance(point, o.bounds)))
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                .map(|(index, d)| (items[index].id, d));
            assert_eq!(
                tree.nearest_filtered(point, &mut Category(mask))
                    .map(|(o, d)| (o.id, d)),
                expected
            );
            let bounds = Aabb::new(point, Vec2::new(65.0, 37.0));
            let mut actual = Vec::new();
            tree.query_bounds(bounds, |o| actual.push(o.id));
            actual.sort();
            let expected: Vec<_> = items
                .iter()
                .filter(|o| {
                    o.bounds.min.x <= bounds.max.x
                        && o.bounds.max.x >= bounds.min.x
                        && o.bounds.min.y <= bounds.max.y
                        && o.bounds.max.y >= bounds.min.y
                })
                .map(|o| o.id)
                .collect();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn nearest_ties_use_input_order_and_empty_or_invalid_rebuilds_retire_old_rows() {
    let mut tree = SpatialTree::default();
    tree.rebuild([
        Aabb::new(Vec2::X, Vec2::ZERO),
        Aabb::new(-Vec2::X, Vec2::ZERO),
    ])
    .unwrap();
    assert_eq!(tree.nearest(Vec2::ZERO, |_| true).unwrap().0.min, Vec2::X);
    assert!(tree.nearest(Vec2::splat(f32::NAN), |_| true).is_none());
    let mut touching = 0;
    tree.query_bounds(Aabb::new(Vec2::ZERO, Vec2::ONE), |_| touching += 1);
    assert_eq!(touching, 2);
    assert_eq!(
        tree.rebuild([Aabb {
            min: Vec2::ONE,
            max: Vec2::ZERO
        }]),
        Err(InvalidSpatialBounds { index: 0 })
    );
    assert!(tree.is_empty());
    assert!(tree.nearest(Vec2::ZERO, |_| true).is_none());
    tree.rebuild([Aabb::new(Vec2::ZERO, Vec2::ONE)]).unwrap();
    tree.rebuild([]).unwrap();
    assert!(tree.is_empty());
}

#[test]
fn a_tree_can_borrow_the_callers_objects() {
    let bounds = [
        Aabb::new(Vec2::ZERO, Vec2::ONE),
        Aabb::new(Vec2::splat(10.0), Vec2::ONE),
    ];
    let mut tree = SpatialTree::default();
    tree.rebuild(bounds.iter()).unwrap();
    let (found, distance) = tree.nearest(Vec2::splat(10.0), |_| true).unwrap();
    assert!(std::ptr::eq(*found, &bounds[1]));
    assert_eq!(distance, 0.0);
}
