//! ECS-owned held item capability for actors.
//!
//! The item component is the durable answer to "what is this actor holding?".
//! Brain/action builders may derive an `ActionSet` from it, projectile visuals can
//! route by its id, and future item drops can read the same component without
//! adding archetype-specific Rust branches.

use bevy::prelude::{Commands, Component, Entity, Query};

/// Runtime component attached to actors that are visibly / mechanically holding
/// an item. The spec is data-authored in `character_archetypes.ron` and cloned onto
/// the actor when it spawns or changes state.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct HeldItem {
    pub spec: ambition_characters::brain::HeldItemSpec,
}

impl HeldItem {
    pub fn new(spec: ambition_characters::brain::HeldItemSpec) -> Self {
        Self { spec }
    }

    pub fn id(&self) -> &str {
        self.spec.id.as_str()
    }
}

/// What a MOVE put in this body's hand, and what it displaced.
///
/// ⭐ THE MEMORY IS THE WHOLE POINT. A brandish that simply removed the item
/// afterwards would disarm a fighter who was already carrying one — the pirate
/// raider holds a gun-sword, the heavy holds the heavy one — so the restore has
/// to be "put back exactly what was there", not "take the new one away".
///
/// ⛔ ROLLBACK STATE, and registered as such. It is derived from `MovePlayback`
/// on the tick the move starts and OUTLIVES that tick, which is the definition
/// of state a rewind has to carry: a body rolled back to before the draw must
/// not be holding the sword, and one rolled back to mid-move must be.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct MoveBrandishedItem {
    /// The move that drew it. The brandish ends when this move stops playing —
    /// asking the id rather than "is anything playing" so a move CANCELLED into
    /// another one that brandishes nothing still puts the old item back.
    pub move_id: String,
    /// The SPEC this displaced, restored when the move ends. `None` is a body
    /// that was carrying nothing, which is most of them.
    ///
    /// ⛔⛔ THIS WAS THE ID, AND RE-RESOLVING IT COULD DESTROY THE ITEM. The
    /// restore looked the id up through `ambition_characters::brain::held_item_by_id`
    /// -- ONE of the two held-item registries -- and on `None` it removed the
    /// body's `HeldItem` entirely. `axe` and `javelin` live only in the OTHER
    /// registry (`ambition_held_items::held_spec_for_item`), so a body carrying
    /// one and playing a move that brandishes would have had it deleted rather
    /// than returned. Per item custody I1 the hand IS the record, so that is not
    /// a cosmetic loss: the axe is not in the bag either. It is gone.
    ///
    /// ⛔ AND THE WIDE RESOLVER CANNOT BE CALLED FROM HERE. `held_spec_by_id`
    /// consults both registries, but it lives in `ambition_held_items`, which
    /// DEPENDS on this crate; calling it would be a cycle. The split is forced by
    /// the layering, so no amount of care at the call site fixes it.
    ///
    /// ⭐ SO THE LOOKUP IS GONE INSTEAD OF FIXED. The body HAD the spec; storing
    /// its id and deriving the spec back was a second authority for "what this
    /// body was holding", and the derivation was the part that could fail.
    /// Keeping the spec makes the restore INFALLIBLE -- no registry, no `None`
    /// branch, and no arrangement of ids that loses a weapon. The cost is one
    /// `HeldItemSpec` per brandishing body, the same type `HeldItem` already
    /// keeps in rollback state on that same entity.
    pub previous: Option<ambition_characters::brain::HeldItemSpec>,
}

/// The localizer's window on a brandish: WHICH move drew it and WHAT it
/// displaced.
///
/// ⛔ A PRESENCE PROBE SEES NOTHING OF THE VALUE, and `rollback_exit_oracle`
/// says so by name. Both fields are strings that decide what ends up in a
/// fighter's hand when the move stops, so a rewind that restored the component
/// with the wrong `previous` would be invisible without this.
pub fn move_brandished_item_probe(item: &MoveBrandishedItem) -> u64 {
    fn hash(text: &str) -> u64 {
        // FNV-1a: stable across builds and platforms, which `DefaultHasher` is
        // explicitly not.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in text.as_bytes() {
            h ^= u64::from(*byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }
    let mut out = hash(&item.move_id);
    // ⚠ Still the ID and nothing else: the probe's job is a stable checksum, and
    // widening it to the whole spec would change every recorded value for no
    // gain -- two bodies holding the same id hold the same spec.
    out = out.rotate_left(17) ^ item.previous.as_ref().map_or(0, |spec| hash(&spec.id));
    out
}

/// Put a move's authored weapon in its owner's hand while it plays, and put
/// back what it displaced when it stops.
///
/// ⭐ THE MOVE IS THE AUTHORITY AND ITS CLOCK IS THE TIMER. There is no equip
/// duration to keep in step with the animation, because the brandish IS the
/// move playing — a move that gets a longer windup brandishes for longer by
/// construction, and a move interrupted at frame three puts the sword away at
/// frame three.
///
/// ⛔ IT NEVER TOUCHES A BODY WHOSE ITEM IT DID NOT PUT THERE. The guard is
/// [`MoveBrandishedItem`] rather than "does the held item match what the
/// character authors": a fighter who picked something up off the stage differs
/// from its authored row too, and a rule that could not tell those apart would
/// confiscate the pickup on the next tick.
pub fn brandish_the_playing_move_s_weapon(
    mut commands: Commands,
    bodies: Query<(
        Entity,
        Option<&crate::moveset::MovePlayback>,
        Option<&HeldItem>,
        Option<&MoveBrandishedItem>,
    )>,
) {
    for (entity, playback, held, brandished) in &bodies {
        let wants = playback.and_then(|pb| {
            pb.spec
                .equips
                .as_deref()
                .map(|item| (pb.spec.id.as_str(), item))
        });
        match (wants, brandished) {
            // Already brandishing this move's weapon — nothing to do.
            (Some((move_id, _)), Some(active)) if active.move_id == move_id => {}
            // A move that brandishes has started, or a different one took over.
            (Some((move_id, item)), _) => {
                // ⚠ THE NARROW REGISTRY IS THE RIGHT ONE HERE, unlike the
                // restore below. This resolves what the MOVE authored, and
                // `MoveSpec.equips` is authored in `ambition_characters`, which
                // cannot see `ambition_items` -- so the set a move can name IS
                // this table, by construction. A move cannot brandish `axe` or
                // `javelin`, and that is a coherent limit rather than the bug the
                // restore had: the failure is a WARNING about a name, not the
                // silent loss of an object the body owned.
                let Some(spec) = ambition_characters::brain::held_item_by_id(item) else {
                    // A warning and not a refusal: the move is fine without the
                    // prop, and a silent nothing is how a typo in an id becomes
                    // invisible.
                    bevy::log::warn!(
                        "move `{move_id}` brandishes `{item}`, which is not a registered held item"
                    );
                    continue;
                };
                commands.entity(entity).insert((
                    HeldItem::new(spec),
                    MoveBrandishedItem {
                        move_id: move_id.to_string(),
                        // ⛔ WHAT WAS THERE BEFORE THIS MOVE, which is not
                        // necessarily what the character authors: a body already
                        // brandishing another move's weapon remembers the item
                        // THAT one displaced, so a cancel chain unwinds to the
                        // fighter's own hands rather than to the middle of it.
                        previous: match brandished {
                            Some(active) => active.previous.clone(),
                            None => held.map(|h| h.spec.clone()),
                        },
                    },
                ));
            }
            // The move ended. Put back exactly what it displaced.
            (None, Some(active)) => {
                let mut body = commands.entity(entity);
                body.remove::<MoveBrandishedItem>();
                // ⭐ NO LOOKUP. `previous` IS the spec, so `None` here means the
                // body was carrying nothing before the move -- the one reading
                // that legitimately ends with an empty hand.
                match active.previous.clone() {
                    Some(spec) => {
                        body.insert(HeldItem::new(spec));
                    }
                    None => {
                        body.remove::<HeldItem>();
                    }
                }
            }
            (None, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec};
    use bevy::ecs::system::RunSystemOnce as _;
    use bevy::prelude::*;

    /// End the move the way the engine does.
    ///
    /// ⛔ THROUGH `cancel_move_playback`, NOT BY STRIPPING THE COMPONENT. The
    /// absence contract `ending-a-move-goes-through-the-one-teardown-path`
    /// caught the first version of these tests doing the latter, and it is right
    /// to: a playback removed on its own orphans the move's live strike boxes,
    /// and a test that ends a move by a route the game never takes is measuring
    /// a different situation from the one it names.
    fn end_the_move(app: &mut App, body: Entity) {
        let mut playback = app
            .world()
            .entity(body)
            .get::<crate::moveset::MovePlayback>()
            .expect("the move is playing")
            .clone();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                crate::moveset::cancel_move_playback(&mut commands, body, &mut playback, crate::moveset::MoveEnd::Interrupted);
            })
            .expect("the teardown runs");
    }

    /// A move that draws `item`, or nothing.
    fn drawing(id: &str, item: Option<&str>) -> MoveSpec {
        MoveSpec {
            id: id.to_string(),
            display_name: None,
            clip: ClipBinding {
                clip: "special".to_string(),
                fallbacks: Vec::new(),
            },
            duration_s: 0.5,
            windows: Vec::new(),
            events: Vec::new(),
            gates: MoveGates::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            smash_charge: None,
            charge_gesture: Default::default(),
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
            equips: item.map(str::to_string),
            flow: None,
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_systems(Update, brandish_the_playing_move_s_weapon);
        app
    }

    fn held(app: &App, body: Entity) -> Option<String> {
        app.world()
            .entity(body)
            .get::<HeldItem>()
            .map(|item| item.id().to_string())
    }

    /// ⛔⛔ A WEAPON THIS REGISTRY HAS NEVER HEARD OF STILL COMES BACK.
    ///
    /// The restore used to resolve `previous` through
    /// `ambition_characters::brain::held_item_by_id` -- ONE of the two
    /// held-item registries -- and remove the body's `HeldItem` when that
    /// returned `None`. `axe` and `javelin` live ONLY in the other registry
    /// (`ambition_held_items::held_spec_for_item`), so a body carrying one and
    /// playing a move that brandishes had it DELETED rather than handed back.
    /// Per item-custody I1 the hand is the record, so the axe was not in the bag
    /// either.
    ///
    /// ⭐ This test uses an id NO registry answers to, deliberately. Pinning it
    /// with `axe` would pass again the moment somebody adds `axe` to the narrow
    /// table -- which would fix this one weapon and leave the SHAPE, because the
    /// two registries are forced apart by a dependency edge
    /// (`ambition_held_items` depends on this crate, so the wide resolver cannot
    /// be called from here). The property is "the restore consults nothing", and
    /// an unresolvable id is the only way to state it.
    #[test]
    fn a_carried_weapon_no_registry_knows_is_returned_and_not_destroyed() {
        let mut app = app();
        let carried = ambition_characters::brain::HeldItemSpec {
            id: "a_weapon_no_registry_has_heard_of".to_string(),
            melee: None,
            ranged: None,
            use_behavior: Default::default(),
        };
        assert!(
            ambition_characters::brain::held_item_by_id(&carried.id).is_none(),
            "premise: the id must be unresolvable, or this proves nothing"
        );

        let body = app
            .world_mut()
            .spawn((
                HeldItem::new(carried.clone()),
                crate::moveset::MovePlayback::new(
                    drawing("run_out_the_guns", Some("admiral_gun_sword")),
                    1.0,
                ),
            ))
            .id();

        app.update();
        assert_eq!(
            held(&app, body).as_deref(),
            Some("admiral_gun_sword"),
            "premise: the move must actually draw, or the restore never runs"
        );

        end_the_move(&mut app, body);
        app.update();
        assert_eq!(
            held(&app, body).as_deref(),
            Some(carried.id.as_str()),
            "the body was carrying a weapon this registry cannot resolve; the \
             brandish must hand back what it displaced, not destroy it"
        );
    }

    #[test]
    fn a_move_that_draws_a_weapon_puts_it_in_an_empty_hand_and_takes_it_back() {
        let mut app = app();
        let body = app
            .world_mut()
            .spawn(crate::moveset::MovePlayback::new(
                drawing("run_out_the_guns", Some("admiral_gun_sword")),
                1.0,
            ))
            .id();
        app.update();
        assert_eq!(held(&app, body).as_deref(), Some("admiral_gun_sword"));

        // The move ends. The hand goes back to empty, and the brandish's own
        // memory goes with it.
        end_the_move(&mut app, body);
        app.update();
        assert_eq!(held(&app, body), None, "the gun-sword must be put away");
        assert!(app
            .world()
            .entity(body)
            .get::<MoveBrandishedItem>()
            .is_none());
    }

    /// ⛔⛔ THE ARM THAT MAKES THE MEMORY LOAD-BEARING. A brandish that simply
    /// removed the item afterwards passes the test above and DISARMS a fighter
    /// who was already carrying one — the pirate raider holds a gun-sword all
    /// match, and a side-special would confiscate it.
    #[test]
    fn a_body_that_was_already_armed_gets_its_own_weapon_back() {
        let mut app = app();
        let raiders_own = ambition_characters::brain::held_item_by_id("gun_sword")
            .expect("gun_sword is a registered held item");
        let body = app
            .world_mut()
            .spawn((
                HeldItem::new(raiders_own),
                crate::moveset::MovePlayback::new(
                    drawing("run_out_the_guns", Some("admiral_gun_sword")),
                    1.0,
                ),
            ))
            .id();
        app.update();
        assert_eq!(held(&app, body).as_deref(), Some("admiral_gun_sword"));

        end_the_move(&mut app, body);
        app.update();
        assert_eq!(
            held(&app, body).as_deref(),
            Some("gun_sword"),
            "the body's OWN weapon must come back, not nothing"
        );
    }

    /// ⛔ A CANCEL CHAIN UNWINDS TO THE FIGHTER'S OWN HANDS, not to the middle
    /// of itself: a second drawing move that took over must still remember what
    /// the FIRST one displaced.
    #[test]
    fn a_second_drawing_move_still_remembers_the_body_s_own_weapon() {
        let mut app = app();
        let raiders_own = ambition_characters::brain::held_item_by_id("gun_sword")
            .expect("gun_sword is a registered held item");
        let body = app
            .world_mut()
            .spawn((
                HeldItem::new(raiders_own),
                crate::moveset::MovePlayback::new(
                    drawing("first", Some("admiral_gun_sword")),
                    1.0,
                ),
            ))
            .id();
        app.update();
        app.world_mut().entity_mut(body).insert(
            crate::moveset::MovePlayback::new(drawing("second", Some("gun_sword_heavy")), 1.0),
        );
        app.update();
        assert_eq!(held(&app, body).as_deref(), Some("gun_sword_heavy"));

        end_the_move(&mut app, body);
        app.update();
        assert_eq!(
            held(&app, body).as_deref(),
            Some("gun_sword"),
            "the chain must unwind to the fighter's own weapon, not to the \
             first move's"
        );
    }

    /// ⛔ IT NEVER TOUCHES A BODY WHOSE ITEM IT DID NOT PUT THERE. A pickup that
    /// happened to differ from the character's authored row must survive a move
    /// that brandishes nothing.
    #[test]
    fn a_move_that_draws_nothing_leaves_a_carried_item_alone() {
        let mut app = app();
        let picked_up = ambition_characters::brain::held_item_by_id("gun_sword")
            .expect("gun_sword is a registered held item");
        let body = app
            .world_mut()
            .spawn((
                HeldItem::new(picked_up),
                crate::moveset::MovePlayback::new(drawing("plain_jab", None), 1.0),
            ))
            .id();
        app.update();
        assert_eq!(held(&app, body).as_deref(), Some("gun_sword"));
        assert!(app
            .world()
            .entity(body)
            .get::<MoveBrandishedItem>()
            .is_none());
    }
}
