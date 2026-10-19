//! Authored death and mortality semantics for a character template.
//!
//! Construction lowers these facts into runtime combat capabilities. Keeping
//! authoring vocabulary here avoids coupling character definitions to runtime
//! ECS types; the authored and runtime schemas may evolve independently.

/// The authored on-death behaviour of a character template.
///
/// Every field defaults to "nothing special", so a character that says nothing
/// about dying gets the ordinary death — which is what almost every character
/// wants and why the whole struct is `Option` on a definition.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CharacterDeathTraits {
    /// Detonates at the corpse on death, so a point-blank kill is punished.
    pub explodes_on_death: bool,
    /// Character id spawned as offspring on death; `None` means no split.
    pub divides_into: Option<String>,
    /// A fast charge stopped dead by a wall destroys this body.
    pub charge_crash_explodes: bool,
    /// Damage never kills — a training dummy with an effectively infinite pool.
    ///
    ///  not an on-death consequence; a MORTALITY policy. Its consumer is
    /// the damage resolver (`ambition_damage::resolve_body_hit`), which decides
    /// whether a hit kills
    /// at all, so it sits one step before the other four rather than beside
    /// them. Grouped here because it is the same kind of authored character
    /// fact and has the same one consumer family; if this struct ever grows a
    /// second mortality knob, that is the moment to split them.
    pub never_dies: bool,
    /// Whether the body drops its live held item on death.
    pub drops_held_item: bool,
    /// The cue this body plays when it dies, as an SFX bank id. `None` plays
    /// the death cue of the game that owns the body.
    pub death_sound: Option<String>,
    /// What this body leaves when its death row has played. `None` leaves
    /// what the game of its room states (`DeclaredRules<Corpse>`). A body that
    /// is taken away by its own death (it explodes) states `Vanish`; it still
    /// plays its whole death row first.
    pub corpse: Option<Corpse>,
}

/// What a dead body leaves once its death row has played.
///
/// Health reaches zero on the tick of the kill, so combat, rewards and
/// progression see the death at once. The body is drawn while its death row
/// plays in every case; this only says what comes after.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Corpse {
    /// The body stays where it fell, drawn on the last frame of its death row,
    /// until its room goes. Ambition's rule (Jon, 2026-10-18: "bodies persist
    /// by default unless they explicitly don't").
    Stay,
    /// The body holds its last frame for a moment, blinks out, and is drawn no
    /// more. The engine's answer in a room whose game states no rule.
    #[default]
    Vanish,
}
