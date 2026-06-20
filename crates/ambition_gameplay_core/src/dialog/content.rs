//! Dialogue content types — minimal post-Yarn migration.
//!
//! Authored conversation content lives in
//! `assets/dialogue/sandbox/*.yarn` and is loaded by
//! `bevy_yarnspinner`. The runtime types here cover only what the
//! UI view-model + LDtk validator still need:
//!
//! - [`DialogChoice`] — the runtime option representation written
//!   by the Yarn bridge into [`crate::dialog::DialogState.current_options`]
//!   and rendered by `sync_dialog_ui`. `next_node` / `note` /
//!   `close_after` are vestigial fields kept for the existing UI
//!   layout code (the renderer reads `label` and ignores the rest);
//!   they're set to `None` / `false` by the bridge.
//! - [`KNOWN_DIALOGUE_IDS`] — the canonical list of Yarn node ids
//!   `NpcSpawn.dialogue_id` may reference. The LDtk content
//!   validator uses this to flag typos.
//!
//! Adding a dialogue: edit the matching `.yarn` zone file, and
//! add the new id to [`KNOWN_DIALOGUE_IDS`].
//!
//! The pre-migration `DialogTree` / `DialogNode` / `DialogRedirectRule`
//! / `DialogRegistry` types and the RON registry loader have been
//! retired. Boss-cleared / flag-set redirects are now inline
//! `<<if boss_cleared("x")>>` branches inside the `.yarn` files.

/// Option emitted by Yarn's `PresentOptions` event, in the shape
/// the existing UI renderer expects. The bridge fills `label` from
/// the Yarn option's text; the other fields are vestigial.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DialogChoice {
    pub label: String,
    /// Vestigial — was the RON-era "next tree-node index". Yarn
    /// dispatches via `OptionId`, stored on `DialogState.yarn_option_ids`.
    pub next_node: Option<usize>,
    /// Vestigial — was the RON-era "system aside text after picking
    /// this option". Inline Yarn lines now carry asides directly.
    pub note: Option<String>,
    /// Vestigial — was the RON-era "this option closes the dialog".
    /// Yarn's runner reports closure via `DialogueCompleted`.
    pub close_after: bool,
}

/// Canonical list of Yarn node ids the LDtk `NpcSpawn.dialogue_id`
/// field may reference. Mirrors the `title:` headers in
/// `assets/dialogue/sandbox/*.yarn`. The content validator checks
/// authored ids against this list so typos surface at load time
/// instead of as silent fallbacks.
pub const KNOWN_DIALOGUE_IDS: &[&str] = &[
    // intro.yarn
    "alice_after_bob_survey",
    "alice_intro_stub",
    "bob_after_report",
    "bob_intro_stub",
    "creator_final_fast",
    "creator_final_impossible",
    "creator_final_normal",
    "creator_intro",
    "gate_janitor_ripple",
    "intro_lab_raider",
    "intro_salvage_guard",
    "manifest_kiosk_wrong_list",
    "news_board_lab_incident",
    "oiler_intro",
    "oiler_post_stabilizer",
    // kernel.yarn
    "architect_intro",
    "generic_npc",
    "hub_guide",
    "smirking_behemoth_victory_npc",
    "merchant_seed",
    "vault_keeper",
    // factions.yarn
    "goblin_cantina_chieftain",
    "military_general",
    "pulse_voyager_captain",
    "tech_bros_disruptor",
    // cove.yarn
    "pirate_admiral",
    "pirate_admiral_after_treasure",
    "pirate_heavy_broadside_bess",
    "pirate_heavy_iron_mary",
    "pirate_heavy_salt_annet",
    "pirate_lookout",
    "pirate_navigator",
    "pirate_quartermaster",
    "pirate_raider",
    "pirate_raider_after_treasure",
    "parrot_cove",
    // dojo.yarn
    "ninja_duelist",
    "ninja_leader",
    // symmetry.yarn — four C4-symmetric Emmy clones, one per kernel face.
    "emmy_noether",
    "emmy_noether_left",
    "emmy_noether_up",
    "emmy_noether_right",
];

/// Validator surface (LDtk content_validation reads this).
pub fn known_dialogue_ids() -> Vec<&'static str> {
    KNOWN_DIALOGUE_IDS.to_vec()
}
