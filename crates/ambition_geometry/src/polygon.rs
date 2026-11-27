//! Geometry operations on ordered polygon boundaries.

use std::ops::{Add, Div, Mul, Sub};

/// Copy the part of a convex polygon on the nonnegative side of a line into
/// `out`. The vertices and signed distances must be finite. The closing edge
/// connects the last vertex to the first. A point or segment on the line can
/// remain in the output.
pub fn clip_half_plane<V, S>(polygon: &[V], side: impl Fn(V) -> S, out: &mut Vec<V>)
where
    V: Copy + Add<Output = V> + Sub<Output = V> + Mul<S, Output = V>,
    S: Copy + From<i8> + PartialOrd + Sub<Output = S> + Div<Output = S>,
{
    out.clear();
    let zero = S::from(0);
    let Some(&first) = polygon.first() else {
        return;
    };
    let mut a = first;
    let mut sa = side(a);
    for b in polygon[1..].iter().copied().chain(std::iter::once(first)) {
        let sb = side(b);
        if sa >= zero {
            out.push(a);
        }
        if (sa < zero && sb > zero) || (sa > zero && sb < zero) {
            out.push(a + (b - a) * (sa / (sa - sb)));
        }
        a = b;
        sa = sb;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::DVec2;

    #[test]
    fn an_edge_on_the_plane_stays_with_each_vertex_once() {
        let square = [DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y];
        let mut kept = vec![DVec2::NEG_ONE];
        clip_half_plane(&square, |point| -point.x, &mut kept);
        assert_eq!(kept, [DVec2::ZERO, DVec2::Y]);
        clip_half_plane(&square, |point| point.x - 0.5, &mut kept);
        assert_eq!(
            kept,
            [
                DVec2::new(0.5, 0.0),
                DVec2::X,
                DVec2::ONE,
                DVec2::new(0.5, 1.0)
            ]
        );
    }
}
