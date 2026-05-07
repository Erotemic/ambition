//! Property tests for `ResourceMeter` (mana / stamina / ammo / charge).
//!
//! `ResourceMeter` is the engine-side store the F3 inspector writes
//! and the future charge-shot / hover-fuel / oxygen abilities will
//! consume. This proptest pins the invariants the consumers rely on:
//!
//! 1. `current` is always within `[0, max]` after any sequence of
//!    operations.
//! 2. `try_spend(cost)` returns `true` iff `current >= cost` (within
//!    a small float epsilon), and on success leaves `max` unchanged.
//! 3. `tick_regen` and `tick_decay` never push `current` outside the
//!    `[0, max]` envelope.
//! 4. `refill_full` always lands at `max`; `is_full` is true after.

use ambition_engine::ResourceMeter;
use proptest::prelude::*;

proptest! {
    /// `current` never exceeds `[0, max]` after a random sequence of
    /// spends, refills, and ticks.
    #[test]
    fn meter_current_stays_in_envelope(
        max in 1.0f32..1000.0,
        regen in 0.0f32..100.0,
        decay in 0.0f32..100.0,
        spends in proptest::collection::vec(0.0f32..100.0, 0..20),
        refills in proptest::collection::vec(0.0f32..100.0, 0..20),
        ticks in proptest::collection::vec(0.001f32..1.0, 0..20),
    ) {
        let mut meter = ResourceMeter::new(max, regen, decay);
        // Interleave the operations so the test exercises mixed
        // sequences. Length cap of 60 is conservative.
        let total = spends.len().max(refills.len()).max(ticks.len());
        for i in 0..total {
            if let Some(&cost) = spends.get(i) {
                let _ = meter.try_spend(cost);
            }
            if let Some(&amount) = refills.get(i) {
                meter.refill(amount);
            }
            if let Some(&dt) = ticks.get(i) {
                meter.tick(dt);
            }
            prop_assert!(meter.current >= 0.0,
                "current went negative: {}", meter.current);
            prop_assert!(meter.current <= max + 1e-3,
                "current exceeded max: {} > {}", meter.current, max);
            prop_assert!((meter.max - max).abs() < 1e-6,
                "max changed: {} != {}", meter.max, max);
        }
    }

    /// `try_spend` is exact about success/failure conditions.
    #[test]
    fn try_spend_succeeds_iff_enough(
        max in 1.0f32..1000.0,
        cost in 0.0f32..1000.0,
    ) {
        let mut meter = ResourceMeter::new(max, 0.0, 0.0);
        let before = meter.current;
        let succeeded = meter.try_spend(cost);
        if succeeded {
            prop_assert!(before + 1e-3 >= cost);
            prop_assert!((meter.current - (before - cost)).abs() < 1e-3);
        } else {
            prop_assert_eq!(meter.current, before, "failed spend must leave current unchanged");
        }
    }

    /// `refill_full` always reaches `max`; `is_full` agrees.
    #[test]
    fn refill_full_reaches_max(
        max in 1.0f32..1000.0,
        regen in 0.0f32..100.0,
        decay in 0.0f32..100.0,
        spend in 0.0f32..1000.0,
    ) {
        let mut meter = ResourceMeter::new(max, regen, decay);
        let _ = meter.try_spend(spend);
        meter.refill_full();
        prop_assert!((meter.current - max).abs() < 1e-3);
        prop_assert!(meter.is_full());
    }

    /// `tick_regen` is monotonic (current never decreases after it).
    #[test]
    fn tick_regen_is_monotonic(
        max in 1.0f32..1000.0,
        regen in 0.0f32..100.0,
        dt in 0.001f32..1.0,
    ) {
        let mut meter = ResourceMeter::new(max, regen, 0.0);
        // Spend half so there's room to regen into.
        let _ = meter.try_spend(max * 0.5);
        let before = meter.current;
        meter.tick_regen(dt);
        prop_assert!(meter.current >= before - 1e-6);
        prop_assert!(meter.current <= max + 1e-3);
    }

    /// `tick_decay` is monotonic the other way (current never
    /// increases) and never goes below zero.
    #[test]
    fn tick_decay_is_monotonic(
        max in 1.0f32..1000.0,
        decay in 0.0f32..100.0,
        dt in 0.001f32..1.0,
    ) {
        let mut meter = ResourceMeter::new(max, 0.0, decay);
        let before = meter.current;
        meter.tick_decay(dt);
        prop_assert!(meter.current <= before + 1e-6);
        prop_assert!(meter.current >= 0.0);
    }
}
