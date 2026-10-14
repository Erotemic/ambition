//! Optional counts for peers offered to and retained by each world view.
//!
//! Enabled only when requested; the disabled path uses a cheap flag
//! check. A brain with no perception requirement builds no world view,
//! so an absent sample is not a zero-peer sample.
//! The developer measurement tool enables and drains these counters.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);
static VIEWS: AtomicU64 = AtomicU64::new(0);
static OFFERED: AtomicU64 = AtomicU64::new(0);
static KEPT: AtomicU64 = AtomicU64::new(0);
static KEPT_MAX: AtomicU64 = AtomicU64::new(0);
/// Peers VISIBLE to the viewer before the attention budget cut — `kept` plus
/// the remainder. Density is read here; the budget flattens `kept`.
static VISIBLE: AtomicU64 = AtomicU64::new(0);

/// Start recording. Called where the census installs its other rows.
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// Record one built world view: how many peers were OFFERED (the room), how
/// many were VISIBLE (inside the viewport, before the attention budget), and
/// how many were KEPT (carried exactly; the number `Decide`'s cost follows).
///
/// ⛔ THE HOT PATH. Runs once per perceiving body per tick, so it does no
/// allocation, takes no lock, and returns on a single relaxed load when off.
pub fn note_world_view(offered: usize, visible: usize, kept: usize) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    VIEWS.fetch_add(1, Ordering::Relaxed);
    OFFERED.fetch_add(offered as u64, Ordering::Relaxed);
    VISIBLE.fetch_add(visible as u64, Ordering::Relaxed);
    KEPT.fetch_add(kept as u64, Ordering::Relaxed);
    KEPT_MAX.fetch_max(kept as u64, Ordering::Relaxed);
}

/// One drained census window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewCensus {
    pub views: u64,
    pub offered_mean: f64,
    pub visible_mean: f64,
    pub kept_mean: f64,
    pub kept_max: u64,
}

/// Means and the worst single viewer since the last drain.
///
/// Returns `None` when no view was built, so a caller reports nothing rather
/// than a row of zeroes.
pub fn drain() -> Option<ViewCensus> {
    let views = VIEWS.swap(0, Ordering::Relaxed);
    let offered = OFFERED.swap(0, Ordering::Relaxed);
    let visible = VISIBLE.swap(0, Ordering::Relaxed);
    let kept = KEPT.swap(0, Ordering::Relaxed);
    let kept_max = KEPT_MAX.swap(0, Ordering::Relaxed);
    if views == 0 {
        return None;
    }
    Some(ViewCensus {
        views,
        offered_mean: offered as f64 / views as f64,
        visible_mean: visible as f64 / views as f64,
        kept_mean: kept as f64 / views as f64,
        kept_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔ THE PREMISE GUARD FOR EVERY RUN TAKEN WITHOUT THE CENSUS. If the
    /// default recorded, the instrument would be in every measurement of the
    /// thing it measures.
    #[test]
    fn it_records_nothing_until_enabled_and_then_reports_what_it_saw() {
        assert!(!ENABLED.load(Ordering::Relaxed), "off by default");
        note_world_view(129, 14, 14);
        assert!(
            drain().is_none(),
            "a disabled census records nothing at all"
        );

        enable();
        note_world_view(129, 10, 10);
        note_world_view(129, 40, 20);
        let census = drain().expect("two views were recorded");
        assert_eq!(census.views, 2);
        assert_eq!(census.offered_mean, 129.0);
        assert_eq!(census.visible_mean, 25.0, "the MEAN visible set, before the budget");
        assert_eq!(census.kept_mean, 15.0, "the MEAN kept set, after it");
        assert_eq!(
            census.kept_max, 20,
            "and the worst single viewer, which is what a budget caps"
        );

        assert!(drain().is_none(), "draining leaves the counters empty");
        ENABLED.store(false, Ordering::Relaxed);
    }
}
