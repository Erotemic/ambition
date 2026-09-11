//! ONE CONTEST FOR EVERY ATTACK FAMILY — swings and shots arbitrated together,
//! before either road resolves damage.
//!
//! ⭐⭐ THE MISSING HALF OF CLANKING, and S0.1 names it: `arbitrate_attack_clanks`
//! queried strike volumes only, so a bolt flew through an opposing swing and
//! through another bolt without either noticing. That was not one missing query.
//! `step_projectiles` resolves its own contact in `CombatSet::Materialize` and
//! the melee arbitration ran later in the same tick, so a shot had already hit
//! (or missed) before any contest existed — and its victim query is
//! `Without<LiveProjectile>` besides, so shots could not see each other by
//! construction.
//!
//! ⇒ The contest moved to the one place that is BEFORE both damage roads, and
//! both families feed it.
//!
//! ## Why this system is here and the DECISION is in `ambition_combat`
//!
//! [`ambition_combat::clank::resolve_clashes`] holds the rule — ordering,
//! pairing, opposition, overlap, verdict — and has no world in it. This file
//! holds only the two things that need a world: reading each family's boxes out
//! of its own components, and mapping a [`ClashAttack`] back onto that family's
//! way of ending an attack (a move is cancelled; a shot is SPENT, and expires
//! through the stepper's own expiry road on the very next system).
//!
//! ⛔ TWO ARBITRATION SITES WOULD HAVE BEEN TWO ANSWERS to "which attack wins",
//! which is the authority split this packet exists to close. `ambition_combat`
//! cannot host the collection because `LiveProjectile` / `ProjectileSeq` /
//! `ProjectileAllegiance` live outside it, and giving the content-free combat
//! model a dependency on the projectile model to reach three components would
//! push `ambition_input` and the trace schema into every crate that wanted
//! `Damage`. This crate already owns `apply_hitbox_damage` and
//! `step_projectiles` for the same reason.

use bevy::prelude::*;

use ambition_combat::clank::{
    resolve_clashes, AttacksClanked, ClashAttack, ClashContender, ClashFamily, ClashOrder,
};
use ambition_platformer2d_core as ae;

/// Arbitrate every opposed, overlapping pair of live attacks, then end the
/// losers — before `step_projectiles` or `apply_hitbox_damage` asks any of them
/// about a victim.
///
/// ⭐ THE ANSWER MUST BE KNOWN FOR BOTH ATTACKS BEFORE EITHER RESOLVES. A sweep
/// that arbitrated as it went would let whichever attack its query yielded first
/// land before the trade was known.
#[allow(clippy::too_many_arguments)]
pub fn arbitrate_attack_clashes(
    mut commands: Commands,
    // ⛔⛔ `StrikeVolume`, NOT `HitboxLifetime`. `advance_move_playback` spawns
    // authored volumes with a comment reading *"NO `HitboxLifetime` on purpose"*
    // — the authored Active window is their despawn authority — so a filter on
    // the lifetime component makes every Smash jab, tilt, smash and aerial
    // invisible here.
    strikes: Query<(
        &ambition_combat::strike::Hitbox,
        &ambition_combat::moveset::StrikeVolume,
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    )>,
    mut shots: Query<
        (
            Entity,
            &ae::BodyKinematics,
            &mut ambition_platformer2d_shared_tangle::projectile::ProjectileGameplay,
            &ambition_projectiles::ProjectileSeq,
            Option<&ambition_projectiles::ProjectileOwner>,
            Option<&crate::projectile::ProjectileAllegiance>,
        ),
        With<ambition_projectiles::LiveProjectile>,
    >,
    owner_pos: Query<&ae::BodyKinematics>,
    factions: Query<&ambition_combat::components::ActorFaction>,
    teams: Query<&ambition_combat::targeting::MatchTeam>,
    mut playing: Query<&mut ambition_combat::moveset::MovePlayback>,
    tuning: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
    mut clanked: MessageWriter<AttacksClanked>,
) {
    let rules = tuning.as_deref().copied().unwrap_or_default();
    if rules.clank_damage_window <= 0.0 {
        return;
    }

    let mut contenders: Vec<ClashContender<'_>> = Vec::new();

    for (hitbox, volume, sim_id) in strikes.iter() {
        let Ok(kin) = owner_pos.get(volume.owner) else {
            continue;
        };
        // ⛔⛔ THE STANCE IS THE SWING'S, NOT THE FEET'S. Asking
        // `BodyGroundState` at collision time made a ground attack stop clanking
        // the moment its owner walked off a ledge mid-swing, and made an aerial
        // start clanking when its owner landed. "Grounded attack" is a
        // CLASSIFICATION settled when the swing comes out, and
        // `MovePlayback::started_grounded` is the same stance the SELECTOR read
        // to choose this variant.
        let grounded = playing
            .get(volume.owner)
            .map(|playback| playback.started_grounded)
            .unwrap_or(true);
        contenders.push(ClashContender {
            order: ClashOrder::Melee(sim_id.map(|id| id.as_str()).unwrap_or("")),
            attack: ClashAttack::Move(volume.owner),
            owner: volume.owner,
            volume: hitbox.world_volume(kin.pos),
            family: if grounded {
                ClashFamily::GroundedMelee
            } else {
                ClashFamily::AerialMelee
            },
            damage: hitbox.damage,
            faction: factions
                .get(volume.owner)
                .copied()
                .unwrap_or_default(),
            team: teams.get(volume.owner).ok(),
        });
    }

    for (entity, kin, game, seq, owner, allegiance) in shots.iter() {
        // ⛔ THE SIDE IS THE ONE STAMPED AT LAUNCH. A shot outlives its firer, so
        // chasing the owner's components answers about a body that may already
        // have been taken out of play — the same reason `step_projectiles` reads
        // `ProjectileAllegiance` rather than the owner.
        let (faction, team) = match allegiance {
            Some(side) => (side.faction, side.team()),
            None => continue,
        };
        contenders.push(ClashContender {
            order: ClashOrder::Shot(seq.0),
            attack: ClashAttack::Shot(entity),
            // An ownerless environmental volley still fights; it just has no
            // body to pair against, and `Entity::PLACEHOLDER` cannot equal a
            // real owner, so it never suppresses a pair by accident.
            owner: owner.map(|o| o.0).unwrap_or(Entity::PLACEHOLDER),
            volume: ae::CombatVolume::aabb(ae::Aabb::new(kin.pos, kin.size * 0.5)),
            family: ClashFamily::Shot,
            damage: game.damage,
            faction,
            team,
        });
    }

    let resolution = resolve_clashes(&mut contenders, rules.clank_damage_window, rules);

    for attack in resolution.defeated {
        match attack {
            // ⛔ THE MOVE ENDS, NOT ITS RECTANGLE. Despawning one volume left the
            // losing move playing, so its sibling volumes and every later window
            // carried on — a rectangle losing a contest the mechanic describes
            // as an attack losing.
            ClashAttack::Move(owner) => {
                if let Ok(mut playback) = playing.get_mut(owner) {
                    ambition_combat::moveset::cancel_move_playback(
                        &mut commands,
                        owner,
                        &mut playback,
                        ambition_combat::moveset::MoveEnd::Interrupted,
                    );
                }
            }
            // ⭐⭐ THE SHOT IS SPENT, NOT DESPAWNED, and that is the whole
            // reason this system runs BEFORE `step_projectiles` rather than
            // beside it. A shot's destruction already has an owner: the expiry
            // branch of the stepper, which writes the detonation FX from the
            // content-owned visual catalog, pushes the `Expired` trace event and
            // then despawns. Reaching for `commands.despawn()` here would be a
            // second answer to "what a destroyed bolt looks like" — the catalog
            // lookup copied, the trace event lost, and the two free to drift.
            //
            // ⇒ Ageing the shot out makes `ProjectileGameplay::tick` return
            // false on the very next system, so the bolt expires through its own
            // road, this tick, unchanged. It also does not MOVE first, which is
            // what losing a contest should look like.
            ClashAttack::Shot(entity) => {
                if let Ok((_, _, mut game, _, _, _)) = shots.get_mut(entity) {
                    game.age = game.max_lifetime;
                }
            }
        }
    }

    for owners in resolution.clanked {
        clanked.write(AttacksClanked { owners });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    /// ⛔⛔ THREE EQUAL ATTACKS MEETING ON ONE TICK: ALL THREE TRADE.
    ///
    /// The sweep skipped a pair when either owner was already in `ended`, so an
    /// EARLIER pair's outcome decided whether a LATER pair was CONSIDERED at
    /// all. With A/B/C overlapping and equal: A/B resolves first by `SimId`,
    /// both end, A/C and B/C are skipped — and **C survives because of `SimId`
    /// order**. Deterministic, and not simultaneous.
    ///
    /// ⭐ `resolved` IS THE DEDUP; `ended` IS A COMMIT LEDGER, applied after the
    /// sweep, exactly as its own comment says. Reading it as an eligibility gate
    /// was the bug.
    ///
    /// ⛔ NO TEST EXERCISED `arbitrate_attack_clashes` AT ALL before this — the
    /// only clank arm covered the pure `clank_verdict` — which is why a one-line
    /// change to shipped arbitration was written and reverted unverified on
    /// 2026-08-25 rather than shipped blind.
    /// One equal swing, long enough to still be playing after the sweep.
    fn a_swing(team: &str) -> ambition_entity_catalog::MoveSpec {
        ambition_entity_catalog::MoveSpec {
            display_name: None,
            id: format!("swing_{team}"),
            clip: ambition_entity_catalog::ClipBinding {
                clip: "attack".to_string(),
                fallbacks: vec![],
            },
            duration_s: 0.4,
            windows: vec![],
            events: vec![],
            gates: Default::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            smash_charge: None,
            charge_gesture: ambition_entity_catalog::ChargeGesture::Smash,
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
            equips: None,
            flow: None,
        }
    }

    #[test]
    fn three_attacks_meeting_at_once_all_trade() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<AttacksClanked>();
        app.insert_resource(ambition_combat::rules::ResolvedCombatTuning {
            clank_damage_window: 9.0,
            ..Default::default()
        });
        app.add_systems(Update, arbitrate_attack_clashes);

        // Three fighters standing on one spot, each mid-move, each on a
        // different side so every pair is opposed.
        let mut fighter = |team: &str| -> Entity {
            app.world_mut()
                .spawn((
                    ae::BodyKinematics {
                        pos: ae::Vec2::ZERO,
                        vel: ae::Vec2::ZERO,
                        size: ae::Vec2::new(16.0, 32.0),
                        facing: 1.0,
                    },
                    ae::BodyGroundState {
                        on_ground: true,
                        ..Default::default()
                    },
                    ambition_combat::targeting::MatchTeam::new(team.to_string()),
                    ambition_combat::moveset::MovePlayback::new(a_swing(team), 1.0),
                ))
                .id()
        };
        let a = fighter("a");
        let b = fighter("b");
        let c = fighter("c");

        // One equal strike volume each, all overlapping at the origin. The ids
        // are stated so the sweep's order is the fixture's, not the allocator's.
        for (owner, id) in [(a, "vol_a"), (b, "vol_b"), (c, "vol_c")] {
            app.world_mut().spawn((
                ambition_combat::strike::Hitbox {
                    strike_sfx: None,
                    owner,
                    source: ambition_combat::strike::HitSide::Enemy,
                    anchor: ambition_combat::strike::HitboxAnchor::FollowOwner {
                        local_offset: ae::Vec2::ZERO,
                    },
                    half_extent: ae::Vec2::new(20.0, 20.0),
                    shape: None,
                    facing: 1.0,
                    damage: 10,
                    knockback: ambition_combat::strike::HitboxKnockback::FeelScale(0.0),
                    launch_dir: None,
                    frame_down: ae::Vec2::new(0.0, 1.0),
                    reaction: None,
                },
                ambition_combat::moveset::StrikeVolume { owner, window: 0 },
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id),
            ));
        }

        app.update();

        let still_swinging: Vec<Entity> = [a, b, c]
            .into_iter()
            .filter(|e| {
                app.world()
                    .get::<ambition_combat::moveset::MovePlayback>(*e)
                    .is_some()
            })
            .collect();
        assert!(
            still_swinging.is_empty(),
            "{} of three equal attacks survived a simultaneous clank — an \
             earlier pair's outcome is deciding whether a later pair is \
             CONSIDERED, so the last id standing wins by allocator-independent \
             luck rather than by the contest",
            still_swinging.len()
        );
    }

    /// ⛔⛔ A GROUND SWING STAYS A GROUND SWING WHEN ITS OWNER LEAVES THE FLOOR.
    ///
    /// Eligibility asked `BodyGroundState::on_ground` AT COLLISION TIME, so a
    /// ground attack stopped clanking the moment its owner walked off a ledge
    /// mid-swing — and an aerial started clanking when its owner landed.
    /// "Grounded attack" is a CLASSIFICATION and it is settled when the swing
    /// comes out — `MovePlayback::started_grounded` is the stance the SELECTOR
    /// used, and the loser's playback is already in this system's hand.
    ///
    /// ⭐ THE ARMS STRADDLE THE LATCH with the FEET HELD WRONG in both: the
    /// clanking pair is airborne right now, and the refused pair is standing.
    #[test]
    fn the_clank_reads_the_swings_stance_not_the_owners_feet() {
        use bevy::prelude::*;

        let traded = |latched_grounded: bool, feet_on_floor: bool| -> bool {
            let mut app = App::new();
            app.add_message::<AttacksClanked>();
            app.insert_resource(ambition_combat::rules::ResolvedCombatTuning {
                clank_damage_window: 9.0,
                ..Default::default()
            });
            app.add_systems(Update, arbitrate_attack_clashes);

            let mut fighter = |team: &str| -> Entity {
                app.world_mut()
                    .spawn((
                        ae::BodyKinematics {
                            pos: ae::Vec2::ZERO,
                            vel: ae::Vec2::ZERO,
                            size: ae::Vec2::new(16.0, 32.0),
                            facing: 1.0,
                        },
                        ae::BodyGroundState {
                            on_ground: feet_on_floor,
                            ..Default::default()
                        },
                        ambition_combat::targeting::MatchTeam::new(team.to_string()),
                        ambition_combat::moveset::MovePlayback::new(a_swing(team), 1.0)
                            .started_in_stance(latched_grounded),
                    ))
                    .id()
            };
            let a = fighter("a");
            let b = fighter("b");
            for (owner, id) in [(a, "vol_a"), (b, "vol_b")] {
                app.world_mut().spawn((
                    ambition_combat::strike::Hitbox {
                        strike_sfx: None,
                        owner,
                        source: ambition_combat::strike::HitSide::Enemy,
                        anchor: ambition_combat::strike::HitboxAnchor::FollowOwner {
                            local_offset: ae::Vec2::ZERO,
                        },
                        half_extent: ae::Vec2::new(20.0, 20.0),
                        shape: None,
                        facing: 1.0,
                        damage: 10,
                        knockback: ambition_combat::strike::HitboxKnockback::FeelScale(0.0),
                        launch_dir: None,
                        frame_down: ae::Vec2::new(0.0, 1.0),
                        reaction: None,
                    },
                    ambition_combat::moveset::StrikeVolume { owner, window: 0 },
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id),
                ));
            }
            app.update();
            app.world().get::<ambition_combat::moveset::MovePlayback>(a).is_none()
        };

        assert!(
            traded(true, false),
            "two GROUND swings did not trade because their owners had left the \
             floor since — walking off a ledge mid-swing turned the attack into \
             something else"
        );
        assert!(
            !traded(false, true),
            "two AERIALS traded because their owners had landed since — the air \
             stopped being a place where committing costs you"
        );
    }

    // ── S0.1: THE FAMILIES THAT COULD NOT MEET ────────────────────────────

    /// One shot, built through the SAME construction path production uses.
    ///
    /// ⭐ `build_in_flight_projectile`, not a hand-built bundle: a second
    /// spawner would be a second authority on what a projectile entity IS, and
    /// the two drift the first time either gains a component.
    fn a_shot(app: &mut App, pos: ae::Vec2, damage: i32, owner: Entity, team: &str) -> Entity {
        a_shot_on_team(app, pos, damage, owner, Some(team))
    }

    /// `None` for the team is a shot fired OUTSIDE a match, where the faction
    /// rule decides — which is the only configuration in which two attacks from
    /// one owner can be opposed at all.
    fn a_shot_on_team(
        app: &mut App,
        pos: ae::Vec2,
        damage: i32,
        owner: Entity,
        team: Option<&str>,
    ) -> Entity {
        let projectile =
            ambition_projectiles::build_in_flight_projectile(ambition_projectiles::ProjectileSpawn {
                origin: pos,
                dir: ae::Vec2::new(1.0, 0.0),
                speed: 200.0,
                damage,
                max_lifetime: 2.0,
                half_extent: ae::Vec2::new(8.0, 8.0),
                gravity: 0.0,
                visual_id: String::new(),
                bounces: 0,
                bounce_on_world_contact: false,
                splash_half_extent: 0.0,
                boomerang_return_s: None,
            });
        let seq = {
            let mut counter = app
                .world_mut()
                .get_resource_or_insert_with(ambition_projectiles::ProjectileSeqCounter::default);
            counter.next()
        };
        app.world_mut()
            .spawn((
                projectile.body.kin,
                projectile.body.game,
                seq,
                ambition_projectiles::ProjectileOwner(owner),
                ambition_projectiles::LiveProjectile,
                crate::projectile::ProjectileAllegiance {
                    faction: ambition_combat::components::ActorFaction::Player,
                    team: team
                        .map(|t| ambition_combat::targeting::MatchTeam::new(t.to_string())),
                },
            ))
            .id()
    }

    fn a_swinging_fighter(app: &mut App, pos: ae::Vec2, team: &str, grounded: bool) -> Entity {
        app.world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos,
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(16.0, 32.0),
                    facing: 1.0,
                },
                ae::BodyGroundState { on_ground: grounded, ..Default::default() },
                ambition_combat::targeting::MatchTeam::new(team.to_string()),
                ambition_combat::moveset::MovePlayback::new(a_swing(team), 1.0)
                    .started_in_stance(grounded),
            ))
            .id()
    }

    fn a_strike_volume(app: &mut App, owner: Entity, id: &str, damage: i32) {
        app.world_mut().spawn((
            ambition_combat::strike::Hitbox {
                strike_sfx: None,
                owner,
                source: ambition_combat::strike::HitSide::Enemy,
                anchor: ambition_combat::strike::HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::ZERO,
                },
                half_extent: ae::Vec2::new(20.0, 20.0),
                shape: None,
                facing: 1.0,
                damage,
                knockback: ambition_combat::strike::HitboxKnockback::FeelScale(0.0),
                launch_dir: None,
                frame_down: ae::Vec2::new(0.0, 1.0),
                reaction: None,
            },
            ambition_combat::moveset::StrikeVolume { owner, window: 0 },
            SimId::placement(id),
        ));
    }

    fn an_arena(window: f32) -> App {
        arena_with_friendly_fire(window, false)
    }

    fn arena_with_friendly_fire(window: f32, friendly_fire: bool) -> App {
        let mut app = App::new();
        app.add_message::<AttacksClanked>();
        app.insert_resource(ambition_combat::rules::ResolvedCombatTuning {
            clank_damage_window: window,
            friendly_fire,
            ..Default::default()
        });
        app.add_systems(Update, arbitrate_attack_clashes);
        app
    }

    /// Did this shot lose? A beaten shot is SPENT, not despawned — see the
    /// outcome arm's own note.
    fn spent(app: &App, shot: Entity) -> bool {
        let game = app
            .world()
            .get::<ambition_platformer2d_shared_tangle::projectile::ProjectileGameplay>(shot)
            .expect("the shot is still an entity — a clash must not despawn it");
        game.age >= game.max_lifetime
    }

    /// ⛔⛔ S0.1's PREMISE, AND IT WAS TRUE BY CONSTRUCTION. `step_projectiles`
    /// takes its victims `Without<LiveProjectile>`, so two opposing bolts passed
    /// through each other whatever the ruleset said about clanking.
    #[test]
    fn two_opposed_shots_of_equal_damage_destroy_each_other() {
        let mut app = an_arena(9.0);
        let a_owner = app.world_mut().spawn_empty().id();
        let b_owner = app.world_mut().spawn_empty().id();
        let a = a_shot(&mut app, ae::Vec2::ZERO, 6, a_owner, "a");
        let b = a_shot(&mut app, ae::Vec2::ZERO, 6, b_owner, "b");

        app.update();

        assert!(spent(&app, a) && spent(&app, b), "two opposed bolts met and both flew on");
        let clanks = app.world().resource::<Messages<AttacksClanked>>().iter_current_update_messages().count();
        assert_eq!(clanks, 1, "a shot trade announced {clanks} clanks; it is one event");
    }

    /// ⭐ THE CONTROL FOR THE ARM ABOVE. If the fixture's two bolts simply
    /// always die, the test above certifies nothing — so the same geometry with
    /// clanking UNDECLARED must leave both flying. Every shipped Ambition
    /// ruleset declares `clank_damage_window: 0.0`, so this is also the arm that
    /// proves this system changes nothing in a room that never asked for it.
    #[test]
    fn an_undeclared_world_lets_both_shots_through() {
        let mut app = an_arena(0.0);
        let a_owner = app.world_mut().spawn_empty().id();
        let b_owner = app.world_mut().spawn_empty().id();
        let a = a_shot(&mut app, ae::Vec2::ZERO, 6, a_owner, "a");
        let b = a_shot(&mut app, ae::Vec2::ZERO, 6, b_owner, "b");

        app.update();

        assert!(!spent(&app, a) && !spent(&app, b), "clanking ran in a world that never declared it");
    }

    /// ⭐ AND TWO BOLTS THAT DO NOT TOUCH DO NOT TRADE — the other half of
    /// "the fixture's bolts do not simply always die".
    #[test]
    fn two_shots_that_do_not_overlap_both_fly_on() {
        let mut app = an_arena(9.0);
        let a_owner = app.world_mut().spawn_empty().id();
        let b_owner = app.world_mut().spawn_empty().id();
        let a = a_shot(&mut app, ae::Vec2::ZERO, 6, a_owner, "a");
        let b = a_shot(&mut app, ae::Vec2::new(400.0, 0.0), 6, b_owner, "b");

        app.update();

        assert!(!spent(&app, a) && !spent(&app, b), "bolts 400 px apart traded");
    }

    /// ⛔ ALLIED FIRE PASSES THROUGH, exactly as an allied hit does.
    #[test]
    fn two_shots_on_the_same_team_pass_through_each_other() {
        let mut app = an_arena(9.0);
        let a_owner = app.world_mut().spawn_empty().id();
        let b_owner = app.world_mut().spawn_empty().id();
        let a = a_shot(&mut app, ae::Vec2::ZERO, 6, a_owner, "a");
        let b = a_shot(&mut app, ae::Vec2::ZERO, 6, b_owner, "a");

        app.update();

        assert!(!spent(&app, a) && !spent(&app, b), "two team-mates' bolts destroyed each other");
    }

    /// ⛔⛤ NOBODY FIGHTS THEMSELVES, AND FRIENDLY FIRE IS THE ONLY PLACE THAT
    /// CLAIM IS REACHABLE. With teams declared, the team rule already refuses
    /// any same-owner pair, so the guard is unfalsifiable there — the first
    /// version of this arm gave both bolts a team and PASSED with the guard
    /// removed, certifying the team rule under a name about ownership.
    ///
    /// ⭐ Outside a match with friendly fire ON, `can_damage(f, f, ..)` is TRUE,
    /// so a fighter's own two bolts ARE opposed and only the ownership guard
    /// stops them. The control arm beside it fires the same two bolts from two
    /// DIFFERENT owners and they do trade, which is what friendly fire means.
    #[test]
    fn a_fighter_s_own_two_bolts_never_fight_even_under_friendly_fire() {
        let mut app = arena_with_friendly_fire(9.0, true);
        let owner = app.world_mut().spawn_empty().id();
        let a = a_shot_on_team(&mut app, ae::Vec2::ZERO, 6, owner, None);
        let b = a_shot_on_team(&mut app, ae::Vec2::ZERO, 6, owner, None);

        app.update();

        assert!(
            !spent(&app, a) && !spent(&app, b),
            "a fighter's own two bolts destroyed each other"
        );

        // The control: the same two bolts from different owners DO trade, so
        // the arm above is about ownership and not about the fixture.
        let mut app = arena_with_friendly_fire(9.0, true);
        let one = app.world_mut().spawn_empty().id();
        let two = app.world_mut().spawn_empty().id();
        let a = a_shot_on_team(&mut app, ae::Vec2::ZERO, 6, one, None);
        let b = a_shot_on_team(&mut app, ae::Vec2::ZERO, 6, two, None);

        app.update();

        assert!(
            spent(&app, a) && spent(&app, b),
            "friendly fire was declared and two allied bolts still passed \
             through each other, so the arm above proves nothing"
        );
    }

    /// ⛔⛔ THE OTHER HALF OF S0.1: A SWING AND A BOLT ARE ONE CONTEST.
    #[test]
    fn a_shot_and_an_opposed_grounded_swing_trade() {
        let mut app = an_arena(9.0);
        let swinger = a_swinging_fighter(&mut app, ae::Vec2::ZERO, "a", true);
        a_strike_volume(&mut app, swinger, "vol_a", 6);
        let shooter = app.world_mut().spawn_empty().id();
        let shot = a_shot(&mut app, ae::Vec2::ZERO, 6, shooter, "b");

        app.update();

        assert!(spent(&app, shot), "the bolt flew through an opposing swing");
        assert!(
            app.world().get::<ambition_combat::moveset::MovePlayback>(swinger).is_none(),
            "the swing survived a trade with a bolt"
        );
    }

    /// ⭐⭐ THE GROUNDED RULE IS ABOUT SWINGS MEETING SWINGS, AND THIS IS THE ARM
    /// THAT SAYS SO. An aerial passes through an opposing SWING — that is what
    /// keeps the air a place where committing costs you — and it does NOT pass
    /// through a bolt. A contest that reused the melee eligibility test verbatim
    /// would make every aerial immune to projectiles, which is not a rule any
    /// game in this genre has.
    #[test]
    fn an_aerial_swing_still_meets_a_bolt() {
        let mut app = an_arena(9.0);
        let flyer = a_swinging_fighter(&mut app, ae::Vec2::ZERO, "a", false);
        a_strike_volume(&mut app, flyer, "vol_a", 6);
        let shooter = app.world_mut().spawn_empty().id();
        let shot = a_shot(&mut app, ae::Vec2::ZERO, 6, shooter, "b");

        app.update();

        assert!(spent(&app, shot), "a bolt passed through an airborne opponent's attack");
        assert!(
            app.world().get::<ambition_combat::moveset::MovePlayback>(flyer).is_none(),
            "the aerial survived a trade with a bolt"
        );
    }

    /// ⭐ DAMAGE IS THE PROXY WHERE NOTHING AUTHORED A PRIORITY (Jon, 2026-09-11:
    /// *"AUTHORED PRIORITY IS THE ANSWER, IF THERE IS NONE DAMAGE CAN BE A
    /// PROXY"*), and it decides ACROSS families the same way it decides within
    /// one: far enough apart and the stronger attack continues untouched.
    #[test]
    fn a_much_stronger_bolt_beats_a_weak_swing_and_flies_on() {
        let mut app = an_arena(9.0);
        let swinger = a_swinging_fighter(&mut app, ae::Vec2::ZERO, "a", true);
        a_strike_volume(&mut app, swinger, "vol_a", 2);
        let shooter = app.world_mut().spawn_empty().id();
        let shot = a_shot(&mut app, ae::Vec2::ZERO, 30, shooter, "b");

        app.update();

        assert!(!spent(&app, shot), "the stronger bolt was spent by a jab");
        assert!(
            app.world().get::<ambition_combat::moveset::MovePlayback>(swinger).is_none(),
            "the weaker swing survived a much stronger bolt"
        );
    }

    /// ⛔⛤ ONE OWNER'S THREE BOLTS ARE THREE ATTACKS, NOT ONE. Dedup by OWNER
    /// pair — which is right for a move's several volumes — would let the first
    /// bolt's outcome decide whether the second was considered at all, and a
    /// fighter with three in the air would lose one and keep two by id order.
    #[test]
    fn every_bolt_in_a_volley_is_its_own_attack() {
        let mut app = an_arena(9.0);
        let a_owner = app.world_mut().spawn_empty().id();
        let b_owner = app.world_mut().spawn_empty().id();
        let mine: Vec<Entity> = (0..3)
            .map(|_| a_shot(&mut app, ae::Vec2::ZERO, 6, a_owner, "a"))
            .collect();
        let theirs: Vec<Entity> = (0..3)
            .map(|_| a_shot(&mut app, ae::Vec2::ZERO, 6, b_owner, "b"))
            .collect();

        app.update();

        let survivors: Vec<Entity> = mine
            .iter()
            .chain(theirs.iter())
            .copied()
            .filter(|e| !spent(&app, *e))
            .collect();
        assert!(
            survivors.is_empty(),
            "{} of six mutually-overlapping bolts flew on — the contest is \
             deduping by OWNER, so a volley is being treated as one attack",
            survivors.len()
        );
    }

    /// A move with a REAL Active window and a real volume, so
    /// `ambition_combat::moveset::advance_move_playback` actually spawns a strike the arbitration can see.
    ///
    /// ⛔ `uncancelable` authors `windows: vec![]` — it spawns nothing, and a
    /// clank fixture built on it is a fixture that cannot reach the case, which is
    /// the exact defect this whole slice is repairing.
    fn clashing_swing() -> ambition_entity_catalog::MoveSpec {
        // ⚠ LONGER THAN THE FIXTURE'S OWN LOOP. The assertion below is "the
        // move is gone", and a swing that simply RAN OUT inside 40 ticks
        // satisfies it without any contest happening.
        let mut spec = a_swing("swing");
        spec.duration_s = 0.5;
        spec.windows = vec![ambition_entity_catalog::MoveWindow {
            start_s: 0.02,
            end_s: 0.30,
            tag: ambition_entity_catalog::WindowTag::Active,
            volumes: vec![ambition_entity_catalog::HitVolume {
                shape: ambition_entity_catalog::VolumeShape::Rect {
                    // Reaching FORWARD, and wide enough that two fighters 30px apart
                    // meet in the middle.
                    offset: (14.0, 0.0),
                    half_extents: (18.0, 16.0),
                },
                damage: 10,
                knockback: 100.0,
                knockback_growth: None,
                launch_dir: None,
                reaction: None,
                on_hit: None,
                vfx: None,
                hit_sfx: None,
            }],
            motion_scale: 1.0,
            sustain_effect: None,
        }];
        spec
    }

    /// ⭐⭐ TWO REAL AUTHORED ATTACKS TRADE — through `ambition_combat::moveset::advance_move_playback`, which
    /// is the only road that spawns the volumes a match actually contains.
    ///
    /// ⛔⛔ THE DEFECT THIS EXISTS TO PREVENT RECURRING, found by review 2026-08-24:
    /// `arbitrate_attack_clanks` filtered on `With<HitboxLifetime>`, and authored
    /// volumes are spawned with a comment reading *"NO `HitboxLifetime` on purpose"*
    /// because their Active window owns their lifetime. Every Smash jab, tilt, smash
    /// and aerial was invisible to the system — and the tests passed, because they
    /// hand-spawned boxes carrying exactly the component production refuses.
    ///
    /// ⇒ **a synthetic hitbox is not proof of a moveset mechanic.** This fixture
    /// starts two moves and lets the runtime build their volumes.
    #[test]
    fn two_authored_attacks_that_meet_trade_and_both_moves_end() {
        let mut app = App::new();
        app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
        app.init_resource::<ambition_combat::authored_volumes::AuthoredAttackVolumeResolver>();
        app.add_message::<ambition_combat::events::HitEvent>();
        app.add_message::<ambition_combat::hitbox::LandedBodyHit>();
        app.add_message::<ambition_combat::hitbox::ParriedBodyHit>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_combat::moveset::MoveEventMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.add_message::<AttacksClanked>();
        app.init_resource::<ambition_time::WorldTime>();
        app.world_mut().resource_mut::<ambition_time::WorldTime>().scaled_dt = 0.016;
        app.world_mut().resource_mut::<ambition_time::WorldTime>().raw_dt = 0.016;
        app.insert_resource(ambition_combat::rules::ResolvedCombatTuning {
            clank_damage_window: 9.0,
            ..Default::default()
        });
        app.add_systems(
            Update,
            (ambition_combat::moveset::advance_move_playback, arbitrate_attack_clashes).chain(),
        );

        // Two fighters facing each other, close enough that their swings meet.
        let fighter = |app: &mut App, x: f32, faction: ambition_combat::components::ActorFaction, facing: f32| {
            app.world_mut()
                .spawn((
                    ae::BodyKinematics {
                        pos: ae::Vec2::new(x, 0.0),
                        vel: ae::Vec2::ZERO,
                        size: ae::Vec2::new(16.0, 32.0),
                        facing,
                    },
                    faction,
                    ambition_combat::moveset::MovePlayback::new(clashing_swing(), facing),
                ))
                .id()
        };
        let left = fighter(&mut app, 0.0, ambition_combat::components::ActorFaction::Player, 1.0);
        let right = fighter(&mut app, 30.0, ambition_combat::components::ActorFaction::Enemy, -1.0);

        // ⛔ STOP ON THE TICK OF THE TRADE. A Bevy message survives two updates, so
        // running on past it and then reading would report zero announcements and
        // blame the arbitration for the double-buffer.
        let mut ticks = 0;
        while app.world().get::<ambition_combat::moveset::MovePlayback>(left).is_some() && ticks < 40 {
            app.update();
            ticks += 1;
        }
        assert!(
            app.world().get::<ambition_combat::moveset::MovePlayback>(left).is_none(),
            "the left fighter's ATTACK survived the trade — the arbitration is \
             cancelling rectangles rather than attacks, so its sibling and later \
             windows carry on"
        );
        assert!(
            app.world().get::<ambition_combat::moveset::MovePlayback>(right).is_none(),
            "the right fighter's attack survived the trade"
        );

        let messages = app
            .world()
            .resource::<bevy::ecs::message::Messages<AttacksClanked>>();
        let mut cursor = messages.get_cursor();
        let announced: Vec<_> = cursor.read(messages).collect();
        assert_eq!(
            announced.len(),
            1,
            "the trade was announced {} times — one per VOLUME rather than one per \
             ATTACK, which rebounds the same two fighters once for each rectangle",
            announced.len()
        );
        let named = std::collections::BTreeSet::from([announced[0].owners.0, announced[0].owners.1]);
        assert_eq!(
            named,
            std::collections::BTreeSet::from([left, right]),
            "the clank named the wrong bodies"
        );
    }
}
