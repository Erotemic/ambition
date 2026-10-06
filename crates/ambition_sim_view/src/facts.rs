//! The observation-boundary staging ground (E4): small sim-resolved view
//! resources presentation consumes INSTEAD of querying live sim components.
//!
//! Every resource here is a plain-data snapshot rebuilt once per tick in the
//! sim tail (`Platformer2dSimulationPhaseMonolith::FeatureViewSync`) by a function of sim state — no
//! caching across ticks, no `Entity`/`Handle` borrows — so any observer
//! (render, RL, netcode confirmation, the fighter brain) can read the same
//! facts. This module (with `view_index`/`anim_helpers`/`pose_view`/
//! `camera_snapshot`) is the seed of the `ambition_sim_view` crate; it moves
//! wholesale at the E4 mint.

use bevy::prelude::*;

use ambition_characters::actor::{BodyHealth, BodyWallet};
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::resources::{ActorResources, ResourceLevel};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// The controlled body's HUD meters, resolved sim-side (E4 slices 5+6+16):
/// health / mana / wallet follow the [`ControlledSubject`] — while
/// possessing, the HUD shows THAT body's meters, never the vacated home
/// avatar's. `present == false` means no controlled body resolved this tick
/// (startup frames) and the HUD holds its last drawn state.
///
/// The declared readouts read this (Mary-O's coins, Sanic's rings), so they
/// are one per session. The built-in vitals HUD reads each view's
/// [`ViewHudFacts`] instead.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct PlayerHudFacts {
    pub present: bool,
    pub hp_current: i32,
    pub hp_max: i32,
    /// `None` for a body that holds no Mana.
    pub mana: Option<ResourceLevel>,
    pub balance: i32,
}

type HudBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static BodyHealth,
        Option<&'static ActorResources>,
        Option<&'static BodyWallet>,
    ),
>;

/// The meters of `body`, or `None` when it has no health.
fn meters_of(body: Entity, bodies: &HudBodies) -> Option<PlayerHudFacts> {
    let (health, resources, wallet) = bodies.get(body).ok()?;
    Some(PlayerHudFacts {
        present: true,
        hp_current: health.current(),
        hp_max: health.max(),
        mana: ambition_abilities::mana::level(resources),
        balance: wallet.map(|wallet| wallet.balance).unwrap_or(0),
    })
}

pub fn rebuild_player_hud_facts(
    mut facts: ResMut<PlayerHudFacts>,
    controlled: Res<ControlledSubject>,
    bodies: HudBodies,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let subject = controlled.0.or_else(|| primary.single().ok());
    match subject.and_then(|body| meters_of(body, &bodies)) {
        Some(meters) => *facts = meters,
        None => facts.present = false,
    }
}

/// THE METERS ONE VIEW'S HUD SHOWS (Q150: a HUD per participant).
///
/// A view that follows a body or a seat shows the meters of the body it
/// follows. A view that names nothing shows the controlled body's, as its
/// camera frames that body. Before this, every HUD showed the controlled body,
/// so Bob's view in his own live room showed Alice's health.
///
/// A view that names a body or a seat that does not resolve holds its last
/// state (`present == false`). It must not show the meters of another
/// participant.
#[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
pub struct ViewHudFacts(pub PlayerHudFacts);

/// THE OTHER PARTICIPANTS ON ONE VIEW'S SCREEN, by seat (Q150: a HUD per
/// participant, also on a merged screen).
///
/// The first view that names nothing frames the controlled body. Each other
/// seat whose driven body is in that body's live room, and that no other view
/// follows, is on the same screen, and has its own HUD in that view. This
/// holds their meters, in seat order. It is empty on every other view: a view
/// that follows a body or a seat shows that body only.
///
/// Every seat is a local seat here. An online peer that must show only its
/// own seats needs the client-local view layout (multiplayer A4).
#[derive(Component, Default, Clone, Debug, PartialEq)]
pub struct SharedViewHudFacts(pub Vec<(ambition_characters::control::PlayerSlot, PlayerHudFacts)>);

/// THE SEAT WHOSE BODY A VIEW'S OWN HUD SHOWS, so that two HUDs on one
/// screen can say whose each is: the seat that drives the body it shows
/// (the controlled body holds `DrivingParticipant(PRIMARY)`). `None` when
/// the view shows no body, or a body that no seat drives (a view that
/// follows an NPC).
#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewHudSeat(pub Option<ambition_characters::control::PlayerSlot>);

/// Fill each view's [`ViewHudFacts`] from the subject that view resolved,
/// and the [`SharedViewHudFacts`] of the merged screen. Runs after
/// `resolve_view_subjects`, in the camera observation chain.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn rebuild_view_hud_facts(
    mut views: Query<
        (
            Entity,
            &crate::local_view::LocalViewId,
            &mut ViewHudFacts,
            Option<&mut SharedViewHudFacts>,
            Option<&mut ViewHudSeat>,
            &crate::local_view::ResolvedViewSubject,
            Option<&crate::local_view::ViewSubject>,
            Option<&crate::local_view::ViewParticipant>,
        ),
        With<crate::local_view::LocalView>,
    >,
    controlled: Option<Res<ControlledSubject>>,
    bodies: HudBodies,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    drivers: Query<(Entity, &ambition_characters::control::DrivingParticipant)>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    let controlled = controlled
        .and_then(|controlled| controlled.0)
        .or_else(|| primary.single().ok());
    // The seats and the bodies a view follows by name have their own view.
    let mut followed_seats = Vec::new();
    let mut followed_bodies = Vec::new();
    let mut merged = None;
    for (view, id, _, _, _, _, subject, participant) in views.iter() {
        match (subject, participant) {
            (Some(subject), _) => followed_bodies.push(subject.0),
            (None, Some(participant)) => followed_seats.push(participant.0),
            (None, None) => {
                if merged.is_none_or(|(first, _)| (*id, view) < first) {
                    merged = Some(((*id, view), view));
                }
            }
        }
    }
    let merged = merged.map(|(_, view)| view);
    let shared: Vec<(ambition_characters::control::PlayerSlot, PlayerHudFacts)> = match controlled {
        Some(subject) if merged.is_some() => {
            let room = live.of(subject);
            let mut slots: Vec<_> = drivers.iter().map(|(_, driver)| driver.0).collect();
            slots.sort_unstable();
            slots.dedup();
            slots
                .into_iter()
                .filter(|slot| !followed_seats.contains(slot))
                .filter_map(|slot| {
                    let body = ambition_platformer2d_actor_monolith::control::body_driving_seat(
                        &drivers, slot,
                    )?;
                    let shares = body != subject
                        && !followed_bodies.contains(&body)
                        && live.of(body) == room;
                    shares.then(|| meters_of(body, &bodies).map(|meters| (slot, meters)))?
                })
                .collect()
        }
        _ => Vec::new(),
    };
    for (view, _, mut facts, shared_facts, seat, resolved, subject, participant) in &mut views {
        let subject = if subject.is_some() || participant.is_some() {
            resolved.0
        } else {
            controlled
        };
        if let Some(mut seat) = seat {
            let next = subject.and_then(|body| drivers.get(body).ok().map(|(_, driver)| driver.0));
            seat.set_if_neq(ViewHudSeat(next));
        }
        let next = subject
            .and_then(|body| meters_of(body, &bodies))
            .unwrap_or(PlayerHudFacts {
                present: false,
                ..facts.0
            });
        facts.set_if_neq(ViewHudFacts(next));
        if let Some(mut shared_facts) = shared_facts {
            let next = if Some(view) == merged { shared.clone() } else { Vec::new() };
            shared_facts.set_if_neq(SharedViewHudFacts(next));
        }
    }
}

/// EVERY body's held item, resolved sim-side: the geometry facts the hand-sprite
/// needs plus the item identity and its brain-resolved aim (so a possessed
/// body's ranged item points where THAT body aims).
///
/// ⛔ Not every holder — every holder [`drawn_in_the_hand`] claims. The
/// over-hand road takes the rest; see that function for why the line is there.
#[derive(Resource, Default, Clone, Debug)]
pub struct HeldItemView(pub Vec<HeldItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct HeldItemFact {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub facing: f32,
    pub item_id: String,
    pub ranged: bool,
    pub aim: ae::Vec2,
    /// The live room of the thing this row draws (`LiveRooms::of`). The
    /// visual is placed by that room's geometry and stamped with it.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// WHICH OF THE TWO HELD-ITEM ROADS A HOLDER IS ON, stated once.
///
/// ⛔⛔ THE TWO ROADS MUST PARTITION THE HOLDERS — every holder on exactly one.
/// [`HeldItemView`] draws the item IN the hand from `HeldItemArtManifest`;
/// [`HostileWieldedItemsView`] draws it OVER the hand from
/// `WieldedItemVisualCatalog`, scaled to the wielder and gripped at an authored
/// point. Both registries carry `gun_sword`, so a holder on both roads is drawn
/// TWICE, and a holder on neither is drawn not at all — which is the defect this
/// function exists to make impossible to reintroduce in one road alone.
///
/// The line is the over-hand road's own admission test, hoisted out of it: a
/// living body with a hostile disposition. A match fighter carries no
/// `ActorDisposition` at all and a peaceful NPC's is `Peaceful`, so both fall to
/// the in-hand road, which is where their art is registered.
pub fn drawn_over_the_hand(
    disposition: Option<ambition_combat::components::ActorDisposition>,
    alive: Option<bool>,
) -> bool {
    disposition.is_some_and(|d| !d.is_peaceful()) && alive.unwrap_or(false)
}

/// The complement of [`drawn_over_the_hand`], for the road that reads it.
pub fn drawn_in_the_hand(
    disposition: Option<ambition_combat::components::ActorDisposition>,
    alive: Option<bool>,
) -> bool {
    !drawn_over_the_hand(disposition, alive)
}

#[allow(clippy::type_complexity)]
pub fn rebuild_held_item_view(
    mut view: ResMut<HeldItemView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: Query<(
        bevy::prelude::Entity,
        &BodyKinematics,
        &ambition_platformer2d_actor_monolith::features::HeldItem,
        &ActorControl,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        Option<&ambition_combat::components::ActorDisposition>,
        Option<&BodyHealth>,
    )>,
) {
    // ⛔⛔ THIS READ ONLY `ControlledSubject`, so exactly ONE held item was ever
    // drawn — in a couch match seat one's drawn weapon was invisible, and in a
    // CPU-versus-CPU match neither Admiral's gun-sword showed at all. The
    // argument against that is written twelve lines below for a sibling view:
    // *"it is plural: a couch-versus match has two driven bodies and neither is
    // more protected than the other, because a rule that privileges one
    // participant stops being a rule about bodies."* Same rule, same reason.
    //
    // ⛔⛔ AND `With<DrivingParticipant>` WAS STILL THE WRONG POPULATION, which
    // the plural fix did not go far enough to see: a weapon is visible because a
    // BODY holds it, not because a human drives that body. The Admiral's side-B
    // draws `admiral_gun_sword`, which is registered in the IN-HAND art manifest
    // and nowhere else, so a CPU Admiral's gun-sword was drawn by no road at all
    // — and a match fighter carries no `ActorDisposition`, so the over-hand road
    // could not pick it up either.
    //
    // ⇒ every holder now, minus the ones the over-hand road claims. See
    // [`drawn_over_the_hand`] for why that subtraction is a partition and not a
    // carve-out.
    //
    // ⛔ SORTED BY `SimId`, because this is a Vec built in query order and the
    // consumer spawns one visual per row: unsorted, two held items would swap
    // draw order between runs.
    let mut rows: Vec<_> = bodies
        .iter()
        .filter(|(_, _, _, _, _, disposition, health)| {
            drawn_in_the_hand(disposition.copied(), health.map(|health| health.alive()))
        })
        .map(|(body, kin, held, control, id, _, _)| {
            (
                id.clone(),
                HeldItemFact {
                    pos: kin.pos,
                    size: kin.size,
                    facing: kin.facing,
                    item_id: held.spec.id.clone(),
                    ranged: held.spec.ranged.is_some(),
                    aim: control.0.aim.vec(),
                    room: live.of(body),
                },
            )
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    view.0.clear();
    view.0.extend(rows.into_iter().map(|(_, fact)| fact));
}

/// The box of every body a participant is DRIVING this tick.
///
/// Presentation needs this to know what it must not obscure — the world-label
/// placement pass dims a label that would be drawn across a driven body rather
/// than shoving the label aside (`ambition_render::rendering::label_layout`).
///
/// Derived from WHO DRIVES the body, not from a player marker, for two reasons.
/// First, possession is a SEAT REDIRECT, so a possessed enemy carries the seat
/// and the vacated home avatar does not — asking who holds the seat gets that
/// right for free. Second, it is plural: a couch-versus match has two driven
/// bodies and neither is more protected than the other, because a rule that
/// privileges one participant stops being a rule about bodies.
///
/// Note what this is NOT: it is not the nameplate index's `controlled` flag.
/// That flag lives on rows keyed by `FeatureId`, and the home avatar carries
/// no `FeatureId` at all — so the flag is only ever true while possessing a
/// feature actor. A label-occlusion rule built on it would have protected
/// every body EXCEPT the one you normally play.
#[derive(Resource, Default, Clone, Debug)]
pub struct ControlledBodiesView(pub Vec<ControlledBodyFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlledBodyFact {
    pub center: ae::Vec2,
    pub size: ae::Vec2,
    /// The live room of the body (`LiveRooms::of`). A view keeps labels off
    /// only the bodies of the room it frames.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

pub fn rebuild_controlled_bodies_view(
    mut view: ResMut<ControlledBodiesView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: Query<(Entity, &BodyKinematics), With<ambition_characters::control::DrivingParticipant>>,
) {
    // AMBITION_REVIEW(determinism): query order is not stable, and this Vec is
    // built in it. Safe: the only consumer asks "does any of these boxes
    // overlap mine", which is order-independent, and this is derived
    // presentation state that never enters a sim trajectory.
    view.0.clear();
    view.0.extend(bodies.iter().map(|(entity, kin)| ControlledBodyFact {
        center: kin.pos,
        size: kin.size,
        room: live.of(entity),
    }));
}

/// Every ground item's visual facts (position, box, item id).
#[derive(Resource, Default, Clone, Debug)]
pub struct GroundItemsView(pub Vec<GroundItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct GroundItemFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
    pub item_id: String,
    /// The live room of the thing this row draws (`LiveRooms::of`). The
    /// visual is placed by that room's geometry and stamped with it.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// only items that are IN THE WORLD. A picked-up item is no longer
/// destroyed — it keeps its entity and its identity and records that a body is
/// carrying it (`ItemCustody`) — so "there is a `GroundItem` component" stopped
/// meaning "there is an axe lying over there". The in-hand overlay is a separate
/// view (`HeldItemView`) drawn from the holder, and publishing a carried item
/// here would draw it twice: once in the hand and once on the floor where it was
/// grabbed.
pub fn rebuild_ground_items_view(
    mut view: ResMut<GroundItemsView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    grounds: Query<(
        bevy::prelude::Entity,
        &ambition_held_items::GroundItem,
        &ambition_held_items::ItemCustody,
    )>,
) {
    view.0.clear();
    view.0.extend(
        grounds
            .iter()
            .filter(|(_, _, custody)| custody.in_world())
            .map(|(item, ground, _)| GroundItemFact {
                pos: ground.pos,
                half_extent: ground.half_extent,
                item_id: ground.spec.id.clone(),
                room: live.of(item),
            }),
    );
}

/// Every walk-into world item's visual facts (position, box, the row it grants —
/// so the renderer can pick an icon/tint per pickup).
#[derive(Resource, Default, Clone, Debug)]
pub struct WorldItemsView(pub Vec<WorldItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct WorldItemFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
    /// The equipment row id the item grants (e.g. `"grow_cap"`), used only to
    /// choose the visual. An empty string if the payload has no id.
    pub row_id: String,
    /// Optional presentation art id (e.g. `"super_mary_o_milk_carton"`) the render
    /// layer resolves to a real sprite; `None` draws the row-tinted quad.
    pub sprite: Option<String>,
    /// Still emerging from whatever produced it — draw it BEHIND the world.
    ///
    /// DERIVED from the motion, never mirrored from the item. `WorldItem` carried an
    /// `emerging: bool` that Mary-O set `true` at spawn and NOTHING ever set back to `false`,
    /// so a wand finished rising, began its ordinary arc, and stayed drawn behind the world for
    /// the rest of its life.
    ///
    /// the motion already knew — `ItemMotion::emerging()` compares elapsed rise
    /// against the authored one. A second mutable copy of a fact the simulation
    /// derives per frame can only ever go stale; this asks the one that cannot.
    pub emerging: bool,
    /// The live room of the thing this row draws (`LiveRooms::of`). The
    /// visual is placed by that room's geometry and stamped with it.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

pub fn rebuild_world_items_view(
    mut view: ResMut<WorldItemsView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    items: Query<(
        bevy::prelude::Entity,
        &ambition_world_items::world_item::WorldItem,
        Option<&ambition_world_items::item_motion::ItemMotion>,
    )>,
) {
    use ambition_world_items::world_item::WorldItemPayload;
    view.0.clear();
    view.0
        .extend(items.iter().map(|(entity, item, motion)| WorldItemFact {
            pos: item.pos,
            half_extent: item.half_extent,
            row_id: match &item.payload {
                WorldItemPayload::Equip(row) => row.id.clone(),
            },
            sprite: item.sprite.clone(),
            // An item with no motion is not rising: a dropped or authored item sits
            // where it is, and belongs in front of the world like any other pickup.
            emerging: motion.is_some_and(|motion| motion.emerging()),
            room: live.of(entity),
        }));
}

/// Every player's dropped recall-mark position, with the live room it was
/// dropped in (for a mark with no room, its player's room).
#[derive(Resource, Default, Clone, Debug)]
pub struct MarkBeaconsView(
    pub Vec<(ae::Vec2, Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>)>,
);

pub fn rebuild_mark_beacons_view(
    mut view: ResMut<MarkBeaconsView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    marks: Query<(
        Entity,
        &ambition_abilities::traversal::mark_recall::PlayerMark,
    )>,
) {
    view.0.clear();
    view.0.extend(
        marks
            .iter()
            .filter_map(|(entity, mark)| Some((mark.pos?, mark.room.or_else(|| live.of(entity))))),
    );
}

/// A countdown riding one body that the PLAYER must be able to read.
///
/// ⭐ THE GENERIC HALF OF A STATUS TELEGRAPH. A delayed mark, a poison, a
/// burning fuse — anything that puts a clock on a body and sells the read — is
/// this row: where the body is, how tall it is, and how much of the clock is
/// left. Presentation draws the row and knows nothing about which mechanic wrote
/// it, which is the E4 rule every other view here follows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyClockFact {
    /// The body the clock rides. A drawable that presents it says so with
    /// `PresentationOf(body)`, which is how a portal pane learns to clip it.
    pub body: Entity,
    pub pos: ae::Vec2,
    /// Half the body's height, so the telegraph can sit above the head.
    pub half_height: f32,
    /// `1.0` fresh down to `0.0` on the tick it runs out.
    pub remaining_fraction: f32,
}

/// Every readable clock on every body this tick.
///
/// ⚠ CLEARED HERE, FILLED ELSEWHERE. This crate cannot see the mechanics that
/// put clocks on bodies — the delayed mark is a smash ruleset component — so
/// `rebuild_body_clocks_view` only empties the row set, and each contributor
/// pushes its clocks ordered `.after` it in the same phase. A contributor that
/// forgot the ordering would race the clear and flicker, which is visible.
#[derive(Resource, Default, Clone, Debug)]
pub struct BodyClocksView(pub Vec<BodyClockFact>);

/// The body-clock view's two phases, published so a contributor orders against
/// vocabulary rather than against this crate's private clearer.
///
/// ⭐ `Reset` empties the view; `Contribute` is where every mechanic pushes its
/// clocks. Both live inside `FeatureViewSync`, chained here once, so a ruleset
/// adding clocks writes `.in_set(BodyClockViewSet::Contribute)` and nothing
/// else -- and prerequisite C1 (no cross-crate private-system ordering) holds.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyClockViewSet {
    Reset,
    Contribute,
}

pub fn rebuild_body_clocks_view(mut view: ResMut<BodyClocksView>) {
    view.0.clear();
}

/// Every heal shrine's geometry.
#[derive(Resource, Default, Clone, Debug)]
pub struct ShrinesView(pub Vec<ShrineFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShrineFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
    /// The live room of the thing this row draws (`LiveRooms::of`). The
    /// visual is placed by that room's geometry and stamped with it.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

pub fn rebuild_shrines_view(
    mut view: ResMut<ShrinesView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    shrines: Query<(Entity, &ambition_platformer2d_actor_monolith::shrine::HealShrine)>,
) {
    view.0.clear();
    view.0.extend(shrines.iter().map(|(entity, shrine)| ShrineFact {
        pos: shrine.pos,
        half_extent: shrine.half_extent,
        room: live.of(entity),
    }));
}

pub fn tick_shrine_activation_pulse(
    world_time: Res<ambition_time::WorldTime>,
    mut activation: ResMut<ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse>,
) {
    if activation.remaining > 0.0 {
        activation.remaining = (activation.remaining - world_time.sim_dt()).max(0.0);
    }
}

/// Presentation facts for every living hostile actor wielding an item: the
/// authored item id, hand position, aim target, and wielder height. The sim
/// publishes the open identity; presentation catalogs decide which ids have a
/// visible over-hand prop and how that prop is drawn.
#[derive(Resource, Default, Clone, Debug)]
pub struct HostileWieldedItemsView(pub Vec<HostileWieldedItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct HostileWieldedItemFact {
    pub item_id: String,
    pub hand_world: ae::Vec2,
    pub aim_world: ae::Vec2,
    pub wielder_height: f32,
    /// The live room of the thing this row draws (`LiveRooms::of`). The
    /// visual is placed by that room's geometry and stamped with it.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// A WIELDER AIMS AT WHAT IT IS FIGHTING, not at "the player".
///
/// This took `Query<&BodyKinematics, PrimaryPlayerOnly>`, `single()`d it, and
/// `return`ed without one — so in a match, where no session home avatar exists,
/// every hostile wielder's held item vanished from the view entirely. And when
/// there WAS a player the fact was still wrong for a match: two fighters both
/// aimed their weapons at a third body neither was fighting.
///
/// The controlled subject is the fallback for a wielder with no target — an exploration enemy
/// that has not acquired one still points its pistol at the person it is menacing — and a
/// wielder with neither is simply aimed where it faces, which is a fact rather than a hole.
#[allow(clippy::type_complexity)]
pub fn rebuild_hostile_wielded_items_view(
    mut view: ResMut<HostileWieldedItemsView>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    wielders: Query<(
        Entity,
        &ambition_combat::components::ActorDisposition,
        &ambition_platformer2d_actor_monolith::features::HeldItem,
        Option<&BodyKinematics>,
        Option<&BodyHealth>,
        Option<&ambition_combat::components::ActorTarget>,
    )>,
    bodies: Query<&BodyKinematics>,
    controlled: Res<ControlledSubject>,
    player_q: Query<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    // Where a wielder holds its item: the simulation's own answer, which the
    // muzzle of a hand weapon also reads, so the prop is drawn where the shot
    // leaves.
    landmarks: ambition_combat::body_landmarks::BodyLandmarks,
) {
    view.0.clear();
    // The session's own subject, for a wielder that has acquired nothing. `None`
    // in a match with no local participant, which is legitimate rather than a
    // reason to publish nothing.
    let subject_pos = controlled
        .0
        .and_then(|entity| bodies.get(entity).ok())
        .or_else(|| player_q.single().ok())
        .map(|kin| kin.pos);
    for (entity, disposition, held_item, kin, health, target) in &wielders {
        let Some(kin) = kin else {
            continue;
        };
        // ⛔ THE SAME LINE THE IN-HAND ROAD READS, so the two cannot both claim a
        // holder or both skip one.
        if !drawn_over_the_hand(Some(*disposition), health.map(|health| health.alive())) {
            continue;
        }
        let wielder_height = kin.size.y;
        // Its own target first; the session subject second; where it faces last.
        let aim_world = target
            .and_then(|target| target.entity)
            .and_then(|entity| bodies.get(entity).ok())
            .map(|kin| kin.pos)
            .or(subject_pos)
            .unwrap_or_else(|| kin.pos + ae::Vec2::new(kin.facing * wielder_height, 0.0));
        view.0.push(HostileWieldedItemFact {
            item_id: held_item.id().to_owned(),
            hand_world: ambition_held_items::holding_hand_world(&landmarks, entity, kin, ae::Vec2::Y),
            aim_world,
            wielder_height,
            room: live.of(entity),
        });
    }
}

/// Render queries ONLY this component — never the live `BodyKinematics` — and resolves `visual_id`
/// through the content-owned `ProjectileVisualCatalog`. Removed when a pooled projectile stops
/// being live.
#[derive(Component, Clone, Debug)]
pub struct ProjectileView {
    pub visual_id: String,
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub size: ae::Vec2,
}

#[allow(clippy::type_complexity)]
pub fn rebuild_projectile_views(
    mut commands: Commands,
    mut live: Query<
        (
            Entity,
            &BodyKinematics,
            &ambition_projectiles::ProjectileVisualId,
            Option<&mut ProjectileView>,
        ),
        With<ambition_projectiles::LiveProjectile>,
    >,
    // Pooled projectiles: a reused entity that is no longer live must drop
    // its view so render despawns the visual instead of drawing a corpse.
    stale: Query<
        Entity,
        (
            With<ProjectileView>,
            Without<ambition_projectiles::LiveProjectile>,
        ),
    >,
) {
    for (entity, kin, visual_id, view) in &mut live {
        let next = ProjectileView {
            visual_id: visual_id.0.clone(),
            pos: kin.pos,
            vel: kin.vel,
            size: kin.size,
        };
        match view {
            Some(mut view) => *view = next,
            None => {
                commands.entity(entity).insert(next);
            }
        }
    }
    for entity in &stale {
        commands.entity(entity).remove::<ProjectileView>();
    }
}

#[derive(Clone, Debug)]
pub struct DynamicFeatureFact {
    pub id: String,
    /// Display label for the visual's debug `Name`.
    pub label: String,
    /// Family label ("Encounter mob" / "Staged actor" / "Post-boss NPC" /
    /// "Reward chest" / "Dropped pickup") — presentation naming only.
    pub family: &'static str,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub visual_kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind,
    pub fighting: bool,
    /// The placeholder entity-sprite the spawn resolves to (from the actor's
    /// brain / the NPC's interactable / the chest payload).
    pub sprite_key: Option<ambition_sprite_sheet::game_assets::EntitySprite>,
    /// An ANIMATED prop-sheet id to draw instead of the placeholder (a spinning
    /// ring, a pulsing gem) — the same `GameAssets.characters.props` key the
    /// room-load pass resolves for an authored pickup. `None`  the placeholder.
    pub prop_sheet: Option<String>,
    /// The live room of the body this fact draws, by the rule of
    /// `LiveRooms::of`. The visual is placed by that room's geometry and
    /// stamped with it. `None` when the room cannot be told.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct DynamicFeatureViews(pub Vec<DynamicFeatureFact>);

#[allow(clippy::type_complexity)]
pub fn rebuild_dynamic_feature_views(
    mut view: ResMut<DynamicFeatureViews>,
    // The live room of each body, so its visual is drawn in that room.
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    ecs_mobs: Query<
        (
            bevy::prelude::Entity,
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::ActorDisposition,
            Option<(
                &ambition_combat::actor_tuning::ActorConfig,
                &ambition_combat::components::ActorIdentity,
            )>,
        ),
        With<ambition_combat::components::EncounterMob>,
    >,
    staged_actors: Query<
        (
            bevy::prelude::Entity,
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::ActorDisposition,
            Option<(
                &ambition_combat::actor_tuning::ActorConfig,
                &ambition_combat::components::ActorIdentity,
            )>,
        ),
        With<ambition_combat::components::RuntimeStagedActor>,
    >,
    post_boss_npcs: Query<
        (
            bevy::prelude::Entity,
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::FeatureName,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::ActorDisposition,
            Option<&ambition_combat::actor_tuning::ActorConfig>,
            Option<&ambition_combat::components::ActorInteraction>,
        ),
        With<ambition_combat::components::PostBossNpc>,
    >,
    ecs_reward_chests: Query<
        (
            bevy::prelude::Entity,
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::ChestFeature,
        ),
        bevy::prelude::Or<(
            With<ambition_combat::components::EncounterRewardChest>,
            With<ambition_combat::components::BossRewardChest>,
        )>,
    >,
    // Loot the running simulation MINTED — Sanic's scattered rings, and every
    // future drop. Selected by construction PROVENANCE (`SpawnOrigin::Dynamic`)
    // rather than a per-game marker: "this pickup was not in the room spec" is
    // exactly the condition under which the room-load visual pass could not have
    // seen it, so it is exactly the set that needs discovering here. An authored
    // pickup already has its visual and is filtered out below.
    dropped_pickups: Query<
        (
            bevy::prelude::Entity,
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::FeatureName,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::PickupFeature,
            &ambition_platformer2d_shared_tangle::construction::SpawnOrigin,
            Option<&ambition_platformer2d_actor_monolith::features::PickupArt>,
        ),
        Without<ambition_combat::components::Collected>,
    >,
) {
    use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
    use ambition_sprite_sheet::game_assets;
    view.0.clear();
    for (entity, id, aabb, disposition, config) in &ecs_mobs {
        // ⛔⛔ "PEACEFUL" IS NOT "DOES NOT EXIST". Skipping a peaceful mob here on
        // the argument that encounter mobs are hostile by construction publishes
        // no `DynamicFeatureFact`, so `spawn_dynamic_feature_visuals` never makes
        // it a `FeatureVisual` and it has NO SPRITE however healthy its art is.
        // That is how the pirate's summoned shark comes out INVISIBLE: it is
        // deliberately nobody's enemy, and the targeting stand-down marks even an
        // unengaged hostile actor peaceful. ⚠ Every actor-side measurement says
        // the body is fine, because it is — the renderer is never asked.
        //
        // ⭐ THE FIELD FOR THIS ALREADY EXISTED. `fighting` is exactly the
        // distinction the skip was abusing existence to express, and the
        // post-boss arm below has always used it that way. Two arms disagreed
        // with a third in the same function.
        let Some((config, identity)) = config else {
            continue;
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: identity.name.clone(),
            family: "Encounter mob",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting: !disposition.is_peaceful(),
            // ⚠ STILL THE BRAIN'S KEY for a peaceful one. Unlike a post-boss NPC
            // there is no dialogue interactable to resolve art from, and an
            // encounter mob's art is its own either way — a shark that stops
            // hunting is still a shark.
            sprite_key: game_assets::entity_sprite_for_enemy(&config.brain),
            prop_sheet: None,
            room: live.of(entity),
        });
    }
    for (entity, id, aabb, disposition, config) in &staged_actors {
        // The same correction as the arm above: a staged actor that is not
        // fighting is still a body somebody has to be able to see.
        let Some((config, identity)) = config else {
            continue;
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: identity.name.clone(),
            family: "Staged actor",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting: !disposition.is_peaceful(),
            sprite_key: game_assets::entity_sprite_for_enemy(&config.brain),
            prop_sheet: None,
            room: live.of(entity),
        });
    }
    for (entity, id, name, aabb, disposition, config, interaction) in &post_boss_npcs {
        let fighting = !disposition.is_peaceful();
        // A peaceful post-boss NPC resolves its sprite from the dialogue
        // interactable; a hostile one (provoked) from its archetype brain.
        let sprite_key = if disposition.is_peaceful() {
            match interaction {
                Some(i) => game_assets::entity_sprite_for_runtime_interactable(&i.interactable),
                None => continue,
            }
        } else {
            match config {
                Some(c) => game_assets::entity_sprite_for_enemy(&c.brain),
                None => continue,
            }
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: name.0.clone(),
            family: "Post-boss NPC",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting,
            sprite_key,
            prop_sheet: None,
            room: live.of(entity),
        });
    }
    for (entity, id, aabb, chest) in &ecs_reward_chests {
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: id.as_str().to_string(),
            family: "Reward chest",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Chest,
            fighting: false,
            sprite_key: game_assets::entity_sprite_for_runtime_chest(&chest.chest),
            prop_sheet: None,
            room: live.of(entity),
        });
    }
    for (entity, id, name, aabb, pickup, origin, art) in &dropped_pickups {
        if !matches!(
            origin,
            ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Dynamic { .. }
        ) {
            continue;
        }
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: name.0.clone(),
            family: "Dropped pickup",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Pickup,
            fighting: false,
            // The static per-kind fallback, used only when the drop names no animated sheet or
            // that sheet hasn't loaded.
            sprite_key: game_assets::entity_sprite_for_runtime_pickup(pickup.kind()),
            prop_sheet: art.map(|art| art.0.clone()),
            room: live.of(entity),
        });
    }
}

/// Render draws the ember ring; it computes nothing.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct BlinkPreviewFact {
    /// Ring visible this tick (blink held / aiming, ability owned, gameplay
    /// allowed).
    pub active: bool,
    /// Predicted landing point.
    pub target: ae::Vec2,
    /// Precision (steered) aim vs quick-tap — picks the ember palette.
    pub precision: bool,
    /// The blinking body's smaller AABB extent — ring radius + ember size
    /// scale off it.
    pub body_min_extent: f32,
    /// The live room of the blinking body (`LiveRooms::of`). The ring is
    /// placed by that room's geometry and stamped with it.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// Rebuild [`BlinkPreviewFact`] each tick. Mirrors the destination
/// resolution used by the engine and the `show_blink_preview` debug overlay.
/// The blink button shares ground with menu input, so this honours the same
/// gameplay-only gate as `draw_player_debug` — paused / dialog states don't
/// light up the ring.
#[cfg(feature = "input")]
#[allow(clippy::type_complexity)]
pub fn rebuild_blink_preview_fact(
    mut fact: ResMut<BlinkPreviewFact>,
    // THE ONE COLLISION READ-API, because the preview was resolving
    // against a DIFFERENT WORLD than the blink. This took the room plus
    // `MovingPlatformSet` and composed `world_with_moving_platforms` itself,
    // under a comment claiming *"the moving-platform-aware temporary world is
    // what the actual blink resolves against"*. That was true when written and
    // is not: the body integrates against `world_with_sandbox_solids`, which
    // ALSO carries the ECS overlay (gate lock-walls, falling-sand pools,
    // broken-brick subtractions) and the portal carves.
    //
    //  the reticle could show a destination through a lock wall the blink
    // stops at, or stop at a portal aperture the blink passes through. A
    // preview that disagrees with the action is worse than none.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    // The subject's own live room (OW1): the blink resolves against that
    // room's walls, so the preview does too.
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mode: Res<bevy::prelude::State<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
    action_query: Query<
        &leafwing_input_manager::prelude::ActionState<
            ambition_input::Platformer2dInputActionMonolith,
        >,
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    // The blink reticle previews from the CONTROLLED SUBJECT (the body
    // holding `DrivingParticipant(PRIMARY)`) — the body you are driving — so it
    // follows a possessed body instead of hovering at the vacated home
    // avatar. Both player and actor bodies carry these blink clusters.
    controlled: Res<ControlledSubject>,
    player_q: Query<(
        &BodyKinematics,
        &ambition_platformer2d_core::BodyAbilities,
        &ambition_platformer2d_core::BodyMotionFacts,
    )>,
) {
    use ambition_input::read_gameplay_control_frame;
    use ambition_platformer2d_core as ae;

    fact.active = false;
    let Some((subject, (kin, abilities, motion_facts))) =
        controlled.0.and_then(|e| player_q.get(e).ok().map(|body| (e, body)))
    else {
        return;
    };
    let actions = if mode.get().allows_gameplay() {
        action_query.single().ok()
    } else {
        None
    };
    let controls = actions.map(read_gameplay_control_frame).unwrap_or_default();

    if !(abilities.abilities.blink && (controls.blink_held || motion_facts.blink_aiming)) {
        return;
    }

    // The SAME composition `step_motion` collides against — see the parameter
    // — in the subject's own live room. The sole live room's was read here,
    // so while two rooms were live no reticle showed.
    let live_room = live.of(subject);
    let room = live_room.map(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance);
    let Some(blink_world) = collision.room(room.as_ref()).and_then(|room| room.solids()) else {
        return;
    };
    let target = if motion_facts.blink_aiming {
        ae::blink_destination_to_point_clusters(
            &blink_world,
            kin,
            abilities,
            kin.pos + motion_facts.blink_aim_offset,
        )
    } else {
        let aim = ae::Vec2::new(controls.axis_x, controls.axis_y)
            .normalize_or(ae::Vec2::new(kin.facing, 0.0));
        ae::blink_destination_clusters(&blink_world, kin, abilities, aim, ae::BLINK_DISTANCE)
    };

    *fact = BlinkPreviewFact {
        active: true,
        target,
        precision: motion_facts.blink_aiming,
        body_min_extent: kin.size.min_element(),
        room: live_room,
    };
}

/// Registers the observation-boundary view resources + their rebuilds in the
/// sim tail. Owned here (anti-god rule 5): the plugin that rebuilds a view
/// initializes it; presentation only reads.
pub struct SimViewPlugin;

impl Plugin for SimViewPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<PlayerHudFacts>()
            .init_resource::<HeldItemView>()
            .init_resource::<ControlledBodiesView>()
            .init_resource::<GroundItemsView>()
            .init_resource::<WorldItemsView>()
            .init_resource::<MarkBeaconsView>()
            .init_resource::<BodyClocksView>()
            .init_resource::<ShrinesView>()
            .init_resource::<HostileWieldedItemsView>()
            .init_resource::<DynamicFeatureViews>()
            .init_resource::<BlinkPreviewFact>();
        // The blink-preview resolve reads device actions, so it exists only
        // with the input layer; the FACT resource above is unconditional so
        // consumers read an inert default headless.
        #[cfg(feature = "input")]
        app.add_systems(
            sim,
            rebuild_blink_preview_fact
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );
        app.add_systems(
            sim,
            (
                rebuild_player_hud_facts,
                rebuild_held_item_view,
                rebuild_controlled_bodies_view,
                rebuild_ground_items_view,
                rebuild_world_items_view,
                rebuild_mark_beacons_view,
                rebuild_body_clocks_view.in_set(BodyClockViewSet::Reset),
                rebuild_shrines_view,
                tick_shrine_activation_pulse,
                rebuild_hostile_wielded_items_view,
                rebuild_projectile_views,
                rebuild_dynamic_feature_views,
            )
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );
        app.configure_sets(
            sim,
            (BodyClockViewSet::Reset, BodyClockViewSet::Contribute)
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐ A PEACEFUL ENCOUNTER MOB IS STILL A BODY SOMEBODY HAS TO SEE.
    ///
    /// ⛔⛔ THIS ARM USED TO DROP THEM, under a comment asserting that *"encounter
    /// mobs are hostile by construction"*. They are not: the pirate's summoned
    /// shark is deliberately nobody's enemy, and the targeting stand-down marks
    /// any unengaged hostile actor peaceful anyway. A dropped fact means no
    /// `DynamicFeatureFact`, so `spawn_dynamic_feature_visuals` never builds a
    /// `FeatureVisual`, so the body has no sprite — however healthy its art is.
    /// Jon played it and saw a debug box hovering where a burning shark should
    /// be, and every measurement of the ACTOR came back correct, because the
    /// actor was correct. Presentation was never asked to draw it.
    ///
    /// ⭐ IT ASSERTS `fighting` TOO, not just presence. `fighting` is the field
    /// the skip was abusing existence to express, and a fix that published the
    /// mob while still calling it a fighter would trade one wrong answer for
    /// another.
    #[test]
    fn a_peaceful_encounter_mob_still_publishes_a_visual_fact() {
        use ambition_combat::actor_tuning::ActorConfig;
        use ambition_combat::components::{
            ActorDisposition, CenteredAabb, EncounterMob, FeatureId,
        };
        let mut app = App::new();
        app.init_resource::<DynamicFeatureViews>();
        app.add_systems(Update, rebuild_dynamic_feature_views);

        let config = ActorConfig {
            tuning: Default::default(),
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                "burning_flying_shark".into(),
            ),
            preserves_mirror_symmetry: false,
        };
        app.world_mut().spawn((
            EncounterMob {
                encounter_id: "smash".into(),
            },
            FeatureId("smash_ride_shark".to_string()),
            CenteredAabb::new(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(48.0, 22.0)),
            ActorDisposition::Peaceful,
            config,
            ambition_combat::components::ActorIdentity::new(
                "smash_ride_shark",
                "Burning Flying Shark",
            ),
        ));
        app.update();

        let views = app.world().resource::<DynamicFeatureViews>();
        let shark = views
            .0
            .iter()
            .find(|fact| fact.id == "smash_ride_shark")
            .expect(
                "a peaceful encounter mob published no visual fact, so nothing \
                 downstream will ever give it a sprite",
            );
        assert!(
            !shark.fighting,
            "a peaceful mob was published as a fighter, which is the opposite \
             error from the one this test exists for"
        );
    }

    /// Q150: two views, each following its own seat, show the meters of
    /// their own seat's body. The control is a third view that names
    /// nothing: it shows the controlled body. And a view whose seat has no
    /// body holds, and does not show another participant's meters.
    #[test]
    fn each_view_shows_the_meters_of_the_seat_it_follows() {
        use crate::local_view::{
            resolve_view_subjects, LocalView, LocalViewId, ResolvedViewSubject, ViewParticipant,
        };
        use ambition_characters::actor::Health;
        use ambition_characters::control::{DrivingParticipant, PlayerSlot};
        let mut app = App::new();
        app.add_systems(Update, (resolve_view_subjects, rebuild_view_hud_facts).chain());
        let body = |world: &mut World, slot: u8, damage: i32, balance: i32| {
            let mut health = BodyHealth::new(Health::new(5));
            health.damage(damage);
            world
                .spawn((health, BodyWallet { balance }, DrivingParticipant(PlayerSlot(slot))))
                .id()
        };
        let alice = body(app.world_mut(), 0, 2, 7);
        body(app.world_mut(), 1, 0, 0);
        app.world_mut().insert_resource(ControlledSubject(Some(alice)));
        let view = |world: &mut World, seat: Option<u8>| {
            let id = LocalViewId(seat.map_or(9, |seat| seat));
            let mut view = world.spawn((
                LocalView,
                id,
                ResolvedViewSubject::default(),
                ViewHudFacts::default(),
            ));
            if let Some(seat) = seat {
                view.insert(ViewParticipant(PlayerSlot(seat)));
            }
            view.id()
        };
        let views = [
            view(app.world_mut(), Some(0)),
            view(app.world_mut(), Some(1)),
            view(app.world_mut(), None),
            view(app.world_mut(), Some(2)),
        ];
        app.update();
        let shown: Vec<_> = views
            .iter()
            .map(|view| {
                let facts = app.world().get::<ViewHudFacts>(*view).expect("a view's HUD facts").0;
                (facts.present, facts.hp_current, facts.balance)
            })
            .collect();
        assert_eq!(
            shown,
            vec![(true, 3, 7), (true, 5, 0), (true, 3, 7), (false, 0, 0)],
            "(present, health, money) of the views that follow seat 0, seat 1, \
             nothing, and seat 2 (no body); the controlled body is seat 0's"
        );
    }

    /// Q150 on a merged screen: Alice and Bob in one live room share the one
    /// view, and Bob's meters are on it beside Alice's. The controls: Cid,
    /// whose seat has a view of its own, is on his view only; and Dan, in
    /// another live room with no view yet, is not on Alice's screen.
    #[test]
    fn a_shared_view_shows_each_seat_on_it() {
        use crate::local_view::{
            resolve_view_subjects, LocalView, LocalViewId, ResolvedViewSubject, ViewParticipant,
        };
        use ambition_characters::actor::Health;
        use ambition_characters::control::{DrivingParticipant, PlayerSlot};
        let mut app = App::new();
        app.add_systems(Update, (resolve_view_subjects, rebuild_view_hud_facts).chain());
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let here = LiveRoomInstance::ACTIVATION;
        let body = |world: &mut World, slot: u8, balance: i32, room: LiveRoomInstance| {
            world
                .spawn((
                    BodyHealth::new(Health::new(5)),
                    BodyWallet { balance },
                    DrivingParticipant(PlayerSlot(slot)),
                    InRoomInstance(room),
                ))
                .id()
        };
        let alice = body(app.world_mut(), 0, 7, here);
        body(app.world_mut(), 1, 11, here);
        body(app.world_mut(), 2, 13, here);
        body(app.world_mut(), 3, 17, here.next());
        app.world_mut().insert_resource(ControlledSubject(Some(alice)));
        let shared = app
            .world_mut()
            .spawn((
                LocalView,
                LocalViewId::FIRST,
                ResolvedViewSubject::default(),
                ViewHudFacts::default(),
                SharedViewHudFacts::default(),
            ))
            .id();
        let cids = app
            .world_mut()
            .spawn((
                LocalView,
                LocalViewId(1),
                ViewParticipant(PlayerSlot(2)),
                ResolvedViewSubject::default(),
                ViewHudFacts::default(),
                SharedViewHudFacts::default(),
            ))
            .id();
        app.update();
        let shown = |view: Entity| {
            let world = app.world();
            let own = world.get::<ViewHudFacts>(view).expect("HUD facts").0.balance;
            let others: Vec<(u8, i32)> = world
                .get::<SharedViewHudFacts>(view)
                .expect("shared HUD facts")
                .0
                .iter()
                .map(|(slot, facts)| (slot.0, facts.balance))
                .collect();
            (own, others)
        };
        assert_eq!(
            (shown(shared), shown(cids)),
            ((7, vec![(1, 11)]), (13, vec![])),
            "((the shared view's own purse, the other seats on it), (Cid's view's)) by (seat, purse)"
        );
    }

    #[test]
    fn hud_facts_track_the_controlled_body() {
        use ambition_characters::actor::Health;
        let mut app = App::new();
        app.init_resource::<PlayerHudFacts>();
        app.add_systems(Update, rebuild_player_hud_facts);

        // Home avatar with a fat purse; a driven actor with its own economy.
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyHealth::new(Health::new(20)),
            ActorResources::declared(&[ambition_abilities::mana::POOL])
                .expect("valid")
                .expect("declared"),
            BodyWallet { balance: 42 },
        ));
        let mut actor_hp = BodyHealth::new(Health::new(10));
        actor_hp.damage(7);
        let actor = app
            .world_mut()
            .spawn((actor_hp, BodyWallet { balance: 7 }))
            .id();
        app.world_mut()
            .insert_resource(ControlledSubject(Some(actor)));
        app.update();

        let facts = *app.world().resource::<PlayerHudFacts>();
        assert!(facts.present);
        assert_eq!(
            (facts.hp_current, facts.hp_max),
            (3, 10),
            "HUD facts must snapshot the POSSESSED body's health"
        );
        assert_eq!(facts.balance, 7, "money is a body stat");
        assert_eq!(
            facts.mana, None,
            "the possessed body holds no Mana, so the HUD must not show the \
             home avatar's pool as if it were this body's"
        );
    }

    #[test]
    fn shrine_pulse_ticks_down_sim_side() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime::new(0.1, 0.1));
        app.insert_resource(
            ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse { remaining: 0.25 },
        );
        app.add_systems(Update, tick_shrine_activation_pulse);
        app.update();
        let remaining = app
            .world()
            .resource::<ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse>()
            .remaining;
        assert!((remaining - 0.15).abs() < 1e-6);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse>()
                .remaining,
            0.0,
            "pulse clamps at zero"
        );
    }
}

#[cfg(test)]
mod held_item_view_tests {
    use super::*;
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    fn holder(app: &mut App, sim: &str, x: f32, item: &str, slot: u8) {
        let spec = ambition_characters::brain::HeldItemSpec {
            id: item.to_owned(),
            melee: None,
            ranged: None,
            use_behavior: Default::default(),
        };
        app.world_mut().spawn((
            BodyKinematics {
                pos: ae::Vec2::new(x, 0.0),
                size: ae::Vec2::splat(32.0),
                ..Default::default()
            },
            ambition_platformer2d_actor_monolith::features::HeldItem::new(spec),
            ActorControl::default(),
            SimId::placement(sim),
            DrivingParticipant(PlayerSlot(slot)),
        ));
    }

    fn undriven_holder(
        app: &mut App,
        sim: &str,
        item: &str,
        standing: Option<(ambition_combat::components::ActorDisposition, bool)>,
    ) {
        let spec = ambition_characters::brain::HeldItemSpec {
            id: item.to_owned(),
            melee: None,
            ranged: None,
            use_behavior: Default::default(),
        };
        let mut body = app.world_mut().spawn((
            BodyKinematics {
                size: ae::Vec2::splat(32.0),
                ..Default::default()
            },
            ambition_platformer2d_actor_monolith::features::HeldItem::new(spec),
            ActorControl::default(),
            SimId::placement(sim),
        ));
        if let Some((disposition, alive)) = standing {
            let mut health = BodyHealth::new(ambition_characters::actor::Health::new(10));
            if !alive {
                health.damage(10);
            }
            body.insert((disposition, health));
        }
    }

    /// EVERY DRIVEN HOLDER DRAWS, NOT JUST THE ONE YOU ARE LOOKING THROUGH.
    ///
    /// ⛔⛔ THIS VIEW WAS AN `Option`, filled from `ControlledSubject`, so exactly
    /// one held item existed in the whole world: a couch match drew seat zero's
    /// weapon and nothing else, and a CPU-versus-CPU match drew neither Admiral's
    /// gun-sword. The argument against that was already written for a sibling view
    /// in this file — *"a couch-versus match has two driven bodies and neither is
    /// more protected than the other, because a rule that privileges one
    /// participant stops being a rule about bodies"*.
    ///
    /// ⛔ THE ORDER IS ASSERTED, not incidental. This is a `Vec` built from an
    /// unordered query and the renderer spawns one visual per row, so an unsorted
    /// build would swap two items' draw order between runs of the same match.
    #[test]
    fn two_driven_holders_publish_two_facts_in_sim_id_order() {
        let mut app = App::new();
        app.init_resource::<HeldItemView>()
            .add_systems(Update, rebuild_held_item_view);
        // Spawned in the REVERSE of the expected order, so a pass-through of
        // query order cannot agree with the assertion by luck.
        holder(&mut app, "seat_two", 200.0, "gun_sword", 1);
        holder(&mut app, "seat_one", 100.0, "axe", 0);
        app.update();

        let view = app.world().resource::<HeldItemView>();
        assert_eq!(
            view.0
                .iter()
                .map(|f| f.item_id.as_str())
                .collect::<Vec<_>>(),
            vec!["axe", "gun_sword"],
            "both driven holders publish, ordered by `SimId` — one fact means the \
             view is still singular"
        );
    }

    /// ⛔⛔ A CPU FIGHTER'S WEAPON IS DRAWN, and the previous version of this test
    /// asserted the opposite.
    ///
    /// It read *"an autonomous holder drawing its item is a separate question
    /// about NPC presentation"* — but the Pirate Admiral's side-B draws
    /// `admiral_gun_sword`, an id registered in the IN-HAND art manifest and in no
    /// other, so in a CPU-versus-CPU match neither Admiral's gun-sword was drawn
    /// by any road at all. `DrivingParticipant` was never the question a renderer
    /// was asking; custody is.
    #[test]
    fn a_holder_nobody_drives_still_publishes() {
        let mut app = App::new();
        app.init_resource::<HeldItemView>()
            .add_systems(Update, rebuild_held_item_view);
        undriven_holder(&mut app, "cpu_admiral", "admiral_gun_sword", None);
        app.update();
        assert_eq!(
            app.world()
                .resource::<HeldItemView>()
                .0
                .iter()
                .map(|f| f.item_id.as_str())
                .collect::<Vec<_>>(),
            vec!["admiral_gun_sword"],
        );
    }

    /// ⛔ AND THE PARTITION HOLDS AT ITS OWN EDGE: a LIVING HOSTILE holder is the
    /// over-hand road's, so the in-hand road must not also claim it.
    ///
    /// Without this the widening above would draw a hostile pirate's `gun_sword`
    /// TWICE — once in the hand from `HeldItemArtManifest` and once over it from
    /// `WieldedItemVisualCatalog`, both of which register that id.
    #[test]
    fn a_living_hostile_holder_belongs_to_the_other_road() {
        let mut app = App::new();
        app.init_resource::<HeldItemView>()
            .add_systems(Update, rebuild_held_item_view);
        undriven_holder(
            &mut app,
            "hostile_pirate",
            "gun_sword",
            Some((ambition_combat::components::ActorDisposition::Hostile, true)),
        );
        app.update();
        assert!(
            app.world().resource::<HeldItemView>().0.is_empty(),
            "a living hostile wielder is drawn over the hand, so publishing it \
             here too would draw its gun-sword twice"
        );
    }

    /// ⛔ AND A DEAD ONE COMES BACK, because the over-hand road drops it. Nobody
    /// may be on NEITHER road — that is how the Admiral vanished in the first
    /// place, and a corpse is the arm where the two conditions disagree.
    #[test]
    fn a_dead_hostile_holder_falls_back_to_the_in_hand_road() {
        let mut app = App::new();
        app.init_resource::<HeldItemView>()
            .add_systems(Update, rebuild_held_item_view);
        undriven_holder(
            &mut app,
            "fallen_pirate",
            "gun_sword",
            Some((
                ambition_combat::components::ActorDisposition::Hostile,
                false,
            )),
        );
        app.update();
        assert_eq!(app.world().resource::<HeldItemView>().0.len(), 1);
    }

    /// A PEACEFUL holder is nobody's over-hand business either.
    #[test]
    fn a_peaceful_holder_is_drawn_in_the_hand() {
        assert!(drawn_in_the_hand(
            Some(ambition_combat::components::ActorDisposition::Peaceful),
            Some(true)
        ));
        assert!(drawn_in_the_hand(None, Some(true)));
        assert!(!drawn_in_the_hand(
            Some(ambition_combat::components::ActorDisposition::Hostile),
            Some(true)
        ));
    }
}

#[cfg(all(test, feature = "input"))]
mod blink_preview_room_tests {
    use super::*;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};

    /// The reticle of a subject aiming a blink 100 px right from x=100, in
    /// live room `room`. #0 has a wall at x 150..170 and #1 has none.
    fn reticle(room: usize) -> BlinkPreviewFact {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<ambition_platformer2d_shared_tangle::schedule::GameMode>();
        app.init_resource::<BlinkPreviewFact>();
        let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        let wall = ae::Block::solid("wall", ae::Vec2::new(150.0, 0.0), ae::Vec2::new(20.0, 400.0));
        for (instance, blocks) in live.into_iter().zip([vec![wall], Vec::new()]) {
            app.world_mut().spawn((
                RoomInstanceRoot,
                instance,
                ae::RoomGeometry(ae::World::new("blink", ae::Vec2::new(400.0, 400.0), ae::Vec2::ZERO, blocks)),
            ));
        }
        let mut facts = ae::BodyMotionFacts::default();
        facts.blink_aiming = true;
        facts.blink_aim_offset = ae::Vec2::new(100.0, 0.0);
        let subject = app
            .world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::new(100.0, 200.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(16.0, 32.0),
                    facing: 1.0,
                },
                ae::BodyAbilities::new(ae::AbilitySet::sandbox_all()),
                facts,
                InRoomInstance(live[room]),
            ))
            .id();
        app.insert_resource(ControlledSubject(Some(subject)));
        app.add_systems(Update, rebuild_blink_preview_fact);
        app.update();
        *app.world().resource::<BlinkPreviewFact>()
    }

    /// OW1: the blink reticle resolves against the walls of the subject's
    /// own live room. In #1 (no wall) it reaches past x=170; in #0 the wall
    /// stops it before x=150. Before, the preview read the sole live room,
    /// so while two rooms were live no reticle showed.
    #[test]
    fn the_blink_reticle_reads_the_walls_of_its_subjects_own_room() {
        let open = reticle(1);
        assert!(open.active && open.target.x > 170.0, "the reticle in #1: {open:?}");
        assert_eq!(open.room, Some(LiveRoomInstance::ACTIVATION.next()), "the reticle names its subject's room");
        let walled = reticle(0);
        assert!(walled.active && walled.target.x < 150.0, "the reticle in #0: {walled:?}");
    }
}

#[cfg(test)]
mod mark_beacon_room_tests {
    use super::*;

    /// A mark is drawn in the live room it was dropped in, not in the room its
    /// player stands in now. The player crossed from #1 into #2 after
    /// dropping the mark. The control: a mark with no room is drawn in its
    /// player's room. Poison: read the player's room and the mark is drawn in
    /// #2.
    #[test]
    fn a_mark_is_drawn_in_the_live_room_it_was_dropped_in() {
        use ambition_abilities::traversal::mark_recall::PlayerMark;
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let first = LiveRoomInstance::ACTIVATION.next();
        let second = first.next();
        let mut app = App::new();
        app.init_resource::<MarkBeaconsView>();
        app.add_systems(Update, rebuild_mark_beacons_view);
        let pos = ae::Vec2::new(10.0, 20.0);
        app.world_mut().spawn((PlayerMark { pos: Some(pos), room: Some(first) }, InRoomInstance(second)));
        app.world_mut().spawn((PlayerMark { pos: Some(pos), room: None }, InRoomInstance(second)));
        app.update();
        let mut rows: Vec<_> = app.world().resource::<MarkBeaconsView>().0.iter().map(|(_, room)| *room).collect();
        rows.sort();
        assert_eq!(rows, vec![Some(first), Some(second)], "(the dropped room, the unroomed mark's player's room)");
    }
}
