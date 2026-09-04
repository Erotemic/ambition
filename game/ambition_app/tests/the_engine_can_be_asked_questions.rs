//! What can authored content ask this engine, and does asking work?
//!
//! The condition contract's whole claim is that a domain publishes its own
//! questions and nothing central learns they exist. That claim is only worth
//! anything about the composed engine — a contract that works in a hand-built
//! `App` and is wired up nowhere is vocabulary nobody speaks.
//!
//! so this file drives the real host: the real plugin group, the real item
//! domain, the real save layer, a real authored occurrence, and a real pressed
//! pickup. it publishes no condition of its own. The unit tests beside the
//! contract prove a stranger can publish one; this proves the engine actually
//! did.
//!
//! and it is deliberately thin on assertions about WHICH conditions exist.
//! Pinning the full catalog would make every new provider a failing test, which
//! is the opposite of the property being built — new questions are supposed to be
//! cheap. What is pinned is that independent domains are present and that asking
//! them returns real answers about real state.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::platformer::authored_logic::{
    AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome,
};
use ambition_platformer2d::platformer::sim_id::SimId;
use std::path::Path;

use crate::common::{base, fixed_60hz_room_sim};

const ROOM: &str = "blink_run";

fn catalog(sim: &Platformer2dSimHarness) -> ConditionCatalog {
    sim.world()
        .get_resource::<ConditionCatalog>()
        .expect(
            "the composed engine publishes at least one condition, so the catalog resource exists",
        )
        .clone()
}

fn ask(sim: &Platformer2dSimHarness, id: &ConditionId, args: &[AuthoredArg]) -> ConditionOutcome {
    catalog(sim).evaluate(sim.world(), id, args)
}

/// TWO INDEPENDENT DOMAINS PUBLISHED QUESTIONS INTO ONE CATALOG.
///
/// neither names the other, and neither is listed anywhere central: the item
/// domain publishes from its own simulation plugin, the world-fact domain from a
/// plugin of its own. What composed them is composition.
#[test]
fn the_composed_engine_publishes_questions_from_more_than_one_domain() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    let domains: Vec<&str> = catalog
        .describe_all()
        .map(|descriptor| descriptor.id.domain())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert!(
        domains.len() >= 2,
        "the contract's acceptance is that a SECOND domain costs nothing central; \
         found only {domains:?}"
    );
    assert!(domains.contains(&"custody"), "{domains:?}");
    assert!(domains.contains(&"world"), "{domains:?}");
    // ⭐ AND THE BODY DOMAIN, WHICH IS THE INSTALLABILITY HALF. `body.can` and
    // `body.fits` are published by `BodyCapabilityConditionsPlugin` and unit-
    // tested against hand-built worlds; neither fact says the plugin is
    // COMPOSED. A domain that is written, tested and never added is exactly the
    // shape a route author would find by writing a `gated_by` that never opens,
    // and the composed engine is the only place that can witness it.
    assert!(
        domains.contains(&"body"),
        "`BodyCapabilityConditionsPlugin` is not in the composed engine, so no \
         authored route can gate on what a body can do: {domains:?}"
    );
}

/// EVERY PUBLISHED QUESTION DESCRIBES ITSELF WELL ENOUGH TO BE USED.
///
/// this is the discovery half, and it is an acceptance criterion rather than
/// polish: an agent that can list the questions but cannot tell what they take
/// has to read the engine's source, which is the thing this program exists to
/// stop.
#[test]
fn every_published_question_carries_a_schema_an_agent_could_act_on() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    assert!(!catalog.is_empty(), "nothing published");
    for descriptor in catalog.describe_all() {
        let id = &descriptor.id;
        assert!(!descriptor.summary.is_empty(), "`{id}` has no summary");
        assert!(
            id.as_str().contains('.'),
            "`{id}` is not namespaced by its owning domain"
        );
        for param in descriptor.params {
            assert!(!param.name.is_empty(), "`{id}` has an unnamed parameter");
            assert!(
                !param.summary.is_empty(),
                "`{id}` parameter `{}` has no summary, so an author cannot tell what to pass",
                param.name
            );
        }
    }
}

/// ASKING THE ITEM DOMAIN ABOUT A REAL OCCURRENCE TRACKS REAL STATE.
///
/// the interesting assertion is the THIRD one. Satisfied-then-not is a
/// property any boolean would have; the third answer — *unanswerable* about an
/// identity this world never authored — is what stops a gate that opens on the
/// negation from standing open in a level that has no key.
#[test]
fn the_item_domain_answers_about_custody_and_says_so_when_it_cannot() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    let is_held = ConditionId::new("custody", "is_held");

    // The one authored ground item, found through the same road the checkpoint
    // fixture uses: lying in the world, so nobody has it yet.
    let (authored, at) = {
        let mut query = sim.world_mut().query::<(
            &SimId,
            &ambition_platformer2d::held_items::GroundItem,
            &ambition_platformer2d::held_items::ItemCustody,
        )>();
        let found: Vec<(SimId, (f32, f32))> = query
            .iter(sim.world())
            .filter(|(_, _, custody)| custody.in_world())
            .map(|(id, ground, _)| (id.clone(), (ground.pos.x, ground.pos.y)))
            .collect();
        assert_eq!(found.len(), 1, "'{ROOM}' authors exactly one ground item");
        found[0].clone()
    };

    assert!(
        matches!(
            ask(&sim, &is_held, &[AuthoredArg::Reference(authored.clone())]),
            ConditionOutcome::NotSatisfied(_)
        ),
        "it is lying on the floor"
    );

    // Pick it up through the ordinary pressed pickup.
    sim.teleport_player(at);
    for _ in 0..40 {
        sim.step(AgentAction {
            attack: true,
            ..base()
        });
        sim.step(base());
        if ask(&sim, &is_held, &[AuthoredArg::Reference(authored.clone())])
            == ConditionOutcome::Satisfied
        {
            break;
        }
    }
    assert_eq!(
        ask(&sim, &is_held, &[AuthoredArg::Reference(authored.clone())]),
        ConditionOutcome::Satisfied,
        "the pressed pickup took custody, and the domain says so"
    );

    // the third answer.
    let never_authored = SimId::placement("a_key_this_world_does_not_have");
    let outcome = ask(&sim, &is_held, &[AuthoredArg::Reference(never_authored)]);
    assert!(
        matches!(outcome, ConditionOutcome::Unanswerable(_)),
        "an occurrence this world never authored is UNANSWERABLE, not false — a \
         gate opening on the negation would stand open forever. Got {outcome:?}"
    );
    assert!(
        !outcome.is_satisfied(),
        "and unanswerable must never read as satisfied"
    );
}

/// ASKING THE WORLD-FACT DOMAIN READS THE REAL SAVE.
///
/// an unset flag is `NotSatisfied` here, unlike the custody case, and the
/// asymmetry is the point rather than an inconsistency: a flag namespace is open,
/// so *"has this happened yet"* is a meaningful question about a fact nobody has
/// recorded. Answering *unanswerable* would leave every flag-gated thing stuck
/// until something set its flag once.
#[test]
fn the_world_fact_domain_answers_from_the_save_layer() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    let flag_set = ConditionId::new("world", "flag_set");
    let flag = "a_fact_this_run_has_not_recorded";

    assert!(matches!(
        ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]),
        ConditionOutcome::NotSatisfied(_)
    ));

    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(flag, true);

    assert_eq!(
        ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]),
        ConditionOutcome::Satisfied,
        "the domain reads the live save rather than a copy taken at startup"
    );
}

/// ASKING THE INVENTORY DOMAIN READS THE REAL BAG.
///
/// the third domain, and it cost one line of composition. It is here
/// because it is the provider that let an authored Yarn function be deleted:
/// `inventory_has(...)` was a closure over a mirrored copy of `OwnedItems` that
/// `ambition_content` refilled every frame. what is pinned is that the
/// composed engine answers about live inventory — not that this domain exists in
/// some list.
#[test]
fn the_inventory_domain_answers_about_the_live_bag() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    let holds = ConditionId::new("inventory", "holds");
    let carried = |sim: &Platformer2dSimHarness, item: &str| {
        ask(sim, &holds, &[AuthoredArg::Name(item.to_string())])
    };

    assert_eq!(
        carried(&sim, "HealthPotion"),
        ConditionOutcome::Satisfied,
        "the app's starter bag carries health cells, and loose authored spelling \
         resolves through the item catalog's single normaliser"
    );

    // Empty the slot through the domain's own API; the answer follows, with
    // nothing refreshed and no snapshot in between.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::items::OwnedItems>()
        .take(ambition_platformer2d::items::Item::HealthCell, u32::MAX);
    assert!(matches!(
        carried(&sim, "healthcell"),
        ConditionOutcome::NotSatisfied(_)
    ));

    // and a kind no catalog row spells is UNANSWERABLE rather than "no",
    // which is what turns an authored typo into a diagnostic.
    let outcome = carried(&sim, "a_thing_this_game_has_no_row_for");
    assert!(
        matches!(outcome, ConditionOutcome::Unanswerable(_)),
        "got {outcome:?}"
    );
}

/// ⭐⭐ EVERY AUTHORED `gated_by` IN EVERY SHIPPED WORLD PREPARES AGAINST THE
/// COMPOSED CATALOG — the guard that makes the authoring vocabulary safe to use.
///
/// Since `gated_by` became a condition LINE, a level author may write
/// `body.fits 32` or `encounter.cleared goblin_encounter` instead of a bare flag
/// name. ⛔ The failure mode that buys is silent and expensive: a misspelt
/// condition, a wrong argument count or a verb the catalog does not publish
/// leaves the wall STANDING, which is correct behaviour and indistinguishable in
/// play from a gate whose condition is simply not satisfied yet. The engine logs
/// it once at `error!` and the route is shut for the rest of the session.
///
/// ⇒ This asks the same question at test time, against the SAME catalog the game
/// composes, so an unpreparable line fails a build rather than a playthrough.
///
/// ⛔ THE POPULATION IS SMALL AND THAT IS THE POINT OF THE FIRST ASSERTION.
/// Exactly two entity instances in the shipped worlds carry a `gated_by` value
/// today (both `intro.ldtk`, both the flag `bob_field_survey_received`), so a
/// version of this test that merely iterated would pass on an empty list — and
/// would keep passing if the field were renamed, the parser stopped emitting it,
/// or the worlds stopped loading. The count is asserted so the corpus cannot
/// quietly become empty; ⚠ it is a FLOOR, not an inventory, because authoring a
/// new gated wall must not fail this test.
#[test]
fn every_authored_gate_condition_prepares_against_the_composed_catalog() {
    use ambition_platformer2d::actors::world::gated_lock_walls::prepare_authored_gate;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    let mut seen: Vec<(String, String)> = Vec::new();
    let mut unpreparable: Vec<String> = Vec::new();

    for source in ambition_content::worlds::world_manifest().worlds {
        // ⛔⛔ `embedded_text` IS `None` HERE, AND READING ONLY IT MADE THIS TEST
        // VACUOUS ON ITS FIRST RUN. `static_world_text!` compiles to `None`
        // without the `static_map` feature, which this target does not enable —
        // so every world was skipped and the loop found nothing. The floor
        // assertion below is what said so ("Found []") instead of a green tick.
        // ⇒ The loose path is the desktop road and the one this test must read;
        // the embedded text is the web/Android road and is kept as the fallback
        // for a profile that bakes it and ships no files.
        let text = match source.loose_path.as_deref().map(std::fs::read_to_string) {
            Some(Ok(text)) => text,
            _ => match source.embedded_text {
                Some(text) => text.to_string(),
                None => continue,
            },
        };
        let Ok(project) =
            serde_json::from_str::<ambition_platformer2d::ldtk_map::LdtkProject>(&text)
        else {
            continue;
        };
        for level in &project.levels {
            for layer in &level.layer_instances {
                for entity in &layer.entity_instances {
                    for field in &entity.field_instances {
                        if field.identifier != "gated_by" {
                            continue;
                        }
                        let Some(authored) = field.value.as_str() else {
                            continue;
                        };
                        if authored.is_empty() {
                            continue;
                        }
                        seen.push((source.id.as_str().to_string(), authored.to_string()));
                        // ⛔ THE PRODUCTION FUNCTION, not a copy of its rule.
                        // `prepare_authored_gate` is the one decision about what
                        // an authored gate value means, and the wall system calls
                        // the same one — so this cannot drift into validating a
                        // rule the game stopped applying.
                        if let Err(error) = prepare_authored_gate(&catalog, authored) {
                            unpreparable.push(format!(
                                "world `{}` wall gated by `{authored}`: {}",
                                source.id.as_str(),
                                error.reason()
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        seen.len() >= 2,
        "the corpus went empty — `gated_by` was set on 2 entity instances when \
         this was written, so finding fewer means the field was renamed, the \
         parser stopped emitting it, or the worlds stopped loading, and this \
         test would then pass against anything. Found {seen:?}"
    );
    assert!(
        unpreparable.is_empty(),
        "an authored gate names a question this engine cannot prepare, so its \
         wall stands forever and the route behind it is unreachable in play:\n  {}",
        unpreparable.join("\n  ")
    );
}

/// ⭐⭐ EVERY `condition(...)` AN AUTHORED `.yarn` FILE ASKS IS ONE THE COMPOSED
/// ENGINE PUBLISHES — the DIALOGUE half of the guard above, and it was missing.
///
/// `gated_by` got a build-time check when it became a condition line. The other
/// authored road never did: `ambition_conversation`'s Yarn verb
/// `condition(id, arg)` takes the id as a STRING an author types, and there are
/// ten times more of those in shipped `.yarn` than there are gated walls in
/// shipped worlds.
///
/// ⛔ THE FAILURE IS THE SAME SHAPE AND WORSE PLACED. `ask_condition` refuses an
/// unparseable or unpublished id with a `warn!` and returns `false`, so a
/// misspelt `condition("body.cann", …)` does not error — **the branch simply
/// never opens**, for the rest of the game, indistinguishable in play from a
/// condition that is honestly not satisfied. That is a playthrough-time failure
/// this test converts into a build-time one.
///
/// ⭐ ONLY THE GENERIC SPELLING IS GUARDED, and that is deliberate rather than
/// partial. The NAMED functions bound to a condition (`boss_cleared(id)`,
/// `quest_active(id)`) cannot carry a misspelt condition id — the id lives in
/// Rust, not in the `.yarn` — and misspelling the FUNCTION is already a Yarn
/// load error. ⇒ **The guard covers exactly the spelling an author can get wrong
/// silently.**
///
/// ⚠ ARITY IS CHECKED TOO. Yarn's verb passes exactly one argument, so a
/// condition the catalog declares with zero or two parameters is mis-called by
/// construction — a different mistake from a misspelt id and equally silent.
///
/// ⛔ AND THE FLOOR IS THE FIRST ASSERTION, for the reason the wall guard
/// learned by being vacuous on its first run: a version of this that merely
/// iterated would pass on an empty list, and would keep passing if `YARN_SOURCES`
/// stopped being the manifest or the call spelling changed. ⚠ A FLOOR, not an
/// inventory — authoring a new `condition(...)` line must not fail this test.
#[test]
fn every_condition_an_authored_yarn_file_asks_is_published_by_the_engine() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    let mut asked: Vec<(String, String)> = Vec::new();
    let mut unpublished: Vec<String> = Vec::new();
    let mut wrong_arity: Vec<String> = Vec::new();

    // ⭐ THE SHIPPED MANIFEST, not a directory walk: `YARN_SOURCES` is what
    // `yarn_spinner_plugin` registers and what the compile test compiles, so a
    // file this test reads is a file the game loads. A `.yarn` on disk that is
    // not in the manifest is not content, and should not fail a content guard.
    for (name, text) in ambition_content::dialogue::yarn::YARN_SOURCES {
        for rest in text.split("condition(").skip(1) {
            // ⛔⛔ THE QUOTE MUST BE THE VERY NEXT NON-SPACE CHARACTER, and a
            // looser rule made this test's FIRST RUN report a defect that was
            // not there. `kernel.yarn` says, in PROSE, *"condition() reads the
            // world-fact domain directly"* — and a scanner that took the next
            // quoted token after `condition(` walked past the empty parens, past
            // a sentence and a menu option, and grabbed the id out of
            // `<<command "world.set_flag" …>>` two lines later. It then reported
            // a COMMAND id as an unpublished CONDITION.
            //
            // ⇒ An instrument answering a wider question than the one asked: a
            // `condition(` in prose is not a call, and a quoted string much
            // further down the file is not its argument.
            let after = rest.trim_start();
            if !after.starts_with('"') {
                continue;
            }
            let open = rest.len() - after.len();
            let Some(close) = rest[open + 1..].find('"') else {
                continue;
            };
            let raw = &rest[open + 1..open + 1 + close];
            asked.push(((*name).to_string(), raw.to_string()));

            let Some(id) = ConditionId::parse(raw) else {
                unpublished.push(format!("{name}: {raw:?} is not a `domain.question` id"));
                continue;
            };
            match catalog.describe(&id) {
                None => unpublished.push(format!(
                    "{name}: {raw:?} — no domain in the composed engine publishes it"
                )),
                Some(descriptor) if descriptor.params.len() != 1 => wrong_arity.push(format!(
                    "{name}: {raw:?} declares {} parameter(s), but the Yarn verb passes exactly 1",
                    descriptor.params.len()
                )),
                Some(_) => {}
            }
        }
    }

    assert!(
        asked.len() >= 10,
        "only {} authored `condition(...)` call(s) found across {} shipped .yarn \\
         file(s) — the corpus this test walks has gone empty or the call spelling \\
         changed, and an empty walk passes every assertion below. Found: {asked:#?}",
        asked.len(),
        ambition_content::dialogue::yarn::YARN_SOURCES.len(),
    );
    assert!(
        unpublished.is_empty() && wrong_arity.is_empty(),
        "authored dialogue asks for conditions the composed engine cannot answer, \\
         so those branches never open in play:\\n  unpublished: {unpublished:#?}\\n  \\
         wrong arity: {wrong_arity:#?}",
    );
}

/// ⭐⭐ NO PLANNING DOC NAMES A CONDITION ID THE ENGINE DOES NOT PUBLISH — the
/// third road, and the one with no compiler behind it at all.
///
/// The two guards above cover authored CONTENT, where a wrong id silently closes
/// a branch. This one covers authored PROSE, where a wrong id costs something
/// different and slower: the next reader believes it. A condition id in a
/// planning doc is a citation, and a fabricated one is invisible to every check
/// this repository has — it compiles, because no code names it; it passes,
/// because no test names it; and it reads as authoritative, because it is a
/// plausible domain for the crate around it.
///
/// ⛔ THIS IS NOT HYPOTHETICAL AND IT IS NOT ONE MISTAKE. `held.is_held` was
/// written by hand across the planning set for a day — the id is
/// `custody.is_held`, because `ambition_held_items` declares `DOMAIN =
/// "custody"`. It was corrected on 2026-09-04 in four files; a second sweep the
/// same day found NINE more sites in six files, including the roadmap, a queue
/// receipt, and the module doc of the gate code itself. ⚠ And a third spelling,
/// `item.is_held`, sat in `inspection-diagnostics-and-workbench.md` listing the
/// production evaluators — found by this rule, after two prose sweeps missed it.
/// ⇒ Nine of eleven wrong is what a prose sweep scores against a mechanical one.
///
/// ⭐ THE RULE IS THE QUESTION-HALF MATCH, and that is what keeps it quiet. Docs
/// are full of backticked `a.b` tokens (`mod.rs`, `Vec2.x`); flagging every one
/// would be unusable. A token is only suspicious when its QUESTION half is a
/// question the engine really publishes but the whole id is not — `held.is_held`
/// against `custody.is_held`. That is precisely the near-miss shape, and
/// measured over 100 planning docs it fires on the real defects and nothing else.
///
/// ⚠ A DOC MAY NAME A WRONG ID DELIBERATELY — several must, because the
/// correction is their subject, and a guard that forbade it would delete the
/// record of the bug it exists to prevent. The escape is a rule worth having on
/// its own: **name the correct id in the same paragraph.** A wrong spelling
/// alone is a defect; a wrong spelling next to its correction is documentation.
#[test]
fn no_planning_doc_names_a_condition_the_engine_does_not_publish() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    // ⭐ THE COMPOSED CATALOG IS THE AUTHORITY, not a text scan for
    // `ConditionId::new`. A doc is checked against what the engine actually
    // answers, so publishing a condition cannot leave this guard behind.
    let published: Vec<String> = catalog.describe_all().map(|d| d.id.to_string()).collect();
    assert!(
        published.len() >= 5,
        "the composed catalog published {} condition(s); with a near-empty \
         catalog this test cannot recognise a near-miss and passes vacuously",
        published.len(),
    );

    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/planning");
    let mut suspicious: Vec<String> = Vec::new();
    let mut correct_citations = 0usize;
    let mut scanned = 0usize;

    for path in markdown_under(&docs) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}\nThis is an IO failure, NOT a finding about the repository — do not read any verdict from this run.", path.display()));
        scanned += 1;
        let rel = path.strip_prefix(&docs).unwrap_or(&path).display().to_string();
        let lines: Vec<&str> = text.split('\n').collect();

        // paragraph = a run of non-blank lines; the escape is scoped to it
        let mut para_of: Vec<Option<usize>> = Vec::with_capacity(lines.len());
        let mut para = 0usize;
        for line in &lines {
            if line.trim().is_empty() {
                para += 1;
                para_of.push(None);
            } else {
                para_of.push(Some(para));
            }
        }

        for (n, line) in lines.iter().enumerate() {
            for token in backticked_dotted_words(line) {
                if published.iter().any(|p| p == &token) {
                    correct_citations += 1;
                    continue;
                }
                let Some((_, question)) = token.split_once('.') else {
                    continue;
                };
                // Only a token whose QUESTION half is really published is a
                // near-miss; everything else is ordinary dotted prose.
                let intended: Vec<&String> = published
                    .iter()
                    .filter(|p| p.split_once('.').map(|(_, q)| q) == Some(question))
                    .collect();
                if intended.is_empty() {
                    continue;
                }
                let paragraph: String = lines
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| para_of[*i].is_some() && para_of[*i] == para_of[n])
                    .map(|(_, l)| *l)
                    .collect::<Vec<_>>()
                    .join("\n");
                if intended.iter().any(|c| paragraph.contains(c.as_str())) {
                    continue; // names its own correction alongside it
                }
                suspicious.push(format!(
                    "{rel}:{}: `{token}` names no published condition; the engine publishes {intended:?}",
                    n + 1
                ));
            }
        }
    }

    assert!(
        scanned >= 50 && correct_citations >= 10,
        "scanned {scanned} planning doc(s) holding {correct_citations} correct condition \
         citation(s) — the corpus this test walks has moved or gone empty, and an empty \
         walk passes the assertion below"
    );
    assert!(
        suspicious.is_empty(),
        "planning docs cite condition ids the composed engine does not publish. A \
         fabricated id compiles, passes, and reads as authoritative, so the next reader \
         inherits it. Fix the spelling, or name the correct id in the same paragraph if \
         the wrong one is the subject:\n  {}",
        suspicious.join("\n  ")
    );
}

/// Every `.md` beneath `dir`, recursively.
fn markdown_under(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("cannot list {}: {e}\nThis is an IO failure, NOT a finding about the repository.", d.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                out.push(path);
            }
        }
    }
    out
}

/// Backtick-delimited `lower_snake.lower_snake` tokens in one line.
fn backticked_dotted_words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, chunk) in line.split('`').enumerate() {
        if i % 2 == 0 {
            continue; // outside the backticks
        }
        let ok = |c: char| c.is_ascii_lowercase() || c == '_';
        if let Some((domain, question)) = chunk.split_once('.') {
            if !domain.is_empty()
                && !question.is_empty()
                && domain.chars().all(ok)
                && question.chars().all(ok)
            {
                out.push(chunk.to_string());
            }
        }
    }
    out
}
