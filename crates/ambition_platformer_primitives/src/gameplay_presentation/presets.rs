//! Tested author presets.
//!
//! A game selects one of these and moves on. The builder stays available for a
//! genuinely custom profile, but a preset is the expected answer — the engine
//! owes authors a working default, not a policy construction exercise.
//!
//! **These are not game-name branches.** Each preset is a plain configuration
//! of the four axes; the "intended initial consumer" notes record who asked for
//! it, not who is allowed to use it.

pub mod profiles {
    use super::super::{
        FixedAspectFit, GameplayPresentationProfile, GameplayPresentationProfiles,
        SoftFramingProfile,
    };

    /// Normal camera on desktop; full-bleed occlusion-aware soft framing when
    /// touch is primary.
    ///
    /// The desktop half is deliberately *unchanged behavior*: a game that wants
    /// mobile framing should not pay for it with a different desktop camera.
    ///
    /// Intended initial consumer: the Ambition flagship.
    pub fn adaptive_platformer() -> GameplayPresentationProfiles {
        GameplayPresentationProfiles {
            default: GameplayPresentationProfile::full_bleed(),
            touch_primary: Some(
                GameplayPresentationProfile::full_bleed()
                    .with_occlusion_aware_framing(SoftFramingProfile::platformer()),
            ),
            handheld: None,
        }
    }

    /// Full-bleed velocity-aware soft framing everywhere, with the safe region
    /// additionally reduced by control occupancy when touch is primary.
    ///
    /// Intended initial consumer: Sanic.
    pub fn high_speed_full_bleed() -> GameplayPresentationProfiles {
        GameplayPresentationProfiles {
            default: GameplayPresentationProfile::full_bleed()
                .with_soft_framing(SoftFramingProfile::high_speed()),
            touch_primary: Some(
                GameplayPresentationProfile::full_bleed()
                    .with_occlusion_aware_framing(SoftFramingProfile::high_speed()),
            ),
            handheld: None,
        }
    }

    /// A fixed 4:3 gameplay viewport on every platform; the surround is
    /// available to HUD and controls.
    ///
    /// Touch-primary pins the rectangle to the TOP of the safe display rather
    /// than centering it, so the vertical slack collects under the gameplay
    /// area — which is exactly where thumbs are. On a display that is not
    /// taller than 4:3 there is no vertical slack and the two agree.
    ///
    /// Intended initial consumer: Super Mary O.
    pub fn fixed_four_by_three() -> GameplayPresentationProfiles {
        let base = GameplayPresentationProfile::fixed_aspect(4.0, 3.0).with_reserved_surround();
        GameplayPresentationProfiles {
            default: base,
            touch_primary: Some(base.with_fit(FixedAspectFit::Top)),
            handheld: None,
        }
    }
}
