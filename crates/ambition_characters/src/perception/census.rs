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
//! ⛔⛔ **A PREMISE OF EVERY HALL MEASUREMENT, NOT A FOOTNOTE: THE CAST MUST
//! NEED PERCEPTION OR THIS RECORDS NOTHING.** The counting site is inside
//! `build_world_view`, and a brain declaring `PerceptionRequirement::None`
//! never builds a view — so `hall_of_characters`, whose cast is authored
//! `stand_still`, produces ZERO views and the census line carries **no `kept=`
//! field at all**. Measured 2026-09-02: a viewport sweep printed "NO CENSUS ROW"
//! for every arm until the cast was re-brained with
//! `AMBITION_ACTOR_BRAIN_PROFILE=ambition::medium_striker`, whose template
//! consumes a view.
//!
//! ⭐ **THE TELL IS AN ABSENT FIELD, NOT A SMALL NUMBER**, and that distinction
//! is the whole safety of this instrument: `kept=0` would have read as "the
//! budget is binding hard" and been published. An instrument reporting NOTHING
//! looks exactly like an instrument reporting a little. ⇒ Before quoting a
//! `kept`, ask what had to RUN for it to be recorded — the gate that made the
//! hall cheap is the gate that makes it unmeasurable.
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
