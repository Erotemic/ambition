//! The combat rules a match plays under — resolved, not borrowed. (AE6)
//!
//! A route mutating global tuning and undoing it afterwards is a lifecycle borrowing an authority
//! it does not own. So the match DECLARES its rules ([`DeclaredCombatRules`]), a projection folds
//! them over the world's baseline every tick, and combat reads the result
//! ([`ResolvedCombatTuning`]). Nothing is written back, so there is nothing to restore and no
//! window in which the restore has not happened yet: removing the declaration IS the exit.
//!
//! ## Why the type is here and the projection is not
//!
//! `ResolvedCombatTuning` has to live at or below `ambition_combat`, because
//! `on_hit`, `hitbox` and `targeting` are its readers. Its INPUTS do not both
//! live there — `di_max_angle` belongs to `ambition_platformer2d_actor_monolith`' feel tuning — so
//! the projection system lives in `ambition_platformer2d_actor_monolith`, one layer up, where both
//! inputs are visible. Ownership travels down with the type; the fold happens
//! where the facts are.

use bevy::prelude::Resource;

/// What a match asks for. Present means a match (or any other owner of a
/// combat lifecycle) has declared rules; absent means the world's baseline
/// stands on its own.
///
/// Deliberately not `Option` fields: a rule a match does not care about is the
/// baseline's, and expressing that as "declare the baseline's value" would make
/// the declaration a snapshot of the world at declaration time — which is the
/// borrow again, wearing a different hat. A declarer that wants the world's DI
/// omits the whole resource, or reads it and re-declares deliberately.
/// WHO KEEPS A CONTESTED LEDGE. See
/// [`DeclaredCombatRules::ledge_occupancy`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LedgeOccupancy {
    /// The NEWEST grab wins and the older holder is knocked off — Ultimate's
    /// rule, and what every ledge in this engine did before the knob existed.
    #[default]
    Trump,
    /// The body already on the edge KEEPS it and the newcomer loses — Melee's
    /// rule, and the one that makes hogging an edge a real denial.
    Hog,
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct DeclaredCombatRules {
    /// Which shell experience declared these rules.
    ///
    /// required, not optional, and it is a LIFECYCLE field rather than a label. Two stages
    /// declare combat rules — the versus route and the smash demo — and each gives its
    /// declaration back when its experience leaves. Naming the declarer is what lets the
    /// release ask *is this mine* instead of assuming it.
    pub declared_by: String,
    /// How far a launched body may steer its own trajectory (CM2). `0.0`
    /// disables directional influence entirely, which is Ambition's PvE answer.
    pub di_max_angle: f32,
    /// How much a launch GROWS with the victim's accumulated damage, as a
    /// fraction of the move's own base launch speed per point of damage.
    ///
    /// `0.01` means *a hit doubles its launch at 100 damage* — the platform
    /// fighter's whole loop, where a fresh opponent is unlaunchable and a
    /// hundred-percent one dies to the same jab. `0.0` is flat knockback, which
    /// is Ambition's PvE answer and the engine baseline.
    ///
    /// a RULESET fact, not a per-move one, and that is the point. A move may still author its
    /// own `knockback_growth` on a hit volume and that wins outright; this is what a stage says
    /// when its moves author none.
    ///
    /// it scales the move's BASE launch rather than being an absolute
    /// px/s-per-point, so a jab grows less than a smash out of one number. That
    /// is the property a per-move table would otherwise have to restate for
    /// every move.
    pub knockback_growth: f32,
    /// What a DOWNWARD hit does to the attacker.
    ///
    /// one move, two games. The robot's
    /// down-air is one authored swing with one hitbox and one launch direction —
    /// and Ambition reads it as a POGO that bounces the attacker up off whatever
    /// it hit, while a platform fighter reads it as a SPIKE that drives the
    /// victim down and ends a stock offstage. Both readings are correct for
    /// their game, and neither belongs on the move.
    ///
    /// this is what stopped the protagonist carrying its own repertoire.
    /// Attaching the canonical moveset to `player_robot_v3` turned
    /// `gravity_symmetry::pogo_bounces_away_from_gravity_under_both_orientations`
    /// red, and the wrong fix
    /// — authoring the robot a second, Ambition-only down-air — is the
    /// duplicate-moves outcome §16 explicitly forbids.
    ///
    /// [`DownwardHitStyle::Pogo`] is the baseline BECAUSE it is today's
    /// behaviour: the effect is authored on the volume, so an undeclared world
    /// keeps firing it. A stage that wants spikes says so.
    pub downward_hit: DownwardHitStyle,
    /// How long a body spiked out of the AIR cannot recover (seconds).
    /// `0.0` — the baseline — is no meteor rule at all, which is what an
    /// exploration game wants: a downward hit there is a pogo or a shove, not a
    /// sentence.
    ///
    /// it belongs beside [`Self::downward_hit`] and nowhere else. That
    /// field already decides whether this game reads a downward hit as a rebound
    /// or a SPIKE; how long the spiked body is silent is the same question one
    /// step further, and a game that declares `Spike` is exactly the game that
    /// wants to answer it. It briefly lived on the global feel tuning, where it
    /// had no way to be true for one experience and false for another.
    ///
    /// what the genre calls "meteor cancel" is this window ENDING. There is no
    /// second verb to press.
    pub meteor_lock_time: f32,
    /// RAGE — how much a body's OWN accumulated damage raises the knockback it
    /// DEALS, per point. `0.0` (the baseline) is no rage at all.
    ///
    /// the mirror of the percent mechanic and the reason a losing fighter is
    /// dangerous: a body already scales the knockback it TAKES by its own
    /// damage, so without this the fighter behind is punished twice — easier to
    /// launch and no harder to be launched by. Rage is what makes a comeback a
    /// thing the rules produce rather than a thing a player hopes for.
    ///
    /// capped by [`Self::rage_max_scale`], because uncapped it turns the last
    /// stock into a coin flip.
    pub rage_per_damage: f32,
    /// The ceiling on [`Self::rage_per_damage`], as a multiplier. `1.0` = rage
    /// can never help, whatever the per-point rate says.
    pub rage_max_scale: f32,
    /// STALING — how much of its strength a move loses per recent landing of
    /// the same move. `0.0` (the baseline) is no staling.
    ///
    /// it exists to stop one good answer being the ONLY answer. A fighter
    /// with a reliable kill move should have to vary, and a fighter who has
    /// worn one out should find the others suddenly worth throwing.
    pub stale_step: f32,
    /// The floor [`Self::stale_step`] cannot take a move below, as a multiplier.
    /// `1.0` = staling can never weaken anything.
    pub stale_floor: f32,
    /// CROUCH CANCEL — what a CROUCHING victim multiplies an incoming launch
    /// by. `1.0` (the baseline) = crouching buys nothing but a shorter
    /// hurtbox.
    ///
    /// it makes ducking a defensive READ rather than only a shape. flat,
    /// with no percent threshold, because the threshold is emergent: 85% of a
    /// kill move is still a kill, so the option stops mattering by itself
    /// exactly where the genre stops using it.
    pub crouch_cancel_scale: f32,
    /// POST-HIT MERCY WINDOW, as a fraction of the one this game's own feel
    /// tuning authors. `1.0` (the baseline) keeps each damage road's window
    /// exactly as it was; `0.0` says this game has NO blanket window at all.
    ///
    /// ⭐ the window and a strike's own per-target dedup are two answers to one
    /// question, and a platform fighter needs the second. `HitboxHits` already
    /// stops a lingering volume re-hitting, and SEPARATED authored Active
    /// windows are meant to re-hit — that is what a multi-hit move IS. A
    /// blanket window longer than the gap an author wrote makes the later pulse
    /// unreachable: George Booul's `bivalence` authors a weak pop at 0.30s and
    /// a launcher at 0.42s, and the launcher could never land on the body the
    /// pop had hit, because a 0.2s window outlives a 0.12s gap. The kill move
    /// in the demo's most-watched mode had never once connected.
    ///
    /// A SCALE rather than an absolute so the undeclared identity is exactly
    /// today's behaviour on both roads, which arm different windows: the actor
    /// road's is a repeat guard and the player road's is Mario-style mercy
    /// invincibility, and neither is wrong for the game that authored it.
    pub hit_repeat_window_scale: f32,
    /// How long a grab holds a body at 0%, in seconds. Ultimate's 90 frames.
    pub grab_hold_base_seconds: f32,
    /// How much longer per point of the CAPTIVE's damage. Ultimate's 1.7
    /// frames per percent, so a fighter at 100% is held roughly twice as long.
    ///
    /// it makes the grab a percent mechanic like everything else here: the
    /// body that is losing is the body a grab is worth throwing at.
    ///
    /// read ONCE, when the hold begins — pummelling does not extend it, which
    /// is the genre's rule and the reason a pummel is a decision rather than a
    /// free extension of your own advantage.
    pub grab_hold_per_damage: f32,
    /// The ceiling on a hold however hurt the captive is. Also the answer to
    /// *"what ends a hold nobody ends"*: a captor who grabs and then does
    /// nothing must not hold a body for the rest of the match.
    pub grab_hold_max_seconds: f32,
    /// What one mash press buys the captive, in seconds off the hold.
    /// Ultimate's 14.4 frames.
    pub grab_mash_seconds: f32,
    /// Whether same-faction bodies damage each other.
    ///
    /// a match with declared TEAMS should leave this `false`.
    pub friendly_fire: bool,
    /// CLANK: how close two opposed attacks' damage must be for both to be
    /// refused. A difference STRICTLY GREATER than this and the stronger attack
    /// wins outright, continuing untouched while the weaker one is cancelled.
    ///
    /// ⭐ THE GENRE'S NUMBER IS ABOUT NINE, and it is research rather than a
    /// decision: Melee, Brawl, Smash 4 and Ultimate all resolve two grounded
    /// attacks meeting by comparing their damage, and all four use a threshold
    /// in this neighbourhood. Where they differ is detail nobody sees, so this
    /// is a KNOB with the genre's value as its default rather than one game's
    /// frame data transcribed.
    ///
    /// `0.0` disables clanking entirely — every attack passes through every
    /// other, which is what every game in this engine did before this field
    /// existed and what a non-fighter should keep doing.
    pub clank_damage_window: f32,
    /// How hard a traded attack throws its own thrower BACKWARD, in engine
    /// units per second.
    ///
    /// ⭐ THE REBOUND IS WHAT MAKES A CLANK A MECHANIC. Without it two attacks
    /// simply vanish and both fighters stand where they were, mid-animation,
    /// with nothing having happened — which reads as the game dropping inputs.
    /// The genre pushes both bodies apart and takes their moves away, so a trade
    /// resets the exchange instead of freezing it.
    ///
    /// `0.0` = a clank cancels the attacks and moves nobody. Irrelevant while
    /// [`Self::clank_damage_window`] is zero, because then nothing clanks.
    pub clank_rebound_speed: f32,
    /// EDGE CANCEL: does an aerial's landing lag SURVIVE losing the ground
    /// under it?
    ///
    /// `None`/`false` keeps what every body did before this existed — the lag
    /// is a timer and it runs out wherever the body happens to be, including
    /// halfway down a pit. `Some(true)` ends it the moment ground support
    /// disappears.
    ///
    /// ⭐ IT IS THE SAME COMMITMENT SEEN FROM THE OTHER SIDE. Landing lag
    /// exists because an aerial that touches down mid-move should cost you; a
    /// body sliding off a platform lip is no longer touched down, so there is
    /// nothing left for it to be paying. Charging it anyway freezes a body in
    /// mid-air, which is the one place the lag never meant to describe.
    ///
    /// ⛔ A RULE, NOT A PER-MOVE FIELD. Every move's lag cancels or none does;
    /// authoring it per move would be an exemption list, and the genre applies
    /// it to the whole cast.
    pub edge_cancel_recovery: Option<bool>,
    /// B-REVERSE: does a special started with a BACK press turn the fighter
    /// around? `None`/`false` is what every body did — a special comes out the
    /// way you were already facing.
    ///
    /// ⭐ THE SPECIAL'S OWN RESOLVED DIRECTION decides it, not the live stick:
    /// `AttackDir::Back` already means "away from facing" and is republished
    /// for the whole buffered window, so a press read after the stick centres
    /// still means what it meant.
    ///
    /// ⛔ FACING ONLY. The momentum half is [`Self::special_turn_reverses_drift`]
    /// and it is deliberately separate: the genre has both, and a game that
    /// wants the turn without the launch-cancel should not have to take both.
    pub special_turn: Option<bool>,
    /// Does a Back+Special ALSO reverse the fighter's drift?
    ///
    /// ⛔⛔ INDEPENDENT OF [`Self::special_turn`], and that is the point. It was
    /// gated behind the turn until 2026-08-25, which made one real technique
    /// undeclarable — the WAVEBOUNCE, where momentum reverses and the facing
    /// does NOT:
    ///
    /// ```text
    /// turn  drift   technique
    /// ────  ─────   ──────────────────────────────────────────────
    /// no    no      an ordinary special
    /// yes   no      turnaround-B — you come out facing the other way
    /// yes   yes     B-reverse — facing AND momentum turn
    /// no    yes     a wavebounce — momentum turns, facing does not
    /// ```
    ///
    /// Four outcomes from two bits of ONE special-start rule, rather than four
    /// mechanics — which is what keeps any of them from becoming a
    /// fighter-specific velocity hack.
    ///
    /// ⚠ A GAME DECLARES WHICH ONE ITS BACK+SPECIAL PERFORMS. The genre lets a
    /// player pick per press, by the ORDER of stick and button; this seam is
    /// handed one already-resolved direction and cannot see that order.
    pub special_turn_reverses_drift: Option<bool>,
    /// SUDDEN DEATH: the damage every surviving fighter starts on when a timed
    /// match runs out genuinely level. `None` = no sudden death, and a level
    /// timeout is simply a draw.
    ///
    /// ⭐⭐ IT IS NOT AN OUTCOME, IT IS THE MATCH CONTINUING. A tie is entered
    /// INSTEAD of being decided — the match was never settled, so nothing has to
    /// mutate a finished match back into a running one, which is the trap this
    /// mechanic is usually built into. The clock stops applying, both sides are
    /// put on the edge of death, and the fight decides it the ordinary way:
    /// last side standing.
    ///
    /// ⚠ THE NUMBER IS THE GENRE'S SHAPE, not one game's: every Smash puts the
    /// survivors at very high damage so the next clean hit ends it. What is
    /// authored is the damage, because that is the knob that says how short
    /// "short" is.
    pub sudden_death_damage: Option<i32>,
    /// How often a struck body SAYS something, `0.0..=1.0`. `None` — and the
    /// resolved default — is `1.0`: every hit barks, which is what every body
    /// did before this existed.
    ///
    /// ⭐⭐ IT IS A RATE, NOT A COOLDOWN. Jon, 2026-08-24: *"not have barks
    /// happen every time a character is hit. Make it a more rare event. Not
    /// never, but I'd like it to happen less often."* A cooldown would make the
    /// FIRST hit of every exchange bark and the rest silent, which is a rhythm;
    /// a rate keeps them unpredictable, which is what "rare" sounds like.
    ///
    /// ⛔ THE DRAW IS `sim_random`, never a stream: a bark that differed between
    /// peers would desync nothing and look like a bug forever. See
    /// [`ambition_platformer2d_core::sim_random`].
    pub bark_chance: Option<f32>,
    /// LEDGE TRUMP POP — how fast a body that has just had its ledge STOLEN is
    /// thrown off it, engine units/s. `None` and `0.0` drop it in place, which
    /// is what every trump did before this existed.
    ///
    /// ⭐⭐ THE KNOB IS THE POINT, because this is where the GAMES DIFFER.
    /// Trumping exists in every platform fighter; being popped outward and
    /// briefly committed does not. Ambition's own answer is to drop, and a stage
    /// that wants the harsher rule says so — shipping one of them as the law
    /// would be choosing a game rather than building an engine.
    ///
    /// ⛔ OUTWARD MEANS AWAY FROM THE WALL, resolved from the hang's own
    /// `wall_normal_x`. Reading it off the trumped body's facing would be wrong
    /// the moment a body hangs facing out.
    pub ledge_trump_pop: Option<f32>,
    /// WHO KEEPS A CONTESTED EDGE — the newcomer or the body already on it?
    ///
    /// `None` is [`LedgeOccupancy::Trump`], which is what every ledge did
    /// before this knob: the newest grab wins and the older holder is knocked
    /// off. [`LedgeOccupancy::Hog`] is the older generation's answer — the body
    /// that got there first keeps it, and the newcomer loses instead.
    ///
    /// ⭐⭐ THIS IS WHERE THE GAMES DIFFER, which is exactly what should be a
    /// knob rather than a decision: Melee lets you hog an edge and deny a
    /// recovery outright, Ultimate lets the recovering fighter steal it back.
    /// Both are coherent games and neither is a bug.
    ///
    /// ⛔ ONE AUTHORITY EITHER WAY. The policy chooses which holder SURVIVES;
    /// it adds no second rule about who may grab. `resolve_ledge_trumps` still
    /// owns the edge and the loser is knocked off by the same path, with the
    /// same [`Self::ledge_trump_pop`].
    pub ledge_occupancy: Option<LedgeOccupancy>,
    /// THE DOUBLE-JUMP CANCEL: does throwing an aerial out of a jump spent in
    /// the air kill the rest of that jump's rise?
    ///
    /// `None`/`false` is what every body did — an air jump runs its full arc
    /// whatever you throw out of it.
    ///
    /// ⭐ IT TURNS A DOUBLE JUMP FROM A COMMITMENT INTO AN APPROACH: rise,
    /// throw, and land where you chose rather than at the top of the arc. That
    /// is the whole reason the technique has a name.
    ///
    /// ⛔ IT CANCELS A RISE THE JUMP OWNS, and the bound lives in
    /// `BodyMotionFacts::air_jump_rise_owned` rather than here — an AMOUNT, so
    /// the cancel sheds exactly what the jump put in and leaves a launch the
    /// fighter is riding alone.
    pub double_jump_cancel: Option<bool>,
}

/// The rules combat actually reads this tick.
///
/// Derived every tick from [`DeclaredCombatRules`] folded over the world's
/// baseline. A reader must never consult the baseline resources directly: that
/// is how a stage's rules and the world's rules got to disagree.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ResolvedCombatTuning {
    pub di_max_angle: f32,
    /// See [`DeclaredCombatRules::knockback_growth`]. `0.0` = flat knockback.
    pub knockback_growth: f32,
    /// See [`DeclaredCombatRules::downward_hit`].
    pub downward_hit: DownwardHitStyle,
    /// See [`DeclaredCombatRules::meteor_lock_time`].
    pub meteor_lock_time: f32,
    /// See [`DeclaredCombatRules::rage_per_damage`].
    pub rage_per_damage: f32,
    /// See [`DeclaredCombatRules::rage_max_scale`].
    pub rage_max_scale: f32,
    /// See [`DeclaredCombatRules::stale_step`].
    pub stale_step: f32,
    /// See [`DeclaredCombatRules::stale_floor`].
    pub stale_floor: f32,
    /// See [`DeclaredCombatRules::crouch_cancel_scale`].
    pub crouch_cancel_scale: f32,
    /// See [`DeclaredCombatRules::hit_repeat_window_scale`].
    pub hit_repeat_window_scale: f32,
    /// See [`DeclaredCombatRules::edge_cancel_recovery`].
    pub edge_cancel_recovery: bool,
    /// See [`DeclaredCombatRules::special_turn`].
    pub special_turn: bool,
    /// See [`DeclaredCombatRules::special_turn_reverses_drift`].
    pub special_turn_reverses_drift: bool,
    /// See [`DeclaredCombatRules::clank_damage_window`]. `0.0` = attacks pass
    /// through each other, which is what an undeclared world does.
    pub clank_damage_window: f32,
    /// See [`DeclaredCombatRules::clank_rebound_speed`].
    pub clank_rebound_speed: f32,
    /// See [`DeclaredCombatRules::sudden_death_damage`].
    pub sudden_death_damage: Option<i32>,
    /// See [`DeclaredCombatRules::bark_chance`]. RESOLVED, so `1.0` — every hit
    /// barks — is what a world that declared nothing gets, which is what every
    /// body did before the knob existed.
    pub bark_chance: f32,
    /// See [`DeclaredCombatRules::ledge_trump_pop`]. RESOLVED, so `0.0` — drop
    /// the trumped body in place — is what a world that declared nothing gets.
    pub ledge_trump_pop: f32,
    /// See [`DeclaredCombatRules::ledge_occupancy`].
    pub ledge_occupancy: LedgeOccupancy,
    /// See [`DeclaredCombatRules::double_jump_cancel`].
    pub double_jump_cancel: bool,
    /// See [`DeclaredCombatRules::grab_hold_base_seconds`].
    pub grab_hold_base_seconds: f32,
    /// See [`DeclaredCombatRules::grab_hold_per_damage`].
    pub grab_hold_per_damage: f32,
    /// See [`DeclaredCombatRules::grab_hold_max_seconds`].
    pub grab_hold_max_seconds: f32,
    /// See [`DeclaredCombatRules::grab_mash_seconds`].
    pub grab_mash_seconds: f32,
    pub friendly_fire: bool,
}

/// How this game reads a downward attack. See
/// [`DeclaredCombatRules::downward_hit`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DownwardHitStyle {
    /// The ATTACKER rebounds off what it hit — Hollow Knight's down-slash, and
    /// Ambition's. The default, because it is what an authored `pogo_bounce`
    /// effect already does and an undeclared world must not change.
    #[default]
    Pogo,
    /// The attacker keeps falling and the VICTIM is driven down — a platform
    /// fighter's spike, which is a kill offstage and would be nonsense if it
    /// also bounced you back to safety.
    Spike,
}

impl DeclaredCombatRules {
    /// Whether `owner` is the experience that declared these rules — the
    /// question `releasing_owned` asks on the way out.
    pub fn is_declared_by(&self, owner: &str) -> bool {
        self.declared_by == owner
    }
}

pub const FLAT_GRAB_HOLD_SECONDS: f32 = 4.0;
/// What one mash press buys in an undeclared world, in seconds. Twenty
/// presses cleared the old fractional accumulator, and twenty of these clear
/// [`FLAT_GRAB_HOLD_SECONDS`].
pub const FLAT_GRAB_MASH_SECONDS: f32 = FLAT_GRAB_HOLD_SECONDS / 20.0;

impl ResolvedCombatTuning {
    /// What a move is worth after `occurrences` recent landings of it, as a
    /// multiplier, floored. `1.0` for a game that declares no staling and for a
    /// move nobody has thrown lately.
    pub fn stale_scale(self, occurrences: u32) -> f32 {
        if self.stale_step <= 0.0 || occurrences == 0 {
            return 1.0;
        }
        (1.0 - self.stale_step * occurrences as f32).max(self.stale_floor.clamp(0.0, 1.0))
    }

    /// What an attacker's own damage multiplies its knockback by, capped.
    /// `1.0` for a game that declares no rage, and for a fresh fighter in one
    /// that does.
    pub fn rage_scale(self, attacker_damage_taken: i32) -> f32 {
        if self.rage_per_damage <= 0.0 {
            return 1.0;
        }
        (1.0 + self.rage_per_damage * attacker_damage_taken.max(0) as f32)
            .min(self.rage_max_scale.max(1.0))
    }

    /// How long a grab holds a body at this damage, in seconds, capped.
    ///
    /// the caller asks ONCE, as the hold begins, and stores the answer: this
    /// is the captive's percent AT THE GRAB, so damage dealt during the hold
    /// does not extend it.
    pub fn grab_hold_seconds(self, captive_damage_taken: i32) -> f32 {
        (self.grab_hold_base_seconds
            + self.grab_hold_per_damage * captive_damage_taken.max(0) as f32)
            .min(self.grab_hold_max_seconds)
    }

    /// The fold: a declaration wins outright, the baseline stands otherwise.
    pub fn resolve(
        declared: Option<DeclaredCombatRules>,
        baseline_di: f32,
        baseline_ff: bool,
    ) -> Self {
        match declared {
            Some(rules) => Self {
                di_max_angle: rules.di_max_angle,
                knockback_growth: rules.knockback_growth,
                downward_hit: rules.downward_hit,
                meteor_lock_time: rules.meteor_lock_time,
                rage_per_damage: rules.rage_per_damage,
                rage_max_scale: rules.rage_max_scale,
                stale_step: rules.stale_step,
                stale_floor: rules.stale_floor,
                crouch_cancel_scale: rules.crouch_cancel_scale,
                hit_repeat_window_scale: rules.hit_repeat_window_scale,
                grab_hold_base_seconds: rules.grab_hold_base_seconds,
                grab_hold_per_damage: rules.grab_hold_per_damage,
                grab_hold_max_seconds: rules.grab_hold_max_seconds,
                grab_mash_seconds: rules.grab_mash_seconds,
                friendly_fire: rules.friendly_fire,
                clank_damage_window: rules.clank_damage_window,
                clank_rebound_speed: rules.clank_rebound_speed,
                edge_cancel_recovery: rules.edge_cancel_recovery.unwrap_or(false),
                special_turn: rules.special_turn.unwrap_or(false),
                special_turn_reverses_drift: rules.special_turn_reverses_drift.unwrap_or(false),
                sudden_death_damage: rules.sudden_death_damage,
                // A world that declared no rate barks on every hit, which is
                // what every body did before the knob existed.
                bark_chance: rules.bark_chance.unwrap_or(1.0).clamp(0.0, 1.0),
                ledge_trump_pop: rules.ledge_trump_pop.unwrap_or(0.0).max(0.0),
                ledge_occupancy: rules.ledge_occupancy.unwrap_or_default(),
                double_jump_cancel: rules.double_jump_cancel.unwrap_or(false),
            },
            // growth has NO world baseline to fall back to, unlike DI and
            // friendly fire: nothing outside a declaration authors it, so an
            // undeclared world is flat — which is every Ambition room today.
            None => Self {
                di_max_angle: baseline_di,
                knockback_growth: 0.0,
                // an undeclared world POGOS, because that is what the authored
                // effect already does. Anything else would change every Ambition
                // room to buy a Smash feature.
                downward_hit: DownwardHitStyle::Pogo,
                meteor_lock_time: 0.0,
                rage_per_damage: 0.0,
                rage_max_scale: 1.0,
                stale_step: 0.0,
                stale_floor: 1.0,
                crouch_cancel_scale: 1.0,
                hit_repeat_window_scale: 1.0,
                grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_hold_per_damage: 0.0,
                grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
                friendly_fire: baseline_ff,
                // ⛔ AN UNDECLARED WORLD DOES NOT CLANK. Ambition's rooms are
                // not a fighting game: two swings meeting there have always both
                // landed, and turning that on to buy a Smash feature is the
                // mistake `downward_hit` above already names.
                clank_damage_window: 0.0,
                clank_rebound_speed: 0.0,
                // ⛔ AN UNDECLARED WORLD KEEPS ITS LAG. Nothing outside a
                // platform fighter was tuned expecting recovery to vanish at a
                // ledge — the same reasoning `clank_damage_window` states above.
                edge_cancel_recovery: false,
                // ⛔ AN UNDECLARED WORLD'S SPECIALS COME OUT THE WAY IT FACES.
                special_turn: false,
                special_turn_reverses_drift: false,
                sudden_death_damage: None,
                bark_chance: 1.0,
                ledge_trump_pop: 0.0,
                ledge_occupancy: LedgeOccupancy::Trump,
                double_jump_cancel: false,
            },
        }
    }

    /// The friendly-fire toggle in the shape `can_damage` already takes, so the
    /// targeting rule keeps ONE signature whichever side supplies it.
    pub fn friendly_fire(self) -> crate::targeting::FriendlyFire {
        crate::targeting::FriendlyFire {
            enabled: self.friendly_fire,
        }
    }
}

impl Default for ResolvedCombatTuning {
    /// The engine baseline: no directional influence, no friendly fire. Exists so
    /// a composition that never installs the projection still resolves rather
    /// than reading `None` as "zero rules".
    fn default() -> Self {
        Self {
            di_max_angle: crate::feel::Platformer2dFeelTuningMonolith::default().di_max_angle,
            // Every hit barks, which is what every body did before the knob.
            bark_chance: 1.0,
            ledge_trump_pop: 0.0,
            ledge_occupancy: LedgeOccupancy::Trump,
            double_jump_cancel: false,
            knockback_growth: 0.0,
            downward_hit: DownwardHitStyle::Pogo,
            meteor_lock_time: 0.0,
            rage_per_damage: 0.0,
            rage_max_scale: 1.0,
            stale_step: 0.0,
            stale_floor: 1.0,
            crouch_cancel_scale: 1.0,
            hit_repeat_window_scale: 1.0,
            grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
            grab_hold_per_damage: 0.0,
            grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
            grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
            friendly_fire: false,
            clank_damage_window: 0.0,
            clank_rebound_speed: 0.0,
            edge_cancel_recovery: false,
            special_turn: false,
            special_turn_reverses_drift: false,
            sudden_death_damage: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GRAB HOLDS THE HURT FIGHTER LONGER, AND STILL LETS GO.
    ///
    /// three points and not one: the base alone would pass with the rate at
    /// zero, and the rate alone would pass with no ceiling — which is the shape
    /// that turns a grab at high percent into a body removed from the match.
    #[test]
    fn a_grab_holds_longer_the_more_damage_the_captive_has_taken() {
        let rules = ResolvedCombatTuning {
            grab_hold_base_seconds: 1.5,
            grab_hold_per_damage: 0.02,
            grab_hold_max_seconds: 3.0,
            ..Default::default()
        };
        assert_eq!(rules.grab_hold_seconds(0), 1.5);
        assert_eq!(rules.grab_hold_seconds(50), 2.5);
        assert_eq!(
            rules.grab_hold_seconds(999),
            3.0,
            "a hold at high percent outlived its own ceiling"
        );
        // an undeclared world is FLAT, not zero: a rate of zero here would be
        // an instant release rather than "no percent mechanic".
        let flat = ResolvedCombatTuning::default();
        assert_eq!(flat.grab_hold_seconds(0), flat.grab_hold_seconds(300));
        assert!(flat.grab_hold_seconds(0) > 0.0);
    }

    #[test]
    fn an_undeclared_world_reads_its_own_baseline() {
        let resolved = ResolvedCombatTuning::resolve(None, 0.12, true);
        assert_eq!(resolved.di_max_angle, 0.12);
        assert!(resolved.friendly_fire);
    }

    /// The case the borrow could not express: a match's rules apply WITHOUT the
    /// baseline changing, so the world an experience authored is still there
    /// when the match ends.
    #[test]
    fn a_declaration_wins_without_disturbing_the_baseline() {
        let baseline_di = 0.12;
        let resolved = ResolvedCombatTuning::resolve(
            Some(DeclaredCombatRules {
                bark_chance: None,
                ledge_trump_pop: None,
                ledge_occupancy: None,
                double_jump_cancel: None,
                declared_by: "a_stage".to_string(),
                di_max_angle: 0.30,
                knockback_growth: 0.0,
                downward_hit: DownwardHitStyle::Pogo,
                meteor_lock_time: 0.0,
                rage_per_damage: 0.0,
                rage_max_scale: 1.0,
                stale_step: 0.0,
                stale_floor: 1.0,
                crouch_cancel_scale: 1.0,
                hit_repeat_window_scale: 1.0,
                grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_hold_per_damage: 0.0,
                grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
                friendly_fire: false,
                clank_damage_window: 0.0,
                clank_rebound_speed: 0.0,
                edge_cancel_recovery: None,
                special_turn: None,
                special_turn_reverses_drift: None,
                sudden_death_damage: None,
            }),
            baseline_di,
            true,
        );
        assert_eq!(resolved.di_max_angle, 0.30);
        assert!(!resolved.friendly_fire);
        // The baseline is a value this function READ; there is no path by which
        // it could have been written. That is the whole point of the seam, and
        // asserting it here is the cheapest place to say so.
        assert_eq!(baseline_di, 0.12);
    }

    /// Dropping the declaration is the exit. No restore step, so no window in
    /// which the restore has not happened yet.
    #[test]
    fn dropping_the_declaration_returns_to_the_baseline_with_no_restore_step() {
        let declared = Some(DeclaredCombatRules {
            bark_chance: None,
            ledge_trump_pop: None,
            ledge_occupancy: None,
            double_jump_cancel: None,
            declared_by: "a_stage".to_string(),
            di_max_angle: 0.30,
            knockback_growth: 0.0,
            downward_hit: DownwardHitStyle::Pogo,
            meteor_lock_time: 0.0,
            rage_per_damage: 0.0,
            rage_max_scale: 1.0,
            stale_step: 0.0,
            stale_floor: 1.0,
            crouch_cancel_scale: 1.0,
            hit_repeat_window_scale: 1.0,
            grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
            grab_hold_per_damage: 0.0,
            grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
            grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
            friendly_fire: true,
            clank_damage_window: 0.0,
            clank_rebound_speed: 0.0,
            edge_cancel_recovery: None,
            special_turn: None,
            special_turn_reverses_drift: None,
            sudden_death_damage: None,
        });
        assert_eq!(
            ResolvedCombatTuning::resolve(declared, 0.12, false).di_max_angle,
            0.30
        );
        assert_eq!(
            ResolvedCombatTuning::resolve(None, 0.12, false),
            ResolvedCombatTuning {
                di_max_angle: 0.12,
                // An undeclared world barks on every hit.
                bark_chance: 1.0,
                ledge_trump_pop: 0.0,
                ledge_occupancy: LedgeOccupancy::Trump,
                double_jump_cancel: false,
                knockback_growth: 0.0,
                downward_hit: DownwardHitStyle::Pogo,
                meteor_lock_time: 0.0,
                rage_per_damage: 0.0,
                rage_max_scale: 1.0,
                stale_step: 0.0,
                stale_floor: 1.0,
                crouch_cancel_scale: 1.0,
                hit_repeat_window_scale: 1.0,
                grab_hold_base_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_hold_per_damage: 0.0,
                grab_hold_max_seconds: FLAT_GRAB_HOLD_SECONDS,
                grab_mash_seconds: FLAT_GRAB_MASH_SECONDS,
                friendly_fire: false,
                clank_damage_window: 0.0,
                clank_rebound_speed: 0.0,
                // ⛔ AN UNDECLARED WORLD KEEPS ITS LAG. Nothing outside a
                // platform fighter was tuned expecting recovery to vanish at a
                // ledge — the same reasoning `clank_damage_window` states above.
                edge_cancel_recovery: false,
                // ⛔ AN UNDECLARED WORLD'S SPECIALS COME OUT THE WAY IT FACES.
                special_turn: false,
                special_turn_reverses_drift: false,
                sudden_death_damage: None,
            }
        );
    }
}

#[cfg(test)]
mod rage_tests {
    use super::*;

    fn raging(per_damage: f32, max_scale: f32) -> ResolvedCombatTuning {
        ResolvedCombatTuning {
            rage_per_damage: per_damage,
            rage_max_scale: max_scale,
            ..ResolvedCombatTuning::default()
        }
    }

    /// A LOSING FIGHTER HITS HARDER, UP TO A CEILING.
    ///
    /// the reason rage exists at all: a body already scales the knockback it
    /// TAKES by its own damage, so without this the fighter behind is punished
    /// twice — easier to launch and no harder to launch with. And the cap is not
    /// decoration: uncapped, the last stock stops being a fight.
    #[test]
    fn rage_grows_with_the_attackers_own_damage_and_stops_at_the_cap() {
        let rules = raging(0.01, 1.5);
        assert_eq!(rules.rage_scale(0), 1.0, "a fresh fighter got a bonus");
        assert_eq!(rules.rage_scale(50), 1.5);
        assert!(rules.rage_scale(20) > 1.0 && rules.rage_scale(20) < 1.5);
        assert_eq!(
            rules.rage_scale(500),
            1.5,
            "rage ran past its ceiling, so the last stock is a coin flip"
        );
        assert_eq!(rules.rage_scale(-7), 1.0, "healed below zero paid a bonus");
    }

    /// AND A GAME THAT DECLARES NO RAGE NEVER GETS ANY.
    ///
    /// the floor that keeps Ambition's PvE unchanged: the baseline declares
    /// `0.0`, and a rate of zero must be exactly `1.0` however hurt the attacker
    /// is — not `1.0 + 0.0 * n` rounded, but the early return.
    #[test]
    fn an_undeclared_world_has_no_rage() {
        let plain = ResolvedCombatTuning::default();
        assert_eq!(plain.rage_per_damage, 0.0);
        for damage in [0, 1, 50, 999] {
            assert_eq!(plain.rage_scale(damage), 1.0);
        }
        // and a rate with a ceiling of 1.0 cannot help either, whatever the
        // rate says — the cap is the authority.
        assert_eq!(raging(0.05, 1.0).rage_scale(200), 1.0);
    }
}

#[cfg(test)]
mod stale_tests {
    use super::*;

    fn staling(step: f32, floor: f32) -> ResolvedCombatTuning {
        ResolvedCombatTuning {
            stale_step: step,
            stale_floor: floor,
            ..ResolvedCombatTuning::default()
        }
    }

    /// A MOVE THROWN AGAIN AND AGAIN IS WORTH LESS, DOWN TO A FLOOR.
    #[test]
    fn staling_falls_with_repetition_and_stops_at_the_floor() {
        let rules = staling(0.1, 0.5);
        assert_eq!(rules.stale_scale(0), 1.0, "a fresh move was already stale");
        assert!((rules.stale_scale(1) - 0.9).abs() < 1e-6);
        assert!((rules.stale_scale(3) - 0.7).abs() < 1e-6);
        assert_eq!(
            rules.stale_scale(9),
            0.5,
            "staling ran past its floor, so a worn move stops being a move"
        );
    }

    /// AND AN UNDECLARED WORLD NEVER STALES ANYTHING.
    #[test]
    fn an_undeclared_world_has_no_staling() {
        let plain = ResolvedCombatTuning::default();
        assert_eq!(plain.stale_step, 0.0);
        for n in [0, 1, 5, 9] {
            assert_eq!(plain.stale_scale(n), 1.0);
        }
        // A floor of 1.0 cannot weaken anything either, whatever the step says.
        assert_eq!(staling(0.2, 1.0).stale_scale(9), 1.0);
    }
}
