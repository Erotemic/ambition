//! Hit event application for ECS-owned feature entities.
//!
//! Drains [`HitEvent`] messages and applies them to actors (peaceful +
//! hostile), bosses, and breakables — including the side effects
//! (banners, VFX, SFX, debris, gameplay effects, hit-stop) those hits
//! produce. Pogo-orb resolution lives in this same drain loop and
//! branches on `HitSource::Pogo` to do orb-AABB matching rather
//! than broadcast volume overlap. Read-only `ecs_hit_event_hits_*`
//! predicates live here too so the attack / projectile systems can
//! pre-check whether a queued hit will actually land before kicking off
//! cues.

use bevy::ecs::system::SystemParam;
use bevy::prelude::{
    Commands, Entity, MessageReader, MessageWriter, Query, Res, ResMut, With, Without,
};

use super::damage_drops::drop_currency_coin;
use super::{
    sync_actor_components_from_cluster, ActorDisposition, ActorIdentity, BodyCombat,
    BreakableFeature, CenteredAabb, FeatureId, FeatureName, FeatureSimEntity,
};
use ambition_combat::events::{GameplayBanner, HitEvent, HitSource, SetFlagRequested};
use ambition_combat::util::{approximately_same_aabb, midpoint};
// Only the exploding-mite blast test pins this drop tuning constant; the drop
// tests query `PickupFeature` directly. Both are test-only now that the drop
// spawners live in `damage_drops`.
#[cfg(test)]
use super::damage_drops::EXPLODER_BLAST_DAMAGE;
use super::damage_predicates::target_is_ignored;
#[cfg(test)]
use super::PickupFeature;
use ambition_combat::events::ActorStimulus;
use ambition_sfx::SfxWriter;
use ambition_vfx::vfx::DebrisBurstMessage;
use ambition_vfx::vfx::VfxMessage;

/// One side of a combat relationship, as this module reads it off a body.
type CombatSide<'w> = (
    &'w ambition_combat::components::ActorFaction,
    Option<&'w ambition_characters::control::DrivingParticipant>,
    Option<&'w ambition_combat::targeting::MatchTeam>,
);

/// May this attacker's hit damage this boss?
///
/// Named and free-standing so the policy can be stated and tested rather than
/// inferred from the shape of a closure — and so it is visibly the SAME
/// `damage_lands_between` the body resolver applies to every other victim.
///
/// Allegiance is EFFECTIVE, not authored: a possessed boss fights as its
/// driver's side. An UNATTRIBUTED hit lands — a broadcast with no attacker
/// cannot be adjudicated, and refusing it would disarm the hazard and scripted
/// blast paths that legitimately carry no entity.
pub(crate) fn boss_damage_allowed(
    attacker: Option<CombatSide<'_>>,
    boss: Option<CombatSide<'_>>,
    friendly_fire: ambition_combat::targeting::FriendlyFire,
    boss_entity: Entity,
) -> bool {
    let (
        Some((attacker_faction, attacker_driver, attacker_team)),
        Some((boss_faction, boss_driver, boss_team)),
    ) = (attacker, boss)
    else {
        return true;
    };
    ambition_combat::targeting::damage_lands_between(
        ambition_combat::targeting::effective_faction(*attacker_faction, attacker_driver),
        ambition_combat::targeting::effective_faction(*boss_faction, boss_driver),
        attacker_team,
        boss_team,
        friendly_fire,
        None,
        boss_entity,
    )
}

#[derive(SystemParam)]
pub struct FeatureHitWriters<'w, 's> {
    pub set_flag: MessageWriter<'w, SetFlagRequested>,
    pub actor_stimuli: MessageWriter<'w, ActorStimulus>,
    pub sfx: SfxWriter<'w>,
    pub vfx: MessageWriter<'w, VfxMessage>,
    pub debris: MessageWriter<'w, DebrisBurstMessage>,
    pub wallet_shield_spent: MessageWriter<'w, ambition_damage::WalletShieldSpent>,
    /// S4: KOs of bodies a RULESET owns, for the stocks loop. Written from the
    /// `RulesetOwnsDeath` arm, which is where the engine already stops and hands
    /// the consequence over.
    pub knockouts: MessageWriter<'w, ambition_combat::stocks::BodyKnockedOut>,
    /// The hit's RESULT, for the simulation — the match freeze reads it.
    /// `Option` for the reason spelled out on the player-side twin.
    pub resolved: Option<MessageWriter<'w, ambition_combat::hitbox::ResolvedBodyHit>>,
    /// The resolver's DECISION about each hit, for the causal inspector.
    /// `Option` for the reason spelled out on the player-side twin: this is read
    /// by an instrument and by nothing else, so a composition that never
    /// registers it publishes nothing rather than panicking.
    #[cfg(feature = "causal")]
    pub resolutions: Option<MessageWriter<'w, ambition_damage::BodyHitResolved>>,
    /// The LAUNCH the reaction produced. Same `Option` rule as above.
    #[cfg(feature = "causal")]
    pub reactions: Option<MessageWriter<'w, ambition_damage::BodyReactionApplied>>,
    /// Refactor 3: spawning loot/respawns on a hit is a one-liner
    /// (`writers.commands.spawn(...)`) instead of hand-threading a separate
    /// `&mut Commands` through every helper that already takes `writers`.
    pub commands: Commands<'w, 's>,
    /// Captured gameplay-session owner for loot, minions, and death effects.
    pub active_session:
        Option<Res<'w, ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    /// Whose cues each body emits (A13). Read-only, looked up by entity.
    ///
    /// Bundled here rather than added to five helper signatures: every hit-feedback
    /// caller already takes `writers`, and hit feedback is exactly where attribution
    /// matters most — an authored strike sound belongs to the ATTACKER's bank and
    /// the hurt fallback to the VICTIM's, so the emitter needs both.
    pub body_sources: Query<'w, 's, &'static ambition_sfx::BodyPresentationSource>,
    /// Whose death a drop fell out of. Read-only, looked up by entity, and
    /// bundled here for the same reason `body_sources` is: all three drop sites
    /// (actor, boss, breakable) already take `writers`, and a coin, a heart and
    /// an ability pickup each have to state their parent's identity or no render
    /// family will claim them — see `damage_drops::dynamic_drop_origin`.
    pub identities: Query<'w, 's, &'static ambition_platformer2d_shared_tangle::sim_id::SimId>,
}

impl FeatureHitWriters<'_, '_> {
    /// The presentation source for one body, if it has one.
    ///
    /// Returns an OWNED id rather than a reference: callers need it alongside `&mut writers.sfx`,
    /// and a borrow of `self` cannot coexist with that.
    pub fn source_of(
        &self,
        entity: Option<bevy::prelude::Entity>,
    ) -> Option<ambition_sfx::PresentationSourceId> {
        self.body_sources
            .get(entity?)
            .ok()
            .map(|source| source.id().clone())
    }

    /// The simulation identity of one body or prop, if it has one.
    ///
    /// Owned for the same reason [`Self::source_of`] is owned: the drop sites
    /// need it alongside `&mut writers.commands`.
    ///
    /// read, never spelled. `SimId::placement(feature_id)` would reproduce
    /// today's value for every drop parent in the shipped game, and that is
    /// exactly the shortcut `SimId::as_str`'s doc forbids — provenance is a
    /// component the entity carries so that changing the id grammar cannot
    /// silently change what reconstruction believes. A summoned body's identity
    /// is not in the `placement:` namespace at all.
    pub fn identity_of(
        &self,
        entity: bevy::prelude::Entity,
    ) -> Option<ambition_platformer2d_shared_tangle::sim_id::SimId> {
        self.identities.get(entity).ok().cloned()
    }
}

impl FeatureHitWriters<'_, '_> {
    pub fn session_spawn_scope(
        &self,
    ) -> ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope {
        self.active_session.as_deref().map_or(
            ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
            ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::spawn_scope,
        )
    }
}

/// Resolve the identity a drop will descend from, or refuse the drop loudly.
///
/// a drop that cannot name its parent must not spawn — the same rule `apply_summon_effects`
/// applies to a summon, and here it is not even a trade. `rebuild_dynamic_feature_views` selects
/// loot by provenance, so an unprovenanced coin is a coin NO render family claims: the player walks
/// over `draw_unclaimed_feature_views`' magenta diagnostic box, and the room-transition cover —
/// which holds the screen until no stand-in remains — sits out its whole 8-second deadline over it.
///
/// Empirically this never fires in the shipped game: every drop parent is
/// built by the construction executor, which stamps `SimId` before the recipe
/// runs (probed across `proving_grounds`' whole cast).
/// WHEN a hit landed, and WHICH RUN OF THE WORLD it landed in — the two facts a
/// rollback-safe bark draw needs.
///
/// ⛔ A `SystemParam` RATHER THAN TWO `Res`, and not for tidiness:
/// `apply_feature_hit_events` sits at Bevy's parameter ceiling, and adding two
/// bare resources put it over — the error names `chain` and a tuple's trait
/// bounds, several files away from the cause.
///
/// Both halves are optional because a bare fixture has neither. A world with no
/// clock cannot draw, and answers `true`: every hit speaks, which is what every
/// body did before the rate existed.
#[derive(SystemParam)]
pub struct BarkDraw<'w> {
    tick: Option<Res<'w, ambition_time::SimTick>>,
    active: Option<Res<'w, crate::character_runtime::ActiveMatch>>,
    feel_tuning: Option<Res<'w, ambition_combat::feel::Platformer2dFeelTuningMonolith>>,
    combat_rules: Option<Res<'w, ambition_combat::rules::ResolvedCombatTuning>>,
}

impl BarkDraw<'_> {
    /// The world's feel tuning, or the default for a composition without it.
    pub fn feel(&self) -> ambition_combat::feel::Platformer2dFeelTuningMonolith {
        self.feel_tuning.as_deref().copied().unwrap_or_default()
    }

    /// The match's resolved rules, or the baseline.
    pub fn rules(&self) -> ambition_combat::rules::ResolvedCombatTuning {
        self.combat_rules.as_deref().cloned().unwrap_or_default()
    }

    /// May the hit on `victim` speak, under `rules`?
    /// ⛔ THE VICTIM ARRIVES AS ITS SIMULATION NAME, not as an entity. Resolving
    /// it here would mean another query on this already-broad gateway; the
    /// caller that resolves hits holds `identities` for its own reasons.
    pub fn allows(
        &self,
        rules: &ambition_combat::rules::ResolvedCombatTuning,
        victim: Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    ) -> bool {
        bark_is_allowed(
            Some(rules),
            self.tick.as_deref(),
            self.active.as_deref(),
            // ⛔⛔ THE VICTIM'S SIMULATION NAME, NOT ITS ENTITY BITS. See
            // `bark_is_allowed`. `None` for a body with no canonical identity —
            // a bare fixture — which the salt below answers explicitly.
            victim,
        )
    }
}

/// May this hit make the victim SAY something?
///
/// ⭐⭐ A RATE, NOT A COOLDOWN. Jon, 2026-08-24: *"not have barks happen every
/// time a character is hit. Make it a more rare event. Not never, but I'd like
/// it to happen less often."* A cooldown makes the FIRST hit of every exchange
/// bark and the rest silent, which is a rhythm a player learns; a rate keeps
/// them unpredictable, which is what "rare" sounds like.
///
/// ⛔ `sim_random`, NEVER A STREAM. This is read inside the rollback window, so
/// a resimulated hit has to reach the same answer or the bubble flickers on
/// every rewind — and a stream would need rewinding itself.
///
/// ⛔ AND EVERY AXIS IS LOAD-BEARING: the victim is the SALT so two fighters
/// struck on one tick decide independently rather than chorusing; the match is
/// the CONTEXT so match two does not replay match one's barks; the tick is when.
///
/// A world that declares no rate, or has no clock to draw against, barks on
/// every hit — which is what every body did before this existed.
pub(crate) fn bark_is_allowed(
    rules: Option<&ambition_combat::rules::ResolvedCombatTuning>,
    tick: Option<&ambition_time::SimTick>,
    active: Option<&crate::character_runtime::ActiveMatch>,
    victim: Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
) -> bool {
    let chance = rules.map_or(1.0, |rules| rules.bark_chance);
    if chance >= 1.0 {
        return true;
    }
    if chance <= 0.0 {
        return false;
    }
    let Some(tick) = tick else {
        return true;
    };
    let draw = ambition_platformer2d_core::sim_random::sim_random(
        ambition_platformer2d_core::sim_random::DOMAIN_BARK,
        active.map_or(
            ambition_platformer2d_core::sim_random::CONTEXT_UNSEEDED,
            |active| active.instance().random_context(),
        ),
        tick.get(),
        // ⛔⛔ THE SIMULATION NAME, NEVER `Entity::to_bits()`. An entity index is
        // ALLOCATOR HISTORY: two peers that spawned the same cast in a different
        // order hold different bits for one fighter, so a draw salted with them
        // agrees locally and disagrees across the wire. Rollback hides it — a
        // rewind reuses the same ids — which is why it survives every test that
        // is not a netplay test.
        //
        // ⭐ A BODY WITH NO CANONICAL NAME DRAWS FROM ZERO, deliberately: a bare
        // fixture is not in a networked match, and inventing an allocator-derived
        // fallback here would put the same non-determinism back under a longer
        // name.
        victim.map_or(0, |id| {
            ambition_platformer2d_core::sim_random::sim_salt_for_name(id.as_str())
        }),
    );
    // The draw is uniform over the whole `u64`, so a fraction of it is the
    // fraction of hits that speak.
    (draw >> 11) as f64 * (1.0 / (1u64 << 53) as f64) < f64::from(chance)
}

fn drop_parent(
    writers: &FeatureHitWriters<'_, '_>,
    entity: Entity,
    what: &str,
    id: &str,
) -> Option<ambition_platformer2d_shared_tangle::sim_id::SimId> {
    let parent = writers.identity_of(entity);
    if parent.is_none() {
        bevy::log::warn!(
            target: "ambition_platformer2d::damage",
            "{what} `{id}` died with no simulation identity, so its drop is skipped: \
             a pickup that cannot state the body it fell out of is drawn by no \
             render family and reaches the player as a magenta stand-in",
        );
    }
    parent
}

#[derive(SystemParam)]
pub struct FeatureHitCatalogs<'w> {
    pub characters: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    /// Provider-authored sheets (U1 stage B): a split offspring sizes its body
    /// from its sheet like anything else, so the damage path carries it beside
    /// the catalog it already carries.
    pub sheets: Res<'w, ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    pub bosses: Res<'w, ambition_boss_encounter::BossCatalog>,
    /// AD8: the prepared cast, so a struck or provoked character speaks in its
    /// OWN voice rather than the engine's. `Option` because a bare engine App
    /// legitimately has no prepared cast — the same shape the ambient ticker
    /// already uses.
    pub prepared: Option<Res<'w, crate::character_runtime::PreparedCharacterRegistry>>,
}

/// Coins a defeated standard enemy drops. A flat amount — a *working* earn-side
/// (kill -> coin -> wallet -> merchant), not a balanced economy.
pub const ENEMY_BOUNTY: i32 = 5;

/// Coins a smashed crate/pot drops — smaller than an enemy kill, so combat still
/// pays best, but the environment is worth poking.
pub const BREAKABLE_BOUNTY: i32 = 2;

/// Coins a defeated boss drops, beyond its ability reward — a jackpot for the
/// hardest fights (one boss kill ~= ten standard enemies).
pub const BOSS_BOUNTY: i32 = 50;

/// Health a dropped heart restores when the enemy drops one.
pub const ENEMY_HEALTH_DROP: i32 = 1;

/// Apply typed slash / projectile / pogo hit messages to ECS feature targets.
pub fn apply_feature_hit_events(
    mut hit_events: MessageReader<HitEvent>,
    // Every victim's PUBLISHED silhouette, looked up by entity.
    //
    // A separate read-only query rather than another member of the actor tuple:
    // that tuple is already nesting `Option<(..)>` groups to stay inside Bevy's
    // arity ceiling, and this is read, never written, so it may overlap freely.
    //
    // Why it exists: this is the path the PLAYER's own attacks take to reach an
    // enemy, and it tested the enemy's coarse `CenteredAabb`. So the authored
    // hurtbox work reached neither direction of combat — an enemy could not be hit
    // on its authored silhouette by the player, and the player could not be hit on
    // its own. `strike_reaches_victim` is the single shared rule, so the two paths
    // cannot drift again.
    // BUNDLED, and the bundle is the point: this system is AT Bevy's system-param
    // ceiling, so a second by-entity victim read rides with the first rather
    // than becoming a seventeenth parameter that does not compile.
    //
    // `victim_volumes` answers WHAT geometry a strike must reach;
    // `victim_motion` answers whether the body it reached is inside an evade —
    // one term of the shared eligibility gate, which the actor cluster query
    // has no column left to carry.
    (victim_volumes, victim_motion): (
        Query<&ambition_combat::components::DamageableVolumes>,
        Query<&ambition_platformer2d_core::BodyMotionFacts>,
    ),
    mut banner: ResMut<GameplayBanner>,
    combat_banter: Option<Res<crate::features::banter::CombatBanterRegistry>>,
    // Knockback feel for struck actors (§A2 step 6). `Option` so minimal
    // headless test worlds that don't stand up the tuning resource still run
    // (they get the default feel).

    // AE6: the resolved match rules. `di_max_angle` is a rule of the match
    // being played, not world tuning, so it is folded into the local `feel`
    // below rather than written into the world's resource by a route.

    // ⭐ WHEN, AND WHICH RUN OF THE WORLD — bundled, because this system is at
    // Bevy's parameter ceiling and two more bare `Res` put it over. See
    // [`BarkDraw`].
    bark_draw: BarkDraw<'_>,
    // Authored character voice for struck NPCs. This resource is required:
    // a production App that omitted provider catalog composition is malformed,
    // and must not silently degrade to anonymous barks.
    catalogs: FeatureHitCatalogs,
    mut breakables: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &CenteredAabb,
            &mut BreakableFeature,
        ),
        With<FeatureSimEntity>,
    >,
    mut actors: Query<
        (
            Entity,
            &FeatureId,
            &CenteredAabb,
            &mut ActorIdentity,
            &ActorDisposition,
            &mut BodyCombat,
            // Provoke accumulator (shared aggression component). `Option` so
            // minimal test fixtures that spawn a bare actor without it still
            // match; production actors always carry it.
            Option<&mut ambition_combat::components::ActorAggression>,
            // Dialogue payload — present on talkable actors (drives barks).
            Option<&ambition_combat::components::ActorInteraction>,
            // The actor's held locomotion, for directional influence (CM2). The
            // brain writes it every tick; DI reads the SAME field, so a level-9
            // CPU or RL policy DIs its own knockback like a human. `Option` for
            // bare test bodies; inert unless `feel.di_max_angle` is authored.
            Option<&ambition_characters::control::ActorControl>,
            // The body's explicit movement policy — required (absence is never
            // a policy). The crawler's typed cling-break detach goes through it.
            &'static mut ambition_platformer2d_core::movement::MotionModel,
            Option<(
                &'static mut ambition_characters::actor::BodyWallet,
                &'static ambition_characters::actor::BodyWalletShield,
            )>,
            super::actor_clusters::ActorClusterQueryData,
            // CM8: this body's own hurt reaction (its `CombatTuning.hurt_feedback`).
            // `Option` for bare test fixtures spawned without the combat carrier
            // (they fall back to the ENEMY default). The victim owns its spray;
            // the attack owns only the strike sound.
            // Bundled: this query is AT Bevy's 16-column ceiling, so the
            // combat-tuning read rides with the death-ownership flag.
            //
            // `RulesetOwnsDeath`: does a RULESET own this body's death? A match
            // fighter's KO is the match's business; the world's death economy is
            // not invited.
            //
            // They correlate for a fighter mid-round and diverge the moment it is eliminated — the
            // body stays standing, its death still belongs to the match, and it is not fighting any
            // more.
            (
                Option<&'static ambition_combat::CombatTuning>,
                bevy::prelude::Has<ambition_combat::components::RulesetOwnsDeath>,
                bevy::prelude::Has<ambition_combat::components::ActiveCombatant>,
                // ⛔ AND WHETHER THE WORLD HAS ITS HANDS OFF IT. Dropping
                // `ActiveCombatant` does NOT make a body undamageable here:
                // `CombatStanding::of` calls a `Hostile` disposition damageable
                // either way, and prepared match construction makes every
                // fighter Hostile. So a fighter waiting out its death beat took
                // ordinary damage and carried the percent into its next stock.
                bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
            ),
        ),
        // Bosses are handled by the disjoint `bosses` query; both take
        // `&mut BodyKinematics` (the unified component), so exclude bosses
        // here to keep the two queries provably non-aliasing. `Without<PlayerEntity>`
        // keeps this `&mut BodyCombat` actor query disjoint from the player
        // `&mut BodyCombat` query below, now that both share the unified component.
        (
            With<FeatureSimEntity>,
            Without<ambition_boss_encounter::BossConfig>,
            Without<crate::actor::PlayerEntity>,
        ),
    >,
    mut bosses: Query<
        (
            Entity,
            &FeatureId,
            &CenteredAabb,
            ambition_boss_encounter::BossClusterQueryData,
            // The boss's shared body components (§A1): HP authority + the
            // hit-flash the damage path arms. `Without<PlayerEntity>` keeps
            // this `&mut BodyCombat` provably disjoint from the player query
            // below (the actor query is already `Without<BossConfig>`).
            &mut ambition_characters::actor::BodyHealth,
            &mut ambition_characters::actor::BodyCombat,
            Option<(
                &mut ambition_characters::actor::BodyWallet,
                &ambition_characters::actor::BodyWalletShield,
            )>,
            &ambition_characters::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
            // CM8: the boss's own hurt reaction (ENEMY default).
            Option<&ambition_combat::CombatTuning>,
            // The world's hands are off it — the same gate the actor road takes.
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
        ),
        (With<FeatureSimEntity>, Without<crate::actor::PlayerEntity>),
    >,
    // Iterates every player and uses `HitEvent::attacker` (now stamped by every player-attacker
    // emit site — slash, pogo, and player projectile). Events with `attacker = None` remain
    // unattributed unless the source is one of the legacy player-originated variants whose producer
    // predates explicit attacker stamping. Enemy/environmental hits never borrow the primary
    // player's identity.
    mut player_combat_q: Query<
        (
            bevy::prelude::Entity,
            &mut ambition_characters::actor::BodyCombat,
            // The attacker's live swing, so a multi-active-frame slash records
            // which targets it has already struck and never double-hits them.
            Option<&mut crate::actor::BodyMelee>,
        ),
        bevy::prelude::With<crate::actor::PlayerEntity>,
    >,
    primary_q: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<crate::actor::PlayerEntity>,
            bevy::prelude::With<crate::actor::PrimaryPlayer>,
        ),
    >,
    mut writers: FeatureHitWriters,
    mut attacker_moves: Query<&mut ambition_combat::moveset::MovePlayback>,
    // The player's OUTGOING damage scale (the power slider). `Option` so minimal
    // headless test worlds that never stand up settings still run at the neutral
    // 1.0 — the same shape as `feel_tuning`. Read in a sim system exactly as the
    // projectile spawn does; it is a menu-side (non-rollback) setting, constant
    // across a rollback window, so reading it here is deterministic.
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    // Which bodies hit HEAVY. A filter-only query: it reads no components, so
    // it conflicts with nothing here, including the mutable boss query above.
    // Two questions about the ATTACKER, both filter-only so they read no
    // components and conflict with nothing here, including the mutable boss
    // query above. Bundled into one param because this system is at Bevy's
    // 16-param ceiling.
    (heavy_attackers, controlled_attackers, combat_sides): (
        Query<(), With<ambition_boss_encounter::BossConfig>>,
        Query<(), With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>>,
        // Whose side each body is on, read for the boss scan's relationship
        // check. Read-only and looked up by entity, so it may overlap the
        // mutable actor and boss queries freely.
        //
        // `DrivingParticipant` rides along because allegiance is EFFECTIVE, not
        // authored: a possessed boss fights as its driver's side, and a policy
        // that read the authored faction would have it defending the team it was
        // taken from.
        Query<(
            &'static ambition_combat::components::ActorFaction,
            Option<&'static ambition_characters::control::DrivingParticipant>,
            Option<&'static ambition_combat::targeting::MatchTeam>,
        )>,
    ),
    // R3: boss damage mutates the boss ENTITY directly (`apply_boss_hit` →
    // `apply_entity_boss_damage`), so this system no longer needs the boss
    // encounter resources — death save/quest/music resolution lives in
    // `update_boss_encounters`.
) {
    let mut feel = bark_draw.feel();
    let resolved_rules = bark_draw.rules();
    feel.di_max_angle = resolved_rules.di_max_angle;
    // The MATCH's post-hit window, folded the way `di_max_angle` above is. An
    // undeclared world scales by `1.0` and keeps the repeat guard it always had.
    feel.hit_repeat_window_scale = resolved_rules.hit_repeat_window_scale;
    // AE6: the MATCH's friendly-fire rule, not the world's baseline toggle —
    // the same value the body resolver reads.
    let friendly_fire = resolved_rules.friendly_fire();
    let catalog = &*catalogs.characters;
    // AD8: the prepared cast, borrowed once beside the catalog it stands behind.
    let prepared = catalogs.prepared.as_deref();
    // Wave-1 follow-up: apply the player's outgoing power-slider scale to their
    // MELEE, the way `ProjectileKind::spec` already scales player projectiles.
    // Enemy melee (a non-`PlayerSlash` source) is untouched; incoming
    // difficulty/assist is the separate `resolve_body_hit` scale.
    let outgoing_melee_scale = user_settings
        .map(|s| s.gameplay.player_damage_multiplier)
        .unwrap_or(1.0);
    for mut event in hit_events.read().cloned() {
        // The ONE seam the human's outgoing melee is scaled by their difficulty
        // slider. Scale once, before any victim reads `event.damage`; projectiles
        // are already scaled at spawn, so melee-only avoids double-scaling.
        //
        // A possessed enemy's swing carries that spelling and an empowered ally's does not, so the
        // slider reached the wrong strikes in both directions. And it blocks the cause-vocabulary
        // fold outright: once one `Melee` covers every swing, this gate would scale ENEMY damage by
        // the player's own multiplier.
        let attacker_is_controlled = event
            .attacker
            .is_some_and(|attacker| controlled_attackers.contains(attacker));
        if attacker_is_controlled && matches!(event.source, HitSource::Melee) {
            event.damage = (((event.damage as f32) * outgoing_melee_scale).round() as i32).max(1);
        }
        // PogoBounce hits target only the breakable whose AABB
        // approximately matches the orb volume the engine reported.
        // Skip the actor / boss / broadcast-breakable scans entirely;
        // jump straight to the orb-match loop at the bottom.
        if matches!(event.source, HitSource::Pogo) {
            for (entity, _id, name, aabb, mut feature) in &mut breakables {
                if feature.broken() || !feature.breakable.pogo_refresh {
                    continue;
                }
                if !approximately_same_aabb(aabb.aabb(), event.volume.bounds()) {
                    continue;
                }
                let broke = feature.breakable.apply_damage(event.damage.max(1));
                writers.vfx.write(VfxMessage::Impact { pos: aabb.center });
                if broke {
                    begin_ecs_breakable_respawn(&mut writers.commands, entity, &feature.breakable);
                    banner.show(format!("shattered {}", name.0.as_str()), 2.6);
                    emit_breakable_destroyed(
                        aabb.center,
                        &mut writers.sfx,
                        &mut writers.vfx,
                        &mut writers.debris,
                    );
                }
            }
            continue;
        }
        // Relational actor-vs-actor (S3e): an event pre-resolved to a single
        // non-player actor victim (`HitTarget::Body`) is applied to exactly that
        // body, whatever its source direction — this is how an Enemy/Boss swing
        // damages another actor without flowing through the player path.
        let actor_target = match event.target {
            ambition_combat::events::HitTarget::Body(entity) => Some(entity),
            _ => None,
        };
        // Body victims are already resolved by entity. `UnresolvedFeatures` is
        // only for targets such as breakables or boss encounters that the body
        // resolver cannot name, so do not rescan actors for it.
        let bodies_already_resolved = matches!(
            event.target,
            ambition_combat::events::HitTarget::UnresolvedFeatures
        );
        // Is the attacker a HEAVY body? — asked of the attacker entity, which
        // the event already names, rather than pattern-matched out of the cause
        // vocabulary. A boss launches harder and stuns longer; that is a fact
        // about the striker, not about the word its hit happens to be filed
        // under, and reading it here is what lets the vocabulary lose its
        // `BossAttack` / `BossBody` spellings without losing the feel.
        let heavy_attacker = event
            .attacker
            .is_some_and(|attacker| heavy_attackers.contains(attacker));
        // Victim-side sources (enemy touch, enemy swings, boss body
        // contact, hazards) are consumed by the player-damage path.
        // The feature drain only applies attacker-side player hits
        // here (plus the pre-resolved actor-vs-actor hits above);
        // otherwise an `EnemyBody` event would damage the same enemy
        // that emitted it when the volume overlaps its own AABB.
        //
        // `UnresolvedFeatures` passes on its TARGET, not on its source word: the
        // target already says this is a strike hunting for what it could not
        // name. Asking `seeks_victims` as well would drop every enemy swing's
        // unresolved half, since the direction words file those victim-side.
        if actor_target.is_none() && !bodies_already_resolved && !event.source.seeks_victims() {
            continue;
        }
        // Ignore-keys (`prefix:id`) of every target struck by THIS event, folded
        // back into the attacker's per-swing `hit_targets` below so a slash that
        // emits on every active frame only damages each target once.
        let mut landed_keys: Vec<String> = Vec::new();
        let mut actor_hit_this_event = false;
        for (
            actor_entity,
            id,
            aabb,
            mut identity,
            disposition,
            mut combat,
            mut aggression,
            interaction,
            control,
            mut motion_model,
            wallet_shield,
            mut cq,
            (combat_tuning, ruleset_owns_death, active_combatant, out_of_play),
        ) in actors.iter_mut().filter(|_| !bodies_already_resolved)
        {
            // Pre-resolved actor victim: apply ONLY to that entity.
            if let Some(target_entity) = actor_target {
                if actor_entity != target_entity {
                    continue;
                }
            }
            // IDENTITY BEATS EVERY RELATIONSHIP RULE — the body resolver's
            // first line, and this scan did not have it. A broadcast that
            // overlaps its own emitter damaged it.
            //
            // the DIRECTION words were doing this job, which is why nobody
            // noticed: a body-contact hit was filed victim-side, the drain
            // skipped every victim-side broadcast, and the self-hit could not
            // arise. Fold the direction out of the vocabulary and the protection
            // leaves with it — so state the rule that was actually wanted.
            if event.attacker == Some(actor_entity) {
                continue;
            }
            let prefix = if disposition.is_hostile() {
                "enemy"
            } else {
                "npc"
            };
            if target_is_ignored(&event.ignored_targets, prefix, id.as_str()) {
                continue;
            }
            if !ambition_combat::hitbox::strike_reaches_victim(
                &event.volume,
                victim_volumes.get(actor_entity).ok(),
                aabb,
            ) {
                continue;
            }
            let interactable = interaction.map(|i| &i.interactable);
            // The victim's held locomotion (local frame) drives DI (CM2).
            let di_input_local = control.map(|c| c.0.locomotion.vec()).unwrap_or_default();
            let mut em = cq.as_actor_mut();
            // Structural tangibility gate: a dead body is an
            // intangible corpse — a strike neither lands on it nor barks back.
            // This is the ONE place the player-slash actor path consults
            // tangibility, so the peaceful branch (which has no alive check of its
            // own) is covered too; `resolve_body_hit`'s alive check remains as
            // last-line defense.
            if ambition_combat::util::body_is_untouchable(Some(&*em.health), out_of_play) {
                continue;
            }
            let hurt = combat_tuning.map(|ct| ct.hurt_feedback).unwrap_or_default();
            if apply_actor_hit(
                &event,
                catalog,
                prepared,
                &catalogs.sheets,
                actor_entity,
                *disposition,
                ruleset_owns_death,
                active_combatant,
                &mut em,
                &mut motion_model,
                &mut combat,
                wallet_shield.map(|(wallet, shield)| {
                    ambition_damage::WalletArmor::new(wallet.into_inner(), shield)
                }),
                aggression.as_deref_mut(),
                interactable,
                &mut banner,
                combat_banter.as_deref(),
                feel,
                di_input_local,
                hurt,
                heavy_attacker,
                // The victim's published evade, read by entity: the actor
                // cluster query is at Bevy's column ceiling, and this is the
                // same by-entity shape `heavy_attacker` above uses.
                victim_motion
                    .get(actor_entity)
                    .is_ok_and(ambition_platformer2d_core::BodyMotionFacts::evading),
                // ⭐⭐ MAY THIS HIT SPEAK? Jon, 2026-08-24: *"not have barks
                // happen every time a character is hit. Make it a more rare
                // event. Not never."*
                //
                // ⛔ THE VICTIM IS THE SALT, so two fighters struck on the SAME
                // tick decide independently — one salt would make them chorus,
                // which is louder than the thing being fixed.
                //
                // ⛔ AND IT IS `sim_random`, never a stream: this is read inside
                // the rollback window, so a resimulated hit has to reach the
                // same answer or the bubble flickers on a rewind.
                bark_draw.allows(&resolved_rules, writers.identities.get(actor_entity).ok()),
                &mut writers,
            ) {
                actor_hit_this_event = true;
                landed_keys.push(format!("{prefix}:{}", id.as_str()));
                sync_actor_components_from_cluster(&em, &mut identity);
            }
        }
        let mut boss_hit_this_event = false;
        // May this attacker hurt this boss? — the same relational question,
        // answered by the same function, that the body resolver asks of every
        // other victim.
        //
        // this scan asked nothing at all. It damaged any boss an
        // attacker-side volume reached, and got away with it because only the
        // player was allowed to broadcast one — so a boss's "who may hurt me"
        // rule was encoded as *who is permitted to emit a broadcast*, in another
        // crate, by omission. That is exactly the encoding the cause-vocabulary
        // fold dissolves, and a rule that lives in who-may-speak cannot survive
        // everyone being able to speak.
        //
        // an UNATTRIBUTED hit still lands. A broadcast with no attacker cannot
        // be adjudicated, and refusing it would silently disarm the hazard and
        // scripted-blast paths that legitimately carry no entity.
        let may_damage_boss = |boss_entity: Entity| {
            boss_damage_allowed(
                event.attacker.and_then(|a| combat_sides.get(a).ok()),
                combat_sides.get(boss_entity).ok(),
                friendly_fire,
                boss_entity,
            )
        };
        // A pre-resolved actor-vs-actor hit never spills onto bosses / breakables.
        for (
            boss_entity,
            id,
            _aabb,
            mut feature,
            mut health,
            mut combat,
            wallet_shield,
            attack_state,
            animation_frame,
            boss_tuning,
            boss_out_of_play,
        ) in bosses.iter_mut().filter(|_| actor_target.is_none())
        {
            if target_is_ignored(&event.ignored_targets, "boss", id.as_str()) {
                continue;
            }
            if !may_damage_boss(boss_entity) {
                continue;
            }
            // Structural tangibility gate: a defeated boss is intangible — no hit
            // lands and no bark answers. (`apply_boss_hit` also guards on `alive()`
            // as defense-in-depth.)
            if ambition_combat::util::body_is_untouchable(Some(&*health), boss_out_of_play) {
                continue;
            }
            let hurt = boss_tuning.map(|ct| ct.hurt_feedback).unwrap_or_default();
            if apply_boss_hit(
                &catalogs.bosses,
                &event,
                boss_entity,
                feature.as_boss_mut(),
                &mut health,
                &mut combat,
                wallet_shield.map(|(wallet, shield)| {
                    ambition_damage::WalletArmor::new(wallet.into_inner(), shield)
                }),
                attack_state,
                animation_frame,
                &mut banner,
                combat_banter.as_deref(),
                hurt,
                &mut writers,
            ) {
                boss_hit_this_event = true;
                landed_keys.push(format!("boss:{}", id.as_str()));
            }
        }

        // WHO STRUCK, asked ONCE per event (AC7, probe B). The rule
        // below stood here and again 150 lines down, and the two copies did not
        // agree: this one refuses to guess unless the event is an unresolved
        // BROADCAST from a victim-seeking source, and the breakable fold simply
        // wrote `event.attacker.or_else(|| primary_q.single().ok())` — any melee
        // with no attacker credited whichever body happens to be the home avatar.
        //
        // the reasoning is this copy's own and it is the correct one: *"We do not know who
        // did this" is true of a broadcast and of nothing else.
        let unresolved_broadcast = matches!(
            event.target,
            ambition_combat::events::HitTarget::Volume
                | ambition_combat::events::HitTarget::UnresolvedFeatures
                | ambition_combat::events::HitTarget::OrbMatch
        );
        let target_attacker = event.attacker.or_else(|| {
            (unresolved_broadcast && event.source.seeks_victims())
                .then(|| primary_q.single().ok())
                .flatten()
        });
        if actor_hit_this_event || boss_hit_this_event {
            // an UNRESOLVED broadcast may fall back to the primary; a hit
            // that named its victim may not. This used to ask
            // `source.defaults_to_primary_attacker()` — a list of the
            // player-spelled causes — which is the same question asked through
            // the vocabulary, and it gives the wrong answer the moment one
            // `Melee` covers every swing: an enemy shot that named its victim
            // and carries no entity owner would credit the player with the
            // confirm, the hitstop and the per-swing bookkeeping.
            //
            // "We do not know who did this" is true of a broadcast and of nothing else.
            if let Some(attacker) = target_attacker {
                let record_dedup = matches!(event.source, HitSource::Melee);
                // CM4: the strike connected — the attacker's playing move
                // learns it (combo-confirm for OnHit/OnWhiff cancels).
                if let Ok(mut pb) = attacker_moves.get_mut(attacker) {
                    pb.landed_hit = true;
                    // Persist one-hit-per-target dedup on the MOVE itself. The
                    // per-swing accumulator below lives on `BodyMelee.swing`, which
                    // a `MovesetMelee` body rebuilds every frame — so without this
                    // the strike re-hit + re-fired the hit SFX every active tick.
                    // `MovePlayback` is the persistent per-strike home.
                    if record_dedup {
                        pb.hit_targets.extend(landed_keys.iter().cloned());
                    }
                }
                for (entity, mut combat, active_attack) in &mut player_combat_q {
                    if entity != attacker {
                        continue;
                    }
                    // Now the feel field is authoritative. the attacker freezes for exactly
                    // as long as its victim, from the one hitlag law, scaled by the hit it
                    // just landed.
                    combat.hitstop_timer = combat.hitstop_timer.max(
                        ambition_platformer2d_core::hit_response::hitlag_duration(
                            event.damage,
                            &ambition_combat::hit_reaction::hit_response_tuning(
                                &feel,
                                heavy_attacker,
                            ),
                        ),
                    );
                    // Only the body-flash is wrong.
                    if !matches!(event.source, HitSource::Projectile) {
                        combat.hit_flash = combat.hit_flash.max(0.10);
                    }
                    // Record the targets this slash just struck so the next active
                    // frame's emit ignores them (one hit per target per swing).
                    if record_dedup {
                        if let Some(mut active) = active_attack {
                            if let Some(state) = active.swing.as_mut() {
                                state.hit_targets.extend(landed_keys.iter().cloned());
                            }
                        }
                    }
                    break;
                }
            }
            // CM8: the hit SOUND is no longer emitted here (this shared confirm
            // once played `SfxMessage::Hit`, doubling the victim's own sound). It
            // now belongs to the ONE victim-side reaction, keyed on the attack's
            // `strike_sfx` over the victim's `HurtFeedback`. This block keeps only
            // the ATTACKER's feel (hitstop / flash / combo-confirm dedup).
        }

        // Struck breakables, keyed for the one-hit-per-target dedup. Collected here and folded
        // below, exactly like the factioned targets.
        let mut breakable_keys: Vec<String> = Vec::new();
        for (entity, id, name, aabb, mut feature) in
            breakables.iter_mut().filter(|_| actor_target.is_none())
        {
            if target_is_ignored(&event.ignored_targets, "breakable", id.as_str()) {
                continue;
            }
            if feature.broken() || !feature.breakable.trigger.allows_hit() {
                continue;
            }
            if feature.breakable.pogo_refresh {
                continue;
            }
            if !event.volume.intersects_aabb(aabb.aabb()) {
                continue;
            }
            let broke = feature.breakable.apply_damage(event.damage.max(1));
            breakable_keys.push(format!("breakable:{}", id.as_str()));
            let impact = midpoint(event.volume.center(), aabb.center);
            // CM8 on a prop: a breakable has no `HurtFeedback` of its own (no
            // spray, no debris — breaking already has its own FX), so it only
            // borrows the ATTACK's half of the rule. An attack that authored a
            // strike sound is heard here exactly as it is on a body; one that
            // authored none stays silent, as it always has. `METAL` is the
            // material a rigid prop resolves a material-aware strike against —
            // it is NOT a sound the prop makes on its own, so no
            // `strike_sfx == <this cue>` test is needed to keep it quiet.
            if let Some(strike) = event.strike_sfx {
                writers.sfx.write(ambition_sfx::SfxMessage::Play {
                    id: ambition_combat::util::resolve_strike_sfx(
                        ambition_vfx::HurtFeedback::METAL,
                        Some(strike),
                        event.damage,
                    ),
                    pos: impact,
                });
            }
            writers.vfx.write(VfxMessage::Impact { pos: impact });
            if broke {
                begin_ecs_breakable_respawn(&mut writers.commands, entity, &feature.breakable);
                banner.show(format!("broke {}", name.0.as_str()), 2.6);
                // Loot: a smashed crate/pot drops a small coin (same collectible
                // pickup path as enemy drops).
                let session_scope = writers.session_spawn_scope();
                if let Some(parent) = drop_parent(&writers, entity, "breakable", id.as_str()) {
                    drop_currency_coin(
                        &mut writers.commands,
                        session_scope,
                        &parent,
                        id.as_str(),
                        aabb.center,
                        BREAKABLE_BOUNTY,
                    );
                }
                emit_breakable_destroyed(
                    aabb.center,
                    &mut writers.sfx,
                    &mut writers.vfx,
                    &mut writers.debris,
                );
            }
        }

        // Fold struck breakables into the SAME one-hit-per-target accumulator the
        // factioned targets use. Their loop runs after the actor/boss fold-back and
        // they carry no i-frames, so without this a lingering player strike
        // re-smashed each breakable (and re-fired its Impact FX) every active tick.
        // Persist on the move (survives the moveset swing projection) AND the flat
        // swing (non-moveset bodies).
        if matches!(event.source, HitSource::Melee) && !breakable_keys.is_empty() {
            if let Some(attacker) = target_attacker {
                if let Ok(mut pb) = attacker_moves.get_mut(attacker) {
                    pb.hit_targets.extend(breakable_keys.iter().cloned());
                }
                for (entity, _combat, active_attack) in &mut player_combat_q {
                    if entity == attacker {
                        if let Some(mut active) = active_attack {
                            if let Some(state) = active.swing.as_mut() {
                                state.hit_targets.extend(breakable_keys.iter().cloned());
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
}

mod actor_hit;
mod boss_hit;
use actor_hit::*;
use boss_hit::*;

#[cfg(test)]
mod tests;
