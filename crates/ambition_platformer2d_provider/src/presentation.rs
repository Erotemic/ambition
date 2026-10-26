//! Project route declarations into active camera, defense and HUD presentation.

use ambition_platformer2d_shared_tangle::gameplay_presentation::{
    ActiveDefensePresentationPolicy, ActiveGameplayPresentationProfiles, ActiveHudDeclaration,
    DefensePresentationCatalog, DefensePresentationPolicy, GameplayPresentationProfileCatalog,
    HudDeclaration, HudDeclarationCatalog,
};
use bevy::prelude::*;

/// Copy the active route's declared profiles into the resource the host
/// consumes.
///
/// This is the ONLY place routes and presentation meet. It lives in the
/// provider crate because that is the layer that already knows about the shell
/// router; `ambition_platformer2d_host` deliberately cannot see routes at all, and so cannot
/// grow a game-name branch even by accident.
///
/// A route with no declaration — the launcher, a menu, a provider that opted
/// out — resolves to the engine default, which is today's full-bleed normal
/// framing. That is what keeps full-screen menus and startup presentation
/// usable across the whole display.
pub fn select_active_presentation_profiles(
    router: Res<ambition_game_shell::ShellRouter>,
    catalog: Option<Res<GameplayPresentationProfileCatalog>>,
    mut active: ResMut<ActiveGameplayPresentationProfiles>,
) {
    let declared = router
        .active
        .as_ref()
        .zip(catalog.as_deref())
        .and_then(|(active, catalog)| catalog.get(active.route_id.as_str()))
        .copied()
        .unwrap_or_default();
    if active.0 != declared {
        active.0 = declared;
    }
}

/// Publish the active route's CAMERA FEEL into the resources the camera reads.
///
/// Exactly parallel to [`select_active_presentation_profiles`], and deliberately
/// a separate system rather than a few more lines inside it: the camera tuning
/// resources live in `camera_ease` and are read by the snapshot resolve and the
/// shake tick, not by the presentation resolve, so folding them together would
/// couple two schedules that have no other reason to meet.
///
/// A route with no declaration resolves to the profile default, which is exactly
/// the historical constants — so the launcher, menus and any provider that opted
/// out ease and shake as they always did.
pub fn publish_active_camera_feel(
    active: Res<ActiveGameplayPresentationProfiles>,
    mut ease: ResMut<ambition_platformer2d_shared_tangle::camera_ease::CameraEaseTuning>,
    mut shake: ResMut<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeTuning>,
) {
    // Read the DEFAULT profile's feel rather than the environment-selected one.
    let feel = active.0.default.camera_feel;
    if *ease != feel.ease {
        *ease = feel.ease;
    }
    if *shake != feel.shake {
        *shake = feel.shake;
    }
}

/// Copy the active route's defense presentation declaration into the resource
/// the renderer consumes.
///
/// The provider owns the route lookup; the renderer sees only the selected
/// policy and therefore cannot grow game-name branches. A route that declares
/// nothing gets no shared defense effect — games opt into those cues explicitly.
pub fn select_active_defense_presentation(
    router: Res<ambition_game_shell::ShellRouter>,
    catalog: Option<Res<DefensePresentationCatalog>>,
    mut active: ResMut<ActiveDefensePresentationPolicy>,
) {
    let declared = router
        .active
        .as_ref()
        .zip(catalog.as_deref())
        .and_then(|(active, catalog)| catalog.get(active.route_id.as_str()))
        .copied()
        .unwrap_or_else(DefensePresentationPolicy::none);
    if active.0 != declared {
        active.0 = declared;
    }
}

/// Copy the active route's declared HUD into the resource the renderer
/// consumes.
///
/// Exactly parallel to [`select_active_presentation_profiles`], and here for
/// the same reason: this is the only place routes and HUD declarations meet,
/// and `ambition_render` cannot see routes at all, so it cannot grow a
/// game-name branch even by accident.
///
/// A route with no declaration — the launcher, a menu, a game that wants no
/// HUD — resolves to `None`, and the renderer draws no HUD surface.
pub fn select_active_hud_declaration(
    router: Res<ambition_game_shell::ShellRouter>,
    catalog: Option<Res<HudDeclarationCatalog>>,
    mut active: ResMut<ActiveHudDeclaration>,
) {
    let declared = router
        .active
        .as_ref()
        .zip(catalog.as_deref())
        .and_then(|(active, catalog)| catalog.get(active.route_id.as_str()))
        .cloned();
    update_active_hud_declaration(&mut active, declared);
}

fn update_active_hud_declaration(
    active: &mut ActiveHudDeclaration,
    declared: Option<HudDeclaration>,
) -> bool {
    // Equal slot COUNTS do not imply equal declarations. Two routes can each
    // have one slot while disagreeing on id, style, region, or centering; the
    // old length-only check left the previous route's HUD active indefinitely.
    if active.0 == declared {
        return false;
    }
    active.0 = declared;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::gameplay_presentation::HudSlotSpec;

    #[test]
    fn defense_presentation_follows_the_active_route_in_a_multi_game_host() {
        use ambition_game_shell::{
            ActiveShellExperience, ShellActivationId, ShellExperienceId, ShellRouteId, ShellRouter,
        };

        use ambition_platformer2d_shared_tangle::gameplay_presentation::DefenseCueCauses;

        let shared = DefensePresentationPolicy::shared_iframe_blink();
        let empowerment_blinks = shared.with_blink(DefenseCueCauses::EMPOWERED);

        let mut catalog = DefensePresentationCatalog::default();
        catalog.insert("shared", shared);
        catalog.insert("empowerment-blinks", empowerment_blinks);

        let active = |route: &str| ActiveShellExperience {
            activation_id: ShellActivationId(1),
            route_id: ShellRouteId::new(route),
            experience_id: ShellExperienceId::new(route),
            parameters: Default::default(),
            load_authorization: None,
            prepared_session: None,
        };

        let mut app = bevy::app::App::new();
        // ⛔ NOT `ShellRouter { active, ..Default::default() }`. Functional
        // update needs every field VISIBLE even where it names none of them,
        // and the router keeps its activation counters private — so that form
        // stopped compiling the day they became private, taking the whole
        // `--workspace --lib` gate with it.
        let mut router = ShellRouter::default();
        router.active = Some(active("shared"));
        app.insert_resource(router)
            .insert_resource(catalog)
            .init_resource::<ActiveDefensePresentationPolicy>()
            .add_systems(bevy::app::Update, select_active_defense_presentation);

        app.update();
        assert_eq!(
            app.world().resource::<ActiveDefensePresentationPolicy>().0,
            shared
        );

        app.world_mut().resource_mut::<ShellRouter>().active = Some(active("empowerment-blinks"));
        app.update();
        assert_eq!(
            app.world().resource::<ActiveDefensePresentationPolicy>().0,
            empowerment_blinks,
            "the previous game's shared-effect policy leaked across the route switch"
        );

        app.world_mut().resource_mut::<ShellRouter>().active = Some(active("undeclared"));
        app.update();
        assert_eq!(
            app.world().resource::<ActiveDefensePresentationPolicy>().0,
            DefensePresentationPolicy::none(),
            "an undeclared route inherited the last gameplay route's defense effects"
        );
    }

    #[test]
    fn equal_sized_route_huds_still_replace_each_other() {
        let old = HudDeclaration::new().slot(HudSlotSpec::new("rings"));
        let next =
            HudDeclaration::new().slot(HudSlotSpec::new("score").centered().with_font_size(30.0));
        let mut active = ActiveHudDeclaration(Some(old));

        assert!(update_active_hud_declaration(
            &mut active,
            Some(next.clone()),
        ));
        assert_eq!(active.0, Some(next));
    }

    /// : two games in one host can ease and shake differently.
    ///
    /// The zoom rates and the snap epsilon were one global resource and the
    /// shake ceiling was a `const` inside `kick`, so a multi-game host had one
    /// camera feel for every game in it — a one-game-shaped limit sitting in the
    /// middle of the thing that exists to host several.
    ///
    /// Driven through the real system against the real resources, because the
    /// claim is not "the struct has a field" — it is that SWITCHING ROUTES
    /// changes what the camera does.
    #[test]
    fn each_route_publishes_its_own_camera_feel() {
        use ambition_platformer2d_shared_tangle::camera_ease::{
            CameraEaseTuning, CameraShakeTuning,
        };
        use ambition_platformer2d_shared_tangle::gameplay_presentation::{
            CameraFeelPolicy, GameplayPresentationProfile, GameplayPresentationProfiles,
        };

        let snappy = CameraFeelPolicy {
            ease: CameraEaseTuning {
                zoom_out_rate: 9.0,
                zoom_in_rate: 8.0,
                snap_epsilon: 0.5,
            },
            shake: CameraShakeTuning {
                max_amplitude_px: 40.0,
                decay_px_per_s: 200.0,
            },
        };

        let mut app = bevy::app::App::new();
        app.init_resource::<CameraEaseTuning>()
            .init_resource::<CameraShakeTuning>()
            .insert_resource(ActiveGameplayPresentationProfiles(
                GameplayPresentationProfiles::uniform(
                    GameplayPresentationProfile::full_bleed().with_camera_feel(snappy),
                ),
            ))
            .add_systems(bevy::app::Update, publish_active_camera_feel);
        app.update();

        assert_eq!(*app.world().resource::<CameraEaseTuning>(), snappy.ease);
        assert_eq!(*app.world().resource::<CameraShakeTuning>(), snappy.shake);

        // Switch to a route that declares nothing — a menu, the launcher, a
        // provider that opted out. It must land back on the historical defaults
        // rather than inherit the last game's feel, or leaving Sanic would leave
        // his camera behind in Mary-O.
        *app.world_mut()
            .resource_mut::<ActiveGameplayPresentationProfiles>() =
            ActiveGameplayPresentationProfiles::default();
        app.update();

        assert_eq!(
            *app.world().resource::<CameraEaseTuning>(),
            CameraEaseTuning::default(),
            "an undeclared route inherited the previous game's ease"
        );
        assert_eq!(
            *app.world().resource::<CameraShakeTuning>(),
            CameraShakeTuning::default(),
        );
    }

    /// The ceiling is the ROUTE's now, and a kick obeys it.
    #[test]
    fn a_kick_is_clamped_by_the_routes_ceiling_not_a_constant() {
        use ambition_platformer2d_shared_tangle::camera_ease::{
            CameraShakeState, CameraShakeTuning,
        };

        let mut shake = CameraShakeState::default();
        shake.kick(
            1000.0,
            CameraShakeTuning {
                max_amplitude_px: 3.0,
                ..CameraShakeTuning::default()
            },
        );
        assert_eq!(
            shake.amplitude_px, 3.0,
            "the kick used the old hardcoded 14px cap instead of the route's"
        );
    }

    #[test]
    fn identical_route_hud_is_left_unchanged() {
        let declaration = HudDeclaration::new().slot(HudSlotSpec::new("rings"));
        let mut active = ActiveHudDeclaration(Some(declaration.clone()));
        assert!(!update_active_hud_declaration(
            &mut active,
            Some(declaration.clone()),
        ));
        assert_eq!(active.0, Some(declaration));
    }
}
