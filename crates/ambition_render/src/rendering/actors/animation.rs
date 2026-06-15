//! Per-frame sprite animation systems (player, characters, props).
//!
//! Split out of the former 883-line `actors/mod.rs` (2026-06-15).

use super::*;

/// Drive the player sprite's animation state, atlas index, and facing flip.
/// Runs every frame; no-op on color-rectangle fallbacks (no `CharacterAnimator`).
///
/// Query items are split into nested tuples because Bevy 0.18's `Query`
/// tuple impl caps at 15 entries and the picker now reads three more
/// clusters (body_mode, env_contact, abilities) to cover crouch /
/// crawl / slide / ladder / swim.
pub fn animate_player(
    world_time: Res<ambition_sandbox::WorldTime>,
    primary_attack: Query<&ambition_sandbox::player::ActivePlayerAttack, ambition_sandbox::player::PrimaryPlayerOnly>,
    entities: Res<SceneEntities>,
    gravity: Option<Res<ambition_sandbox::physics::GravityField>>,
    mut query: Query<
        (
            (
                &mut Sprite,
                &mut CharacterAnimator,
                &ambition_sandbox::player::BodyKinematics,
                &ambition_sandbox::player::PlayerGroundState,
                &ambition_sandbox::player::PlayerWallState,
                &ambition_sandbox::player::PlayerBlinkState,
                &ambition_sandbox::player::PlayerFlightState,
                &ambition_sandbox::player::PlayerDashState,
                &ambition_sandbox::player::PlayerLedgeState,
                &ambition_sandbox::player::PlayerCombatState,
                &ambition_sandbox::player::PlayerAnimState,
                &ambition_sandbox::player::PlayerBlinkCameraState,
            ),
            (
                &ambition_sandbox::player::PlayerBodyModeState,
                &ambition_sandbox::player::PlayerEnvironmentContact,
                &ambition_sandbox::player::PlayerAbilities,
                &ambition_sandbox::player::PlayerDodgeState,
                &ambition_sandbox::player::PlayerShieldState,
                Option<&ambition_sandbox::time::time_control::ProperTimeScale>,
            ),
        ),
        With<PlayerVisual>,
    >,
) {
    let Ok((
        (
            mut sprite,
            mut animator,
            kinematics,
            ground,
            wall,
            blink,
            flight,
            dash,
            ledge,
            player_combat,
            anim_state,
            blink_cam,
        ),
        (body_mode, env_contact, abilities, dodge, shield, scale),
    )) = query.get_mut(entities.player)
    else {
        return;
    };
    let attack_state = primary_attack.iter().next().and_then(|a| a.0.as_ref());
    let anim = ambition_sandbox::character_sprites::pick_player_anim(
        anim_state,
        player_combat,
        blink_cam,
        attack_state,
        kinematics,
        ground,
        wall,
        blink,
        flight,
        dash,
        ledge,
        body_mode,
        env_contact,
        abilities,
        dodge,
        shield,
    );
    animator.request(anim);
    // ADR 0011 — `entity_dt` collapses to `sim_dt` when no
    // ProperTimeScale is set (SP default), so bullet-time /
    // hitstop / pause still slow the animation in lockstep. Step 4
    // wires the player ProperTimeScale path so future MP regimes
    // can boost the player's cognitive rate without slowing the
    // world for other observers.
    let index = animator.tick(world_time.entity_dt(
        ambition_sandbox::time::time_control::ProperTimeScale::or_default(scale),
    ));
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = index;
    }
    // Gravity-aware facing flip: a ~180° up-gravity roll already mirrors the
    // sprite, so the flip inverts (fixes #33 "move left, face right upside down").
    let player_gravity = gravity
        .as_deref()
        .map_or(ambition_sandbox::engine_core::Vec2::Y, |g| g.dir);
    sprite.flip_x = ambition_sandbox::physics::gravity_aware_flip_x(kinematics.facing, player_gravity);
    // Hit feedback is drawn by the white-silhouette overlay in
    // `presentation::rendering::hit_flash` — a sibling mesh that
    // samples this atlas frame and outputs pure white modulated by
    // `PlayerCombatState::flash_timer`. The source sprite stays at
    // full opacity / untinted; the overlay does the flashing.
    sprite.color = Color::WHITE;
}

/// Drive enemy AND NPC sprite animation, atlas index, and facing flip.
///
/// Enemies and NPCs both render through `CharacterAnimator`; their
/// per-frame state is owned by separate runtime lists, but a feature
/// id only ever appears in one of them at a time. We try the enemy
/// lookup first (most entities in the room) and fall through to the
/// NPC lookup, so a stationary General sheet ticks its 8 idle frames
/// once the animator is attached.
///
/// One system instead of two avoids the borrow conflict on the
/// shared `(&mut Sprite, &mut CharacterAnimator)` query.
pub fn animate_characters(
    world_time: Res<ambition_sandbox::WorldTime>,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &mut CharacterAnimator,
            Option<&ambition_sandbox::time::time_control::ProperTimeScale>,
        ),
        (
            Without<PlayerVisual>,
            Without<ambition_sandbox::rooms::PortalSprite>,
            Without<PropVisual>,
        ),
    >,
    ecs_actors: Query<ambition_sandbox::features::ActorSpriteData>,
    // Localized gravity, so an enemy/NPC wall-walking or on a flipped-gravity
    // ceiling flips the right way (the same gravity-aware facing the player got).
    gravity: ambition_sandbox::physics::GravityCtx,
) {
    // ADR 0011 — per-entity proper time. SP today: no entity carries
    // ProperTimeScale, so `entity_dt` collapses to `sim_dt` and
    // every actor ticks at the world rate. The seam matters once a
    // boss freezes the world but leaves the player un-frozen, or
    // future MP boosts one player's proper time.
    for (visual, mut sprite, mut animator, scale) in &mut query {
        let dt = world_time.entity_dt(ambition_sandbox::time::time_control::ProperTimeScale::or_default(
            scale,
        ));
        let (anim, facing, pos, hit_flash, attacking) = if let Some(state) =
            ambition_sandbox::features::ecs_enemy_anim_state(&visual.id, &ecs_actors)
        {
            (
                ambition_sandbox::character_sprites::pick_enemy_anim(state),
                state.facing,
                state.pos,
                state.hit_flash,
                state.attack_active || state.attack_windup,
            )
        } else if let Some(state) = ambition_sandbox::features::ecs_npc_anim_state(&visual.id, &ecs_actors) {
            (
                ambition_sandbox::character_sprites::pick_npc_anim(state),
                state.facing,
                state.pos,
                state.hit_flash,
                false,
            )
        } else {
            continue;
        };
        animator.request(anim);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        sprite.flip_x = ambition_sandbox::physics::gravity_aware_flip_x(facing, gravity.dir_at(pos));
        // Hit feedback is drawn by the white-silhouette overlay in
        // `presentation::rendering::hit_flash`. Keep the warm
        // attack tint on the multiplicative `sprite.color` channel
        // — it's a separate signal (telegraphing the actor's
        // outgoing swing, not its own damage). The hit_flash
        // boolean still feeds into anim selection upstream so the
        // `hit` row plays.
        sprite.color = if attacking {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
        let _ = hit_flash;
    }
}

/// Prop kinds whose authored "Idle" row depicts motion (e.g. rolling
/// wheels). These props stay pinned at frame 0 in [`animate_props`]
/// until a `PropMotionState` component lands to gate their tick by
/// real motion. Add a kind here when its sprite's idle frame reads
/// as "this prop is moving" — the cart is the v1 case.
pub const PROP_KINDS_STATIC_UNTIL_MOVING: &[&str] = &["intro_cart"];

/// Tick the idle animation row for every `PropVisual` sprite that
/// owns a `CharacterAnimator`. Props have no ECS actor entity, so
/// the regular `animate_characters` lookup would skip them — without
/// this system the sprite stays pinned to frame 0 forever.
///
/// Filtered with `Without<ambition_sandbox::rooms::PortalSprite>` so the gate
/// ring + gate portal stay owned by the portal-presentation systems
/// (which drive the animator from `GatePortalPhase` instead of a flat
/// Idle row tick).
///
/// Motion-gated props: a kind listed in [`PROP_KINDS_STATIC_UNTIL_MOVING`]
/// stays pinned at frame 0. The intro cart's authored "idle" row is a
/// wheel-rolling cycle that reads as "the cart is moving"; without a
/// real motion source today (no scripted push), looping it makes the
/// cart look like it's drifting in place. Until a `PropMotionState`
/// component lands, hold these kinds at rest.
pub fn animate_props(
    world_time: Res<ambition_sandbox::WorldTime>,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &PropVisual,
            Option<&ambition_sandbox::time::time_control::ProperTimeScale>,
        ),
        Without<ambition_sandbox::rooms::PortalSprite>,
    >,
) {
    // ADR 0011 — per-entity proper time. Props that need to keep
    // ticking when the world freezes (a clock prop in a frozen
    // boss arena, say) get a non-1.0 ProperTimeScale.
    for (mut sprite, mut animator, prop, scale) in &mut query {
        if PROP_KINDS_STATIC_UNTIL_MOVING.contains(&prop.kind.as_str()) {
            // Force-rest at frame 0 of the Idle row. `request` selects
            // the row; ticking with dt=0 holds the row's current frame
            // and matches the asset's first frame on entry.
            animator.request(ambition_sandbox::character_sprites::CharacterAnim::Idle);
            let index = animator.tick(0.0);
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = index;
            }
            continue;
        }
        let dt = world_time.entity_dt(ambition_sandbox::time::time_control::ProperTimeScale::or_default(
            scale,
        ));
        animator.request(ambition_sandbox::character_sprites::CharacterAnim::Idle);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
    }
}
