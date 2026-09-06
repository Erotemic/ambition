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
#[derive(Clone, Debug, Default, PartialEq)]
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
    /// the damage resolver (`damage_apply`), which decides whether a hit kills
    /// at all, so it sits one step before the other four rather than beside
    /// them. Grouped here because it is the same kind of authored character
    /// fact and has the same one consumer family; if this struct ever grows a
    /// second mortality knob, that is the moment to split them.
    pub never_dies: bool,
    /// Whether the body drops its live held item on death.
    pub drops_held_item: bool,
}
