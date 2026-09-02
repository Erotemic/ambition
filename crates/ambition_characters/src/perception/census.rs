//! How many peers each viewer actually KEEPS, per tick.
//!
//! ⭐ THE NUMBER THE ATTENTION BUDGET WILL BE JUDGED AGAINST. `Decide` grows
//! superlinearly with population while each viewer's kept-peer set is still
//! growing, and flattens when it saturates — measured 2026-09-01, 5.9x cost for
//! 4x bodies between 16 and 64, then linear above it. That saturation is a
//! property of the hall's GEOMETRY (a sparse gallery, so `Perception::Sighted`'s
//! viewport stops admitting peers), not a law. A dense melee would keep growing.
//!
//! Until now `kept ≈ 14` was a number from a throwaway probe, quoted long after
//! the probe was gone. This makes it a measurement anyone can re-take, and gives
//! `bounded-perception-and-attention.md`'s acceptance criterion — exact set
//! bounded by K regardless of room population — something to assert against.
//!
//! ⚠ OFF BY DEFAULT AND NEARLY FREE WHEN OFF: one relaxed load per world view.
//! The census is a measuring instrument and must not join the population it
//! measures when nobody asked it to.
//!
//! ⭐ IT MOVED HERE FROM `ambition_dev_tools` ON 2026-09-02, and the direction
//! is the point (D33). The counting site is `build_world_view` in the actor
//! kernel — a HOT LOOP, so the number cannot be recovered from outside — and
//! while the sink lived in the developer crate, the simulation had to name that
//! crate to record it. That was one of the two production reads keeping the
//! kernel's upward `ambition_dev_tools` dependency alive.
//!
//! ⛔ THE DEVELOPER CRATE STILL OWNS THE REPORT. `runtime_census` calls
//! [`enable`] where it installs its other rows and [`drain`] where it prints
//! them; nothing here formats, schedules, or decides when to measure. What
//! moved down is the COUNTER, which is a property of the views this module
//! builds, not the instrument's policy.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);
static VIEWS: AtomicU64 = AtomicU64::new(0);
static OFFERED: AtomicU64 = AtomicU64::new(0);
static KEPT: AtomicU64 = AtomicU64::new(0);
static KEPT_MAX: AtomicU64 = AtomicU64::new(0);

/// Start recording. Called where the census installs its other rows.
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// Record one built world view: how many peers were OFFERED and how many KEPT.
///
/// ⛔ THE HOT PATH. Runs once per perceiving body per tick, so it does no
/// allocation, takes no lock, and returns on a single relaxed load when off.
pub fn note_world_view(offered: usize, kept: usize) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    VIEWS.fetch_add(1, Ordering::Relaxed);
    OFFERED.fetch_add(offered as u64, Ordering::Relaxed);
    KEPT.fetch_add(kept as u64, Ordering::Relaxed);
    KEPT_MAX.fetch_max(kept as u64, Ordering::Relaxed);
}

/// Mean offered, mean kept, and the worst single viewer since the last drain.
///
/// Returns `None` when no view was built, so a caller reports nothing rather
/// than a row of zeroes.
pub fn drain() -> Option<(u64, f64, f64, u64)> {
    let views = VIEWS.swap(0, Ordering::Relaxed);
    let offered = OFFERED.swap(0, Ordering::Relaxed);
    let kept = KEPT.swap(0, Ordering::Relaxed);
    let kept_max = KEPT_MAX.swap(0, Ordering::Relaxed);
    if views == 0 {
        return None;
    }
    Some((
        views,
        offered as f64 / views as f64,
        kept as f64 / views as f64,
        kept_max,
    ))
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
        note_world_view(129, 14);
        assert!(
            drain().is_none(),
            "a disabled census records nothing at all"
        );

        enable();
        note_world_view(129, 10);
        note_world_view(129, 20);
        let (views, offered, kept, kept_max) = drain().expect("two views were recorded");
        assert_eq!(views, 2);
        assert_eq!(offered, 129.0);
        assert_eq!(kept, 15.0, "the MEAN kept set");
        assert_eq!(
            kept_max, 20,
            "and the worst single viewer, which is what a budget caps"
        );

        assert!(drain().is_none(), "draining leaves the counters empty");
        ENABLED.store(false, Ordering::Relaxed);
    }
}
