//! THE SMASH BRAIN'S DATA, and it is data because the ORPHAN RULE says so.
//!
//! ⛔⛔ `Brain`'s `SnapshotState` impl is bound to this crate — `ambition_characters`
//! owns `Brain` — and `ambition_combat` DEPENDS on this crate, so this crate can
//! never name combat. Every type the Brain encoder reads is therefore pinned
//! here and cannot follow the behaviour up, however the behaviour is carved.
//! `snapshot_impls.rs` reads `SmashState`, `BroadMode` and `ObsHistory`, so those
//! three are the pinned set for this subtree.
//!
//! ⚠ AND THE SENTENCE ABOVE IS DELIBERATELY NOT SPELLED THE OBVIOUS WAY.
//! `check_absence_contracts.py` finds encoded types by regexing raw source for
//! the impl header that binds the snapshot trait to a type, and it does NOT strip
//! comments — so writing that header in PROSE invents a wire-format entry and
//! turns the contract red. It cost two debugging rounds here, once for the real
//! phrase and once for the version with a placeholder type. Describe the impl;
//! never spell its header.
//!
//! ⭐ THE MODULE EXISTS TO BE PROVED, not to be tidy. If anything below reaches
//! into this subtree's behaviour — `tick_smash`, `choose_action`, `observe`,
//! `apply_difficulty` — this file stops compiling, which is the whole point: the
//! split that lets the behaviour leave has to be checkable, and D168's estimate
//! of it ("253 lines") was not.
//!
//! ⭐ `BroadMode` and `DifficultyProfile` joined them here, so the pinned set for
//! this subtree is COMPLETE in one module: `choose_mode` and `apply_difficulty`
//! stay beside their own behaviour and import the shapes from here, which is the
//! direction that lets the behaviour leave later.

use ambition_platformer2d_core as ae;

/// Tuning knobs for a [`crate::brain::StateMachineCfg::Smash`] brain. Per-actor
/// state lives in [`SmashState`]. Designer-facing today — eventually
/// migrates to data so per-archetype variants live in
/// `character_archetypes.ron`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmashCfg {
    /// Maximum sensing distance (px). Outside this radius the brain
    /// idles regardless of target presence.
    pub aggro_radius: f32,
    /// Distance the brain tries to settle at while in `Engage`.
    /// Slightly outside `attack_range` so the actor has room to
    /// burst forward into an attack.
    pub engage_distance: f32,
    /// Concrete melee attack range (px). When the target is closer
    /// than this AND the actor has melee capability, `Engage` emits
    /// a melee attempt. Authoritative — replaces the old hardcoded
    /// melee-engage range.
    pub attack_range: f32,
    /// Distance below which the actor retreats to avoid being
    /// pinned against a wall by the target.
    pub too_close_distance: f32,
    /// Minimum *upward* gap (px) to the target before the actor jumps to
    /// pursue it vertically. The grunt default (`60`) chases any target a
    /// short hop above; a duelist sets this above a hop's apex so it
    /// only climbs after a target genuinely standing on a platform, instead
    /// of leapfrogging an opponent that is merely mid-hop (the flat-ground
    /// air-juggle cascade). Replaces the former hardcoded threshold.
    pub vertical_chase_min: f32,
    /// Movement speed while in Approach / Chase (px/s).
    pub chase_speed: f32,
    /// Movement speed while in Retreat / Reposition (px/s).
    pub retreat_speed: f32,
    /// Crowding pressure (from same-faction allies) that triggers
    /// `Reposition` mode. `0.0` disables.
    pub crowding_threshold: f32,
    /// When true, the actor commits a [`SpecificAction::Sprint`] — full
    /// locomotion throttle — to close a *large* approach gap (on a cadence)
    /// instead of only walking: a more aggressive, dynamic chase. Off by
    /// default; enabled per archetype (goblins) so it doesn't silently change
    /// every melee enemy's feel.
    ///
    /// it was `dash_to_close` and it named a CAPABILITY the body may not have.
    pub sprint_to_close: bool,
    /// Neutral-game footsies: amplitude (px) the actor weaves IN and OUT
    /// around [`Self::engage_distance`] while spacing against a live
    /// opponent. `0.0` (the grunt default) disables the weave entirely —
    /// the actor closes and holds like before. A positive value makes a
    /// *duelist*: it dips into poke range on a rhythm, then backs out to
    /// bait a whiff, instead of camping point-blank and mashing. Uses only
    /// the target-relative distance, so it's frame-agnostic.
    pub footsies_amplitude: f32,
    /// Seconds per full in→out footsies cycle. Ignored when
    /// [`Self::footsies_amplitude`] is `0.0`.
    pub footsies_period_s: f32,
    /// Minimum seconds between *neutral hops* — short jumps the duelist
    /// mixes into its approach to vary its attack vector and use vertical
    /// space. `0.0` (the grunt default) disables neutral hops.
    pub neutral_jump_cadence_s: f32,
    /// Regroup trigger: accumulated recent damage (as a fraction of max HP,
    /// decaying over a couple seconds) that makes the fighter break off and reset
    /// after taking a beating — instead of trading hits forever at one spacing. It
    /// retreats a real distance (sprinting to cover ground, taking to the air for high
    /// ground if it can fly), then re-engages. `0.0` (the grunt default) disables it.
    pub regroup_damage_threshold: f32,
    /// How long a regroup lasts (s) before the fighter returns to neutral. Ignored
    /// when [`Self::regroup_damage_threshold`] is `0.0`.
    pub regroup_duration_s: f32,
    /// Target separation (px) a regroup opens up: once the fighter has backed off at
    /// least this far it has "regrouped" and re-engages early. Large enough that the
    /// retreat — and the re-approach — cross a real gap (so the sprint/fly traversal
    /// actually fires). Ignored when regroup is disabled.
    pub regroup_distance: f32,
    /// Poke-and-reset discipline (whiff-punish footsies). `0.0` (the grunt default)
    /// disables it: the actor stays in range and re-swings the instant its cooldown clears
    /// (point-blank mashing). A positive value makes a real neutral game — poke, reset,
    /// re-approach — instead of two bodies glued together trading hits. Frame-agnostic (uses
    /// only target-relative spacing); the in/out weave, not a forced retreat, does the spacing
    /// so a cornered fighter never pins itself against a wall.
    pub poke_reset_s: f32,
    /// When true, the fighter may blink-evade a fast-closing opponent (a
    /// perceivable lunge, read from the lagged target history — never from a
    /// privileged attack flag). Capability gate only: the body still needs the
    /// blink ability for the emitted intent to resolve, exactly like the player.
    /// `false` for grunts.
    pub can_blink: bool,
    /// Minimum seconds between blink-evades. Ignored when [`Self::can_blink`]
    /// is `false`.
    pub blink_cooldown_s: f32,
    /// `[0, 1]` — probability the fighter actually reacts to a perceived threat
    /// this tick (blink or block). `< 1.0` models imperfect defense: it does NOT
    /// block every swing, so some attacks land and the fight never turtles into a
    /// stalemate. `0.0` (the grunt default) disables reactive defense entirely.
    /// This is the "no perfect reactions" knob layered on top of `reaction_delay_s`
    /// (which already makes it perceive the lunge late).
    pub defense_reactivity: f32,
    /// Perceived closing speed (px/s, toward the fighter) at or above which a
    /// threat is met with a blink (the mobile evade for a committed lunge /
    /// sprint-in). Below it — but above [`Self::shield_closing_speed`] — the fighter
    /// blocks instead (the stand-ground option for ordinary approach pressure).
    /// Splitting the two is what gives the layered, readable defensive game.
    pub blink_closing_speed: f32,
    /// Minimum perceived closing speed (px/s) that counts as a threat worth
    /// reacting to at all. Above a slow drift, below a walk-in so the fighter
    /// guards an opponent stepping into poke range.
    pub shield_closing_speed: f32,
    /// When true, the fighter may reactive-block a perceived lunge it can't
    /// or won't blink away from — it raises `shield_held` and stands its ground
    /// for a short window. Layered defense: blink is the mobile option, block the
    /// stand-ground one. `false` for grunts.
    pub can_shield: bool,
    /// Whether blocking requires the ground — the GAME's rule, not this
    /// brain's.
    ///
    /// Smash Siblings wants that rule; another game on this engine may not, and answering it
    /// meant editing the brain.
    ///
    /// defaults to `true` everywhere, so nothing changes by accident — the
    /// lift is deliberately behaviour-preserving. Whether AMBITION wants airborne
    /// blocking is a separate product question and stays open
    /// (`awaiting-maintainer-decision.md` #9, "Can a flying fighter shield?").
    /// a game answers it by AUTHORING now, and a duel fixture can state which
    /// rule it is testing under instead of inheriting one silently.
    pub shield_requires_ground: bool,
    /// When true, this is a hybrid flyer: a body that can both fight grounded
    /// (footsies + jump) and take flight (`fly_toggle_pressed`). The brain decides
    /// when to be airborne — to contest an elevated target, or to mount a proactive
    /// aerial foray — and lands again to footsie. `false` = the body never toggles
    /// (a pure grounded brawler, or a pure flyer driven by its `actor_aerial`
    /// body state). Capability gate: the body still needs the fly ability for the
    /// toggle intent to resolve, like the player.
    pub can_fly: bool,
    /// Hybrid flyer: seconds spent grounded between proactive aerial forays.
    /// Ignored unless [`Self::can_fly`].
    pub aerial_foray_cadence_s: f32,
    /// Hybrid flyer: seconds an aerial foray lasts before landing again.
    /// Ignored unless [`Self::can_fly`].
    pub aerial_foray_duration_s: f32,
    /// Relentless engagement: when true, the fighter never disengages while its
    /// foe lives — beyond [`Self::aggro_radius`] it CHASES (Approach) instead of
    /// idling out. This is the committed-duelist property: a platform-fighter
    /// opponent pursues across the whole stage and re-acquires after the player
    /// flings it away with gravity, rather than going inert at distance. `false`
    /// (the grunt default) keeps ambient enemies idling once the player leaves.
    pub relentless: bool,
    /// Stale-fight re-aggression: seconds of the fighter's own offense-drought
    /// (no swing / shot committed) after which it forces an offensive push —
    /// suppressing its reactive defense and neutral-game patience to close and
    /// attack, the way two platform-fighter players break a passive standoff. Resets
    /// whenever it attacks, so it only fires during a genuine lull, never mid-trade.
    /// `0.0` (the grunt default) disables it.
    pub stale_fight_s: f32,
    /// Difficulty profile applied at stage 4.
    pub difficulty: DifficultyProfile,
}

impl SmashCfg {
    /// "Standard melee striker" tuning — humanoid grunt that
    /// approaches, swings, and steps back. Used by MediumStriker,
    /// SmallSkitter, SmallLurker, PirateRaider.
    pub const STRIKER_DEFAULT: Self = Self {
        aggro_radius: 460.0,
        engage_distance: 70.0,
        attack_range: 56.0,
        too_close_distance: 30.0,
        vertical_chase_min: 60.0,
        chase_speed: 170.0,
        retreat_speed: 130.0,
        crowding_threshold: 0.65,
        sprint_to_close: false,
        // Grunts don't play footsies — they close and hold. The neutral game
        // is opt-in (duelists / bosses) so this doesn't change every enemy.
        footsies_amplitude: 0.0,
        footsies_period_s: 1.4,
        neutral_jump_cadence_s: 0.0,
        regroup_damage_threshold: 0.0,
        regroup_duration_s: 0.0,
        regroup_distance: 0.0,
        poke_reset_s: 0.0,
        can_blink: false,
        blink_cooldown_s: 0.0,
        defense_reactivity: 0.0,
        blink_closing_speed: 175.0,
        shield_closing_speed: 175.0,
        can_shield: false,
        // Smash's rule, and the default everywhere: no blocking in mid-air.
        shield_requires_ground: true,
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
        // Ambient grunt: idles out when the player leaves; no stale-fight push.
        relentless: false,
        stale_fight_s: 0.0,
        difficulty: DifficultyProfile::MEDIUM,
    };
    /// Heavy brute tuning — slower, longer reach, less retreat.
    pub const BRUTE_DEFAULT: Self = Self {
        aggro_radius: 380.0,
        engage_distance: 90.0,
        attack_range: 70.0,
        too_close_distance: 24.0,
        vertical_chase_min: 60.0,
        chase_speed: 118.0,
        retreat_speed: 80.0,
        crowding_threshold: 0.55,
        sprint_to_close: false,
        footsies_amplitude: 0.0,
        footsies_period_s: 1.6,
        neutral_jump_cadence_s: 0.0,
        regroup_damage_threshold: 0.0,
        regroup_duration_s: 0.0,
        regroup_distance: 0.0,
        poke_reset_s: 0.0,
        can_blink: false,
        blink_cooldown_s: 0.0,
        defense_reactivity: 0.0,
        blink_closing_speed: 175.0,
        shield_closing_speed: 175.0,
        can_shield: false,
        // Smash's rule, and the default everywhere: no blocking in mid-air.
        shield_requires_ground: true,
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
        relentless: false,
        stale_fight_s: 0.0,
        difficulty: DifficultyProfile::MEDIUM,
    };
    /// Duelist tuning — a 1v1 fighter with a real neutral game: it weaves
    /// in and out of poke range (footsies), mixes in neutral hops, and sprints
    /// to close large gaps. Aware of the whole arena (large aggro). This is the
    /// base the Perfect Cell-ular Automaton and other "platform fighter"
    /// opponents build on; grunts stay on [`Self::STRIKER_DEFAULT`].
    pub const DUELIST_DEFAULT: Self = Self {
        aggro_radius: 1100.0,
        engage_distance: 78.0,
        attack_range: 56.0,
        too_close_distance: 30.0,
        vertical_chase_min: 140.0,
        chase_speed: 200.0,
        retreat_speed: 175.0,
        crowding_threshold: 0.65,
        sprint_to_close: true,
        footsies_amplitude: 60.0,
        footsies_period_s: 1.3,
        neutral_jump_cadence_s: 1.7,
        // After taking ~5% of max HP since the last break-off (a few clean hits),
        // regroup: sprint/fly out to a real distance, then re-engage — spatial depth
        // instead of a glued trade.
        regroup_damage_threshold: 0.05,
        regroup_duration_s: 1.6,
        regroup_distance: 300.0,
        // After every poke, suppress offense and let the weave reset spacing before
        // re-committing — the heart of the neutral game (no point-blank mashing).
        poke_reset_s: 0.38,
        can_blink: true,
        blink_cooldown_s: 1.1,
        // A real defensive game: react to ~60% of perceived threats (imperfect —
        // some hits land), blink a committed lunge (≥230 px/s closing), block the
        // ordinary walk-in pressure (≥70 px/s).
        defense_reactivity: 0.6,
        blink_closing_speed: 230.0,
        shield_closing_speed: 70.0,
        can_shield: true,
        // Smash's rule, and the default everywhere: no blocking in mid-air.
        shield_requires_ground: true,
        // Grounded duelist by default; hybrid flight is opt-in per fighter.
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
        // A committed 1v1 fighter: chases its foe across the whole stage (never idles
        // out at distance) and, after ~2.5 s of its own inaction, forces an offensive
        // push so the bout never stalls into a passive standoff.
        relentless: true,
        stale_fight_s: 2.5,
        // A fair human reaction lag (no frame-perfect counters). Competence is
        // expressed through the neutral game + layered defense above, NOT through
        // crisper reactions: a twitchier profile locks the fight into a shielding
        // standoff (the arena non-degeneracy harness catches it).
        difficulty: DifficultyProfile::MEDIUM,
    };
}

/// Number of opponent-position samples retained for reaction latency.
/// At 60 fps this is ~0.53 s of history — comfortably longer than any
/// authored `reaction_delay_s` (EASY = 0.30 s), so the delayed lookup is
/// always covered once the buffer fills.
pub const OBS_HISTORY_LEN: usize = 32;

/// The actor's OWN state is never delayed (you always know where you are). Pure function of the
/// tick stream → replay-safe and deterministic. `Copy` so `SmashState` stays `Copy`.
#[derive(Clone, Copy, Debug)]
pub struct ObsHistory {
    samples: [(f32, ae::Vec2); OBS_HISTORY_LEN],
    /// Next write index (ring).
    write: usize,
    /// Number of valid samples (saturates at `OBS_HISTORY_LEN`).
    count: usize,
}

impl Default for ObsHistory {
    fn default() -> Self {
        Self {
            samples: [(0.0, ae::Vec2::ZERO); OBS_HISTORY_LEN],
            write: 0,
            count: 0,
        }
    }
}

impl ObsHistory {
    /// Record this tick's observed opponent position.
    pub(super) fn push(&mut self, sim_time: f32, target_pos: ae::Vec2) {
        self.samples[self.write] = (sim_time, target_pos);
        self.write = (self.write + 1) % OBS_HISTORY_LEN;
        self.count = (self.count + 1).min(OBS_HISTORY_LEN);
    }

    /// Stable snapshot view of the reaction-history ring. The brain crate owns the
    /// representation; rollback consumers receive the exact values without exposing
    /// the fields for arbitrary mutation.
    pub fn snapshot_parts(&self) -> (&[(f32, ae::Vec2); OBS_HISTORY_LEN], usize, usize) {
        (&self.samples, self.write, self.count)
    }

    /// Restore the reaction-history ring from a validated snapshot cursor.
    pub fn restore_snapshot_parts(
        &mut self,
        samples: [(f32, ae::Vec2); OBS_HISTORY_LEN],
        write: usize,
        count: usize,
    ) -> Option<()> {
        if write >= OBS_HISTORY_LEN || count > OBS_HISTORY_LEN {
            return None;
        }
        self.samples = samples;
        self.write = write;
        self.count = count;
        Some(())
    }

    /// The opponent position the brain is allowed to perceive this tick: the
    /// most recent sample that is at least `delay` seconds old. Never returns
    /// anything newer than `now - delay`, so the brain truly cannot react
    /// faster than its latency. Until the buffer covers the window (fight
    /// start), returns the oldest sample it has; `None` only when no sample has
    /// been recorded yet.
    pub(super) fn delayed(&self, now: f32, delay: f32) -> Option<ae::Vec2> {
        if self.count == 0 {
            return None;
        }
        let target_time = now - delay.max(0.0);
        let mut best_old: Option<(f32, ae::Vec2)> = None; // newest sample <= target_time
        let mut oldest: Option<(f32, ae::Vec2)> = None;
        for i in 0..self.count {
            let s = self.samples[i];
            if oldest.is_none_or(|o| s.0 < o.0) {
                oldest = Some(s);
            }
            if s.0 <= target_time && best_old.is_none_or(|b| s.0 > b.0) {
                best_old = Some(s);
            }
        }
        Some(best_old.or(oldest).map(|s| s.1).unwrap_or(ae::Vec2::ZERO))
    }
}

/// Per-actor runtime state for the Smash brain.
#[derive(Clone, Copy, Debug, Default)]
pub struct SmashState {
    /// Mode active last tick. Used by the hysteresis check in
    /// `choose_mode` so the brain doesn't flip Approach⇄Retreat
    /// when distance hovers at the threshold.
    pub mode: BroadMode,
    /// Seconds the current mode has been active. Incremented each
    /// tick from `snapshot.dt`; reset to 0 on mode change. Compared
    /// against `MODE_MIN_DWELL_S` for hysteresis.
    pub mode_dwell_s: f32,
    /// Random seed for difficulty jitter (commit probability,
    /// reaction delay variance). Set once at first tick from the
    /// actor id; survives reset_to_spawn via spawn-time init.
    pub rng_seed: u64,
    /// Seconds until the actor's *sprint-to-close* commitment is off cadence (only used when
    /// [`SmashCfg::sprint_to_close`]). Brain-side only — the body has no cooldown on running.
    pub sprint_cooldown_remaining: f32,
    /// See [`ObsHistory`].
    pub obs_history: ObsHistory,
    /// Footsies weave phase (radians), advanced each tick when the cfg enables
    /// the neutral game. A per-actor offset (from `rng_seed`) is added at read
    /// time so two duelists desync rather than mirror-lock.
    pub spacing_phase: f32,
    /// Seconds until the next neutral hop is allowed. Decremented each tick;
    /// re-armed to `neutral_jump_cadence_s` when a hop fires.
    pub neutral_jump_cooldown: f32,
    /// Seconds until the next blink-evade is allowed. Decremented each tick;
    /// re-armed to `blink_cooldown_s` when a blink fires.
    pub blink_cooldown: f32,
    /// Hybrid flyer: seconds left in the current ground-dwell or aerial-foray
    /// phase. Drives the proactive take-off/land cadence with hysteresis so the
    /// fighter doesn't chatter the fly toggle every tick.
    pub foray_timer: f32,
    /// Seconds left holding a reactive block. Set when the fighter chooses to
    /// shield a perceived lunge; while positive it keeps `shield_held` up so the
    /// block spans the opponent's attack instead of flickering for one tick.
    pub shield_hold_timer: f32,
    /// Seconds left in the post-poke neutral reset (whiff-punish footsies). Armed
    /// to [`SmashCfg::poke_reset_s`] on the falling edge of a swing; while positive
    /// the fighter suppresses offense and weaves out to its outer spacing pocket
    /// instead of re-swinging point-blank. `0` outside the window / for grunts.
    pub neutral_reset_timer: f32,
    pub was_attacking: bool,
    /// Seconds left in the current regroup (break-off-and-reset after a beating).
    /// While positive the fighter retreats a real distance — sprinting, and taking to
    /// the air for high ground if able — instead of trading. `0` outside a regroup.
    pub regroup_timer: f32,
    /// Own health fraction observed last tick, to detect DROPS (damage taken).
    pub last_health_fraction: f32,
    /// Decaying memory of recent damage taken (sum of health-fraction drops, bled
    /// off over a couple seconds). Arms a regroup when it crosses the threshold.
    pub damage_accum: f32,
    /// Seconds since this fighter last committed an attack (swing or shot). Drives
    /// the stale-fight re-aggression ([`SmashCfg::stale_fight_s`]): once it
    /// exceeds the threshold the fighter forces an offensive push instead of waiting
    /// out a passive standoff. Reset to `0` on every attack, so it only grows during
    /// a genuine lull. Pure tick-stream bookkeeping → replay-safe.
    pub time_since_offense: f32,
}

/// Top-level "what should I do right now" decision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BroadMode {
    /// No active engagement — patrol / wait. Default.
    #[default]
    Idle,
    /// Close distance to the target.
    Approach,
    /// Create distance from the target (player too close).
    Retreat,
    /// In melee/range window — commit an attack.
    Engage,
    /// Anti-clump: too many allies stacked up; sidestep to spread
    /// out. Higher priority than Approach so a swarm visibly fans
    /// out rather than piling on.
    Reposition,
    /// Off-stage / suspended over a gap. Today a stub —
    /// `TerrainAwareness.off_stage` is always false until the
    /// snapshot builder learns about ledges.
    Recover,
}

/// Per-actor difficulty tuning. Authored today via
/// [`SmashCfg::difficulty`]; an upcoming pass lifts this into
/// `character_archetypes.ron` so designers can tune per-archetype
/// without code edits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DifficultyProfile {
    /// Seconds of observation lag on the OPPONENT. `tick_smash` perceives
    /// the opponent as it was this many seconds ago (via
    /// [`super::SmashState::obs_history`]) so the brain can't frame-perfectly
    /// counter. Higher = easier (sees later). Not consumed by the difficulty
    /// filter itself — it shapes perception upstream, in `observe`.
    pub reaction_delay_s: f32,
    /// `[0.0, 1.0]` — probability of committing the chosen action
    /// this tick. Lower = drops more attempts to Idle.
    pub commit_probability: f32,
    /// `[0.0, 1.0]` — `1.0` = no aim jitter; lower values jitter
    /// the attack axis proportionally. Applied to MeleeAttack /
    /// RangedAttack only.
    pub accuracy: f32,
    /// Hz — informational, for downstream cooldown / mashing
    /// systems to consult.
    #[allow(
        dead_code,
        reason = "consumer lives in the EFFECTS-stage cooldown gate"
    )]
    pub mash_speed_hz: f32,
}

impl DifficultyProfile {
    pub const EASY: Self = Self {
        reaction_delay_s: 0.30,
        commit_probability: 0.55,
        accuracy: 0.65,
        mash_speed_hz: 1.0,
    };
    pub const MEDIUM: Self = Self {
        reaction_delay_s: 0.15,
        commit_probability: 0.85,
        accuracy: 0.85,
        mash_speed_hz: 1.4,
    };
    pub const HARD: Self = Self {
        reaction_delay_s: 0.05,
        commit_probability: 0.98,
        accuracy: 0.98,
        mash_speed_hz: 2.0,
    };
}
