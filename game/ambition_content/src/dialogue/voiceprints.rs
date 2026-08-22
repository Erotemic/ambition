//! Ambition's authored dialogue cast voiceprints.
//!
//! The matching vocabulary and cue identities are game content. The reusable
//! dialogue runtime only provides the open [`ambition_dialog::DialogueVoiceCatalog`]
//! and generic fallback behavior.

use ambition_dialog::DialogueVoiceCatalogAppExt;
use ambition_sfx::SfxId;
use bevy::prelude::*;

struct VoiceprintDef {
    cue: SfxId,
    exact: &'static [&'static str],
    aliases: &'static [&'static str],
}

const ALICE: SfxId = SfxId::from_static("dialogue.blip.alice");
const ARCHITECT: SfxId = SfxId::from_static("dialogue.blip.architect");
const BOB: SfxId = SfxId::from_static("dialogue.blip.bob");
const CREATOR: SfxId = SfxId::from_static("dialogue.blip.creator");
const DARK_LORD: SfxId = SfxId::from_static("dialogue.blip.dark_lord");
const GATE_JANITOR: SfxId = SfxId::from_static("dialogue.blip.gate_janitor");
const GOBLIN_CHIEFTAIN: SfxId = SfxId::from_static("dialogue.blip.goblin_chieftain");
const HAND_SAINT: SfxId = SfxId::from_static("dialogue.blip.hand_saint");
const KERNEL_GUIDE: SfxId = SfxId::from_static("dialogue.blip.kernel_guide");
const MANIFEST_CLERK: SfxId = SfxId::from_static("dialogue.blip.manifest_clerk");
const MERCHANT_PROTOTYPE: SfxId = SfxId::from_static("dialogue.blip.merchant_prototype");
const MILITARY_GENERAL: SfxId = SfxId::from_static("dialogue.blip.military_general");
const NEWS_BOARD: SfxId = SfxId::from_static("dialogue.blip.news_board");
const NINJA: SfxId = SfxId::from_static("dialogue.blip.ninja");
const OILER: SfxId = SfxId::from_static("dialogue.blip.oiler");
const PIRATE: SfxId = SfxId::from_static("dialogue.blip.pirate");
const PULSE_VOYAGER: SfxId = SfxId::from_static("dialogue.blip.pulse_voyager");
const ROBOT: SfxId = SfxId::from_static("dialogue.blip.robot");
const TECH_BRO: SfxId = SfxId::from_static("dialogue.blip.tech_bro");
const VAULT_KEEPER: SfxId = SfxId::from_static("dialogue.blip.vault_keeper");
const WEIRD_HERMIT: SfxId = SfxId::from_static("dialogue.blip.weird_hermit");

const VOICEPRINTS: &[VoiceprintDef] = &[
    VoiceprintDef {
        cue: ALICE,
        exact: &["alice", "hall_npc_alice"],
        aliases: &["alice"],
    },
    VoiceprintDef {
        cue: ARCHITECT,
        exact: &["architect", "architect_npc", "hall_architect"],
        aliases: &["architect"],
    },
    VoiceprintDef {
        cue: BOB,
        exact: &["bob", "hall_npc_bob"],
        aliases: &["bob"],
    },
    VoiceprintDef {
        cue: CREATOR,
        // the tests here cannot tell: a probe that deleted the `creator`
        // EXACT key as well left all three voiceprint tests green, because the
        // `creator` alias matches any key CONTAINING it. So for this cast the
        // exact keys are largely redundant with the alias, and nothing would
        // notice one being wrong — worth knowing before trusting a green run
        // over this table.
        exact: &["creator", "hall_npc_creator"],
        aliases: &["creator"],
    },
    VoiceprintDef {
        cue: DARK_LORD,
        exact: &["dark_lord", "hall_npc_dark_lord"],
        aliases: &["dark_lord"],
    },
    VoiceprintDef {
        cue: GATE_JANITOR,
        exact: &["gate_janitor", "hall_npc_gate_janitor"],
        aliases: &["gate_janitor"],
    },
    VoiceprintDef {
        cue: GOBLIN_CHIEFTAIN,
        exact: &["goblin_chieftain", "fretjaw", "fretjaw_cantina_chieftain"],
        aliases: &["goblin", "chieftain", "fretjaw"],
    },
    VoiceprintDef {
        cue: HAND_SAINT,
        exact: &["hand_saint"],
        aliases: &["hand_saint"],
    },
    VoiceprintDef {
        cue: KERNEL_GUIDE,
        exact: &["kernel_guide", "kernel_guide_npc", "hall_npc_kernel_guide"],
        aliases: &["kernel"],
    },
    VoiceprintDef {
        cue: MANIFEST_CLERK,
        exact: &["manifest_clerk", "hall_npc_manifest_clerk"],
        aliases: &["manifest_clerk"],
    },
    VoiceprintDef {
        cue: MERCHANT_PROTOTYPE,
        exact: &[
            "merchant_prototype",
            "merchant_prototype_npc",
            "hall_npc_merchant_prototype",
        ],
        aliases: &["merchant"],
    },
    VoiceprintDef {
        cue: MILITARY_GENERAL,
        exact: &[
            "military_general",
            "general",
            "general_hero",
            "hall_npc_general",
        ],
        aliases: &["general"],
    },
    VoiceprintDef {
        cue: NEWS_BOARD,
        exact: &["news_board", "drain_market_bulletin"],
        aliases: &["news", "bulletin"],
    },
    VoiceprintDef {
        cue: NINJA,
        exact: &["ninja"],
        aliases: &["ninja", "shadow", "oni", "duelist"],
    },
    VoiceprintDef {
        cue: OILER,
        exact: &["oiler", "hall_npc_oiler"],
        aliases: &["oiler"],
    },
    VoiceprintDef {
        cue: PIRATE,
        exact: &["pirate", "pirate_admiral", "admiral", "hall_pirate_admiral"],
        aliases: &[
            "pirate",
            "admiral",
            "quartermaster",
            "navigator",
            "lookout",
            "raider",
            "broadside",
            "iron_mary",
            "salt_annet",
        ],
    },
    VoiceprintDef {
        cue: PULSE_VOYAGER,
        exact: &[
            "pulse_voyager",
            "captain_pulse",
            "hall_npc_pulse_voyager_captain",
        ],
        aliases: &["pulse"],
    },
    VoiceprintDef {
        cue: ROBOT,
        exact: &["robot", "hall_robot"],
        aliases: &["robot", "automaton", "synthetic", "smart_house"],
    },
    VoiceprintDef {
        cue: TECH_BRO,
        exact: &["tech_bro", "chadwick_iii", "chadwick_disruptor_iii"],
        aliases: &["tech", "chadwick"],
    },
    VoiceprintDef {
        cue: VAULT_KEEPER,
        exact: &["vault_keeper", "vault_keeper_npc", "hall_npc_vault_keeper"],
        aliases: &["vault_keeper"],
    },
    VoiceprintDef {
        cue: WEIRD_HERMIT,
        exact: &["weird_hermit"],
        aliases: &["weird_hermit", "hermit"],
    },
];

pub(super) fn register(app: &mut App) {
    for voiceprint in VOICEPRINTS {
        app.register_dialogue_voiceprint(voiceprint.cue, voiceprint.exact, voiceprint.aliases);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_dialog::DialogueVoiceCatalog;

    fn catalog() -> DialogueVoiceCatalog {
        let mut app = App::new();
        register(&mut app);
        app.world_mut()
            .remove_resource::<DialogueVoiceCatalog>()
            .unwrap()
    }

    #[test]
    fn flagship_exact_and_alias_names_keep_their_authored_voiceprints() {
        let catalog = catalog();
        assert_eq!(catalog.resolve("Bob", ""), Some(BOB));
        assert_eq!(catalog.resolve("Shadow Oni Leader", ""), Some(NINJA));
        assert_eq!(catalog.resolve("", "hall_npc_oiler"), Some(OILER));
        assert_eq!(catalog.resolve("Iron Mary", ""), Some(PIRATE));
    }

    #[test]
    fn every_authored_key_resolves_to_its_declared_cue() {
        let catalog = catalog();
        for voiceprint in VOICEPRINTS {
            for exact in voiceprint.exact {
                assert_eq!(
                    catalog.resolve(exact, ""),
                    Some(voiceprint.cue),
                    "exact voice key {exact:?} drifted",
                );
            }
            for alias in voiceprint.aliases {
                let decorated = format!("speaker_{alias}_variant");
                assert_eq!(
                    catalog.resolve(&decorated, ""),
                    Some(voiceprint.cue),
                    "voice alias {alias:?} drifted",
                );
            }
        }
    }

    /// No exact key is hiding an alias that would steal it.
    ///
    /// `exact` is consulted BEFORE `aliases`, so an exact key silently masks
    /// a collision. If one cue's alias is a substring of another cue's
    /// authored name, the authored name still resolves correctly — through the
    /// exact map — while every *undeclared* speaker label shaped like it goes to
    /// the wrong voice. `every_authored_key_resolves_to_its_declared_cue` cannot
    /// see that: it only ever asks about keys the exact map already answers.
    ///
    /// So this asks the same question with the exact map REMOVED. Every key must
    /// still land on its own cue through aliases alone.
    ///
    /// a failure here does not mean "add an exact key" — that is what
    /// hides it. It means two cues have overlapping alias vocabulary, and one of
    /// them needs a longer alias.
    #[test]
    fn no_exact_key_is_masking_an_alias_that_would_steal_it() {
        let mut app = App::new();
        for voiceprint in VOICEPRINTS {
            app.register_dialogue_voiceprint(voiceprint.cue, &[], voiceprint.aliases);
        }
        let alias_only = app
            .world_mut()
            .remove_resource::<DialogueVoiceCatalog>()
            .unwrap();

        let mut masked: Vec<String> = Vec::new();
        for voiceprint in VOICEPRINTS {
            for exact in voiceprint.exact {
                match alias_only.resolve(exact, "") {
                    Some(cue) if cue == voiceprint.cue => {}
                    other => masked.push(format!(
                        "{exact:?} is authored for {:?} but aliases alone answer {other:?}",
                        voiceprint.cue
                    )),
                }
            }
        }
        assert!(
            masked.is_empty(),
            "{} exact key(s) are the only thing routing a name correctly, which \
             means an alias belonging to another cue matches them — every \
             UNDECLARED speaker shaped like these goes to the wrong voice:\n  {}",
            masked.len(),
            masked.join("\n  ")
        );
    }

    #[test]
    fn registration_is_idempotent_for_repeated_content_plugin_installation() {
        let mut app = App::new();
        register(&mut app);
        register(&mut app);
        assert_eq!(
            app.world()
                .resource::<DialogueVoiceCatalog>()
                .resolve("Alice", ""),
            Some(ALICE),
        );
    }
}
