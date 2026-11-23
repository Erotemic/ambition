use super::*;
use crate::{cue_program::PreparedMusicCues, cue_spec::*};
use ambition_asset_manager::content_library::{ContentLibraries, ContentLibrary};
use serde_json::{json, Value};

pub(crate) fn description() -> Value {
    let frame_plan = json!({"sample_rate": 8000, "score_origin_frame": 0, "musical_end_frame": 32000,
                "file_frames": 40000, "loop_frames": null, "tail_source_frames": 8000,
                "tail_source_audio_frames": 8000, "lead_in_source_frames": 0, "bar_frames": [0,16000,32000],
                "markers": [{"id": "body:entry", "role": "entry", "frame": 0},
                    {"id": "body:exit:2", "role": "exit", "frame": 32000}]});
    let product = json!({"layer": "full", "path": "playback/body/body.full.ogg", "sha256": "00".repeat(32),
                "frames": 40000, "sample_rate": 8000, "channels": 2, "format": "OGG", "subtype": "VORBIS",
                "frame_observer": "libsndfile", "gain_basis": "mastered_full", "runtime_stem_gain_mode": null,
                "applied_runtime_gain_db": 0.0, "mastering": "composition_master"});
    let section = json!({
        "id": "body", "form": ["body"], "loop": false, "lead_in": null, "tail": "ring",
        "lead_in_start_seconds": null, "tail_end_seconds": 5.0, "tail_audio_end_seconds": 5.0,
        "start_seconds": 0.0, "end_seconds": 4.0, "start_beat": 0.0, "end_beat": 8.0,
        "duration_beats": 8.0, "musical_end_seconds": 4.0, "file_duration_seconds": 5.0,
        "clock": {"kind": "constant", "bpm": 120.0, "beats_per_bar": 4.0, "bar_origin_seconds": 0.0},
        "markers": [{"id": "body:entry", "role": "entry", "beat": 0.0, "seconds": 0.0},
            {"id": "body:exit:2", "role": "exit", "beat": 8.0, "seconds": 4.0}],
        "material": {"tail": "appended", "lead_in": "none", "independent_tail": false,
            "scheduled_pickup": false, "tail_excludes_pickup": false, "unretained_pickup_attacks": 0},
        "product_roles": {"full": "mastered_full"},
        "frame_plan": frame_plan,
        "products": [product], "available": true,
    });
    json!({
        "schema": PLAYBACK_SCHEMA, "evidence": "published_products", "cue_id": "source",
        "score_one_shot": false, "compiled_score_fingerprint": "11".repeat(32),
        "render_fingerprint": "22".repeat(32), "score_clock": {}, "product_family": "playback",
        "layer_groups": {}, "sections": [section],
    })
}

const REFERENCE: &str = "content://media/music/cue/report.json";

pub(crate) struct Publication(tempfile::TempDir);
impl Publication {
    pub(crate) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("library.json"),
            json!({
                "schema":"ambition.content_library.v1", "id":"media", "version":"1",
                "asset_roots":{"generated":"runtime"}, "publication":"publication",
                "producers":{"music":{"roots":["music"]}},
            })
            .to_string(),
        )
        .unwrap();
        Self(root)
    }
    pub(crate) fn select(&self, generation: &str, mut description: Value) -> LibrarySnapshot {
        let root = self.0.path().join("publication");
        let mut files = serde_json::Map::new();
        let mut write = |logical: &str, bytes: &[u8]| {
            let relative = format!("bundles/{generation}/{logical}");
            let physical = root.join(&relative);
            std::fs::create_dir_all(physical.parent().unwrap()).unwrap();
            std::fs::write(physical, bytes).unwrap();
            let digest = format!("{:x}", Sha256::digest(bytes));
            files.insert(
                logical.into(),
                json!({"path":relative,"sha256":digest,"role":"runtime","tier":"full"}),
            );
            digest
        };
        for section in description["sections"].as_array_mut().unwrap() {
            for product in section["products"].as_array_mut().unwrap() {
                // These fixtures test the accepted identity, not codec decoding.
                if !validation::relative(product["path"].as_str().unwrap()) {
                    continue;
                }
                let bytes = format!("{generation}:{}", product["path"]);
                product["sha256"] = write(
                    &format!("music/cue/{}", product["path"].as_str().unwrap()),
                    bytes.as_bytes(),
                )
                .into();
            }
        }
        write(
            "music/cue/report.json",
            &serde_json::to_vec(&json!({"music_playback":description,"producer_diagnostics":{}}))
                .unwrap(),
        );
        std::fs::write(
            root.join("published.json"),
            json!({"schema":"ambition.art_publication.v1", "generation":generation,
            "files":files,"bundles":{"producer:music:cue:source":generation},"roots":["music"]})
            .to_string(),
        )
        .unwrap();
        let mut libraries = ContentLibraries::default();
        libraries
            .insert(ContentLibrary::read(self.0.path().join("library.json")).unwrap())
            .unwrap();
        libraries.snapshot().unwrap()
    }
}

pub(crate) fn authored(products: MusicProductSet, layers: &[&str]) -> AuthoredMusicCues {
    AuthoredMusicCues {
        cues: vec![MusicCueSpec {
            id: "program".into(),
            playback: Default::default(),
            media: MusicCueMedia::Published {
                description: REFERENCE.into(),
                products,
            },
            relative_volume: 0.5,
            layers: layers
                .iter()
                .enumerate()
                .map(|(slot, id)| MusicLayerSpec {
                    id: (*id).into(),
                    slot,
                })
                .collect(),
            states: vec![MusicStateSpec {
                id: "song".into(),
                section_id: "body".into(),
                next_state: None,
                gains: layers
                    .iter()
                    .map(|id| MusicLayerGainSpec {
                        layer_id: (*id).into(),
                        gain: 0.4,
                    })
                    .collect(),
            }],
            outro_state: None,
            post_clear_bridge_state: None,
            runtime_balance_overrides: vec![],
        }],
        ..Default::default()
    }
}

#[test]
fn a_program_links_the_selected_media_and_keeps_body_and_tail_separate() {
    let publication = Publication::new();
    let snapshot = publication.select("first", description());
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let prepared =
        PreparedMusicCues::prepare(authored(MusicProductSet::MasteredFull, &["full"]), &media)
            .unwrap();
    let cue = prepared.cue("program").unwrap();
    let section = cue.section("body").unwrap();
    assert_eq!(section.musical_duration_seconds, Some(4.0));
    assert_eq!(section.file_duration_seconds, Some(5.0));
    assert_eq!(section.clock.delay_to_boundary(3.0, 1, None), Some(1.0));
    assert!(section.sources[0].path.contains("/__art/bundles/first/"));
    assert_eq!(section.markers[1].role, MusicMarkerRole::Exit);
    assert_eq!(cue.state("song").unwrap().gains[0], 0.2);
}

#[test]
fn a_variable_clock_uses_its_encoded_bars_after_a_loop_wrap() {
    let mut description = description();
    let section = &mut description["sections"][0];
    section["end_seconds"] = 6.0.into();
    section["musical_end_seconds"] = 6.0.into();
    section["file_duration_seconds"] = 7.0.into();
    section["tail_end_seconds"] = 7.0.into();
    section["tail_audio_end_seconds"] = 7.0.into();
    section["clock"] = json!({"kind":"boundaries","bar_seconds":[0.0,2.0,6.0]});
    section["markers"][1]["seconds"] = 6.0.into();
    section["frame_plan"]["musical_end_frame"] = 48000.into();
    section["frame_plan"]["file_frames"] = 56000.into();
    section["frame_plan"]["bar_frames"] = json!([0, 16000, 48000]);
    section["frame_plan"]["markers"][1]["frame"] = 48000.into();
    section["products"][0]["frames"] = 56000.into();
    let publication = Publication::new();
    let snapshot = publication.select("variable", description);
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let prepared =
        PreparedMusicCues::prepare(authored(MusicProductSet::MasteredFull, &["full"]), &media)
            .unwrap();
    let section = prepared.cue("program").unwrap().section("body").unwrap();
    assert_eq!(
        section.clock.delay_to_boundary(3.0, 1, Some(6.0)),
        Some(3.0)
    );
    assert_eq!(
        section.clock.delay_to_boundary(9.0, 1, Some(6.0)),
        Some(3.0)
    );
}

#[test]
fn a_new_publication_cannot_retarget_an_old_snapshot() {
    let publication = Publication::new();
    let old = publication.select("first", description());
    let mut next = description();
    next["render_fingerprint"] = "33".repeat(32).into();
    let current = publication.select("second", next);
    for (snapshot, generation) in [(&old, "first"), (&current, "second")] {
        let media = MusicMediaCatalog::from_snapshot(snapshot, [REFERENCE]).unwrap();
        let prepared =
            PreparedMusicCues::prepare(authored(MusicProductSet::MasteredFull, &["full"]), &media)
                .unwrap();
        assert!(prepared.cue("program").unwrap().sections[0].sources[0]
            .path
            .contains(&format!("/__art/bundles/{generation}/")));
    }
}

#[test]
fn a_mixed_media_selection_is_refused_even_when_its_paths_exist() {
    let publication = Publication::new();
    let mut snapshot = publication.select("first", description());
    let files = std::sync::Arc::make_mut(&mut snapshot.files);
    files
        .get_mut("content://media/music/cue/playback/body/body.full.ogg")
        .unwrap()
        .sha256 = Some("ff".repeat(32));
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let errors =
        PreparedMusicCues::prepare(authored(MusicProductSet::MasteredFull, &["full"]), &media)
            .unwrap_err();
    assert!(errors[0].contains("digest differs"), "{errors:?}");
}

#[test]
fn invalid_encoded_facts_fail_before_program_registration() {
    let edits: &[fn(&mut Value)] = &[
        |d| d["evidence"] = "score_plan".into(),
        |d| d["render_fingerprint"] = Value::Null,
        |d| d["sections"][0]["frame_plan"]["musical_end_frame"] = 40000.into(),
        |d| d["sections"][0]["frame_plan"]["bar_frames"] = json!([0, 32000, 16000]),
        |d| d["sections"][0]["products"][0]["sample_rate"] = 48000.into(),
        |d| d["sections"][0]["products"][0]["path"] = "../other.ogg".into(),
        |d| d["sections"][0]["material"]["independent_tail"] = true.into(),
        |d| d["sections"][0]["markers"][1]["role"] = "chapter".into(),
    ];
    for edit in edits {
        let mut value = description();
        edit(&mut value);
        let publication = Publication::new();
        let snapshot = publication.select("invalid", value);
        assert!(MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).is_err());
    }
}

#[test]
fn constant_clocks_use_global_frame_rounding() {
    let clock = PreparedMusicClock::Constant {
        seconds_per_bar: 0.0015,
        first_bar_seconds: 0.0,
        bar_count: None,
        frames: Some(MusicFrameOrigin {
            sample_rate: 1000,
            score_seconds: 0.0006,
            score_frame: 1,
        }),
    };
    let delay = clock.delay_to_boundary(0.0005, 1, None).unwrap();
    assert!((delay - 0.0005).abs() < 1e-12);
}

fn grouped_description() -> Value {
    let mut value = description();
    value["layer_groups"] = json!({"bed":["piano"],"lead":["brass"]});
    let section = &mut value["sections"][0];
    let full = section["products"][0].clone();
    for layer in ["bed", "lead"] {
        section["product_roles"][layer] = "grouped_pre_master".into();
        let mut product = full.clone();
        product["layer"] = layer.into();
        product["path"] = format!("playback/body/body.{layer}.ogg").into();
        product["gain_basis"] = "grouped_pre_master".into();
        product["runtime_stem_gain_mode"] = "shared".into();
        product["applied_runtime_gain_db"] = (-6.0).into();
        product["mastering"] = Value::Null;
        section["products"].as_array_mut().unwrap().push(product);
    }
    value
}

#[test]
fn grouped_products_keep_their_gain_basis_and_do_not_add_the_mastered_mix() {
    let publication = Publication::new();
    let snapshot = publication.select("grouped", grouped_description());
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let prepared = PreparedMusicCues::prepare(
        authored(MusicProductSet::GroupedLayers, &["bed", "lead"]),
        &media,
    )
    .unwrap();
    let cue = prepared.cue("program").unwrap();
    assert_eq!(cue.sections[0].sources.len(), 2);
    assert!(cue.sections[0]
        .sources
        .iter()
        .all(|source| source.layer_id != "full"));
    assert_eq!(cue.states[0].gains, [0.2, 0.2, 0.0, 0.0, 0.0, 0.0]);
    assert!(PreparedMusicCues::prepare(
        authored(MusicProductSet::GroupedLayers, &["full", "bed", "lead"]),
        &media
    )
    .is_err());
}

#[test]
fn a_full_mix_does_not_require_unselected_grouped_products() {
    let publication = Publication::new();
    let mut snapshot = publication.select("full_only", grouped_description());
    std::sync::Arc::make_mut(&mut snapshot.files).retain(|_, file| {
        !file.logical.ends_with(".bed.ogg") && !file.logical.ends_with(".lead.ogg")
    });
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    PreparedMusicCues::prepare(authored(MusicProductSet::MasteredFull, &["full"]), &media).unwrap();
    let errors = PreparedMusicCues::prepare(
        authored(MusicProductSet::GroupedLayers, &["bed", "lead"]),
        &media,
    )
    .unwrap_err();
    assert!(errors[0].contains("is absent"), "{errors:?}");
}

#[test]
fn ungenerated_programs_keep_policy_checks_and_publish_no_playable_bindings() {
    let publication = Publication::new();
    let mut snapshot = publication.select("not_generated", description());
    std::sync::Arc::make_mut(&mut snapshot.files)
        .retain(|_, file| !file.logical.ends_with("report.json"));
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let mut policy = authored(MusicProductSet::MasteredFull, &["full"]);
    policy.room_bindings.push(RoomMusicBinding {
        room_id: "room".into(),
        cue_id: "program".into(),
        state: "song".into(),
        entry_state: None,
        fight_state: None,
    });
    policy.encounter_bindings.push(EncounterMusicBinding {
        encounter_id: "fight".into(),
        cue_id: "program".into(),
        starting_state: "song".into(),
        wave_states: vec!["song".into()],
        wave2_reinforced_state: None,
        cleared_state: "song".into(),
    });
    policy.boss_bindings.push(BossMusicBinding {
        boss_id: "boss".into(),
        cue_id: "program".into(),
        phase_states: vec![BossPhaseState {
            policy: Default::default(),
            phase: "Intro".into(),
            state: "song".into(),
        }],
    });
    let prepared = PreparedMusicCues::prepare(policy.clone(), &media).unwrap();
    assert_eq!(
        prepared.ungenerated().collect::<Vec<_>>(),
        [("program", REFERENCE)]
    );
    assert_eq!(prepared.cues().count(), 0);
    assert!(prepared.room_bindings().is_empty());
    assert!(prepared.encounter_bindings().is_empty());
    assert!(prepared.boss_bindings().is_empty());
    #[cfg(feature = "content_pack")]
    {
        use crate::content_schema::*;
        use ambition_content_pack::*;
        let mut draft = ContentPackDraft::from_manifest_ron(
            r#"(id:"music",version:"1",namespace:"music",sources:[(path:"cues.ron",schema:"music_cue_catalog",version:2)])"#,
            [("cues.ron".into(), ron::to_string(&policy).unwrap())],
        ).unwrap();
        draft.selected_libraries = Some(snapshot.clone());
        let mut schemas = SchemaRegistry::new();
        schemas.register(music_cue_catalog_schema()).unwrap();
        let pack = compile(&draft, &schemas, &NoAssets).unwrap();
        assert_eq!(lowered_music_cues(&pack), Some(&prepared));
        assert!(pack
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::UngeneratedSource));
    }
    for edit in [
        |policy: &mut AuthoredMusicCues| {
            policy.cues[0].states[0].gains[0].layer_id = "unknown".into()
        },
        |policy: &mut AuthoredMusicCues| {
            policy.cues[0].states[0].next_state = Some("unknown".into())
        },
        |policy: &mut AuthoredMusicCues| policy.room_bindings[0].state = "unknown".into(),
        |policy: &mut AuthoredMusicCues| policy.cues.push(policy.cues[0].clone()),
        |policy: &mut AuthoredMusicCues| policy.cues[0].relative_volume = f32::NAN,
        |policy: &mut AuthoredMusicCues| policy.cues[0].layers[0].slot = MAX_MUSIC_LAYERS,
    ] {
        let mut invalid = policy.clone();
        edit(&mut invalid);
        assert!(PreparedMusicCues::prepare(invalid, &media).is_err());
    }
    assert!(
        MusicMediaCatalog::from_snapshot(&snapshot, ["content://other/music/report.json"]).is_err()
    );
    assert!(
        MusicMediaCatalog::from_snapshot(&snapshot, ["content://media/authored/report.json"])
            .is_err()
    );
}

#[test]
fn embedded_programs_use_frozen_logical_paths_and_require_their_descriptions() {
    use ambition_asset_manager::content_library::{EmbeddedLibrary, EmbeddedLibraryFile};
    let publication = Publication::new();
    let native = publication.select("embedded", description());
    let files: BTreeMap<_, _> = native
        .files
        .values()
        .map(|file| (file.logical.clone(), file))
        .collect();
    let files: Vec<_> = files
        .into_iter()
        .map(|(path, file)| EmbeddedLibraryFile {
            path: Box::leak(path.into_boxed_str()),
            bytes: Some(Box::leak(file.read().unwrap().into_boxed_slice())),
        })
        .collect();
    let library = Box::leak(Box::new(EmbeddedLibrary {
        manifest: r#"{"schema":"ambition.content_library.v1","id":"media","version":"1","asset_roots":{"generated":"runtime"},"producers":{"music":{"roots":["music"]}}}"#,
        catalog: r#"{"entries":{}}"#,
        files: Box::leak(files.into_boxed_slice()),
    }));
    let mut libraries = ContentLibraries::default();
    libraries
        .insert(ContentLibrary::from_embedded(library).unwrap())
        .unwrap();
    let mut snapshot = libraries.snapshot().unwrap();
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let prepared =
        PreparedMusicCues::prepare(authored(MusicProductSet::MasteredFull, &["full"]), &media)
            .unwrap();
    assert_eq!(
        prepared.cue("program").unwrap().sections[0].sources[0].path,
        "content://media/music/cue/playback/body/body.full.ogg"
    );
    std::sync::Arc::make_mut(&mut snapshot.files)
        .retain(|_, file| !file.logical.ends_with("report.json"));
    assert!(MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).is_err());
}

#[test]
fn edited_metadata_cannot_retime_an_accepted_product() {
    let publication = Publication::new();
    let snapshot = publication.select("accepted", description());
    let file = snapshot.files[REFERENCE].native_path().unwrap();
    let mut data: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    data["music_playback"]["sections"][0]["musical_end_seconds"] = 3.0.into();
    std::fs::write(file, serde_json::to_vec(&data).unwrap()).unwrap();
    let errors = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap_err();
    assert!(
        errors[0].contains("accepted publication digest"),
        "{errors:?}"
    );
}

#[cfg(feature = "content_pack")]
#[test]
fn content_compilation_and_direct_registration_use_the_same_selected_facts() {
    use crate::content_schema::*;
    use ambition_content_pack::*;
    let publication = Publication::new();
    let snapshot = publication.select("selected", description());
    let authored = authored(MusicProductSet::MasteredFull, &["full"]);
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let direct = PreparedMusicCues::prepare(authored.clone(), &media).unwrap();
    let text = ron::to_string(&authored).unwrap();
    let manifest = r#"(id:"music",version:"1",namespace:"music",sources:[(path:"cues.ron",schema:"music_cue_catalog",version:2)])"#;
    let mut draft =
        ContentPackDraft::from_manifest_ron(manifest, [("cues.ron".into(), text)]).unwrap();
    draft.selected_libraries = Some(snapshot);
    let mut schemas = SchemaRegistry::new();
    schemas.register(music_cue_catalog_schema()).unwrap();
    let pack = compile(&draft, &schemas, &NoAssets).unwrap();
    assert_eq!(lowered_music_cues(&pack), Some(&direct));
    assert_eq!(pack.assets.len(), 2);
    assert!(pack
        .assets
        .values()
        .all(|asset| asset.path.contains("/__art/bundles/selected/")));

    #[cfg(feature = "kira")]
    {
        let mut registry = crate::music::AdaptiveMusicCatalogRegistry::default();
        registry.register("provider", direct).unwrap();
        let first = registry.generation("provider");
        let next_snapshot = publication.select("next", description());
        let next_media = MusicMediaCatalog::from_snapshot(&next_snapshot, [REFERENCE]).unwrap();
        let next = PreparedMusicCues::prepare(authored, &next_media).unwrap();
        let reloaded = registry.with_replaced("provider", Some(next)).unwrap();
        assert!(reloaded.generation("provider") > first);
        assert!(registry
            .catalog_for("provider")
            .unwrap()
            .cue("program")
            .unwrap()
            .sections[0]
            .sources[0]
            .path
            .contains("/selected/"));
        assert!(reloaded
            .catalog_for("provider")
            .unwrap()
            .cue("program")
            .unwrap()
            .sections[0]
            .sources[0]
            .path
            .contains("/next/"));
    }
}

#[test]
fn the_exported_bar_count_keeps_a_rounded_final_boundary() {
    let mut value = description();
    let body = 16.55172413793103_f64;
    let period = 240.0_f64 / 116.0;
    assert_eq!((body / period).floor(), 7.0);
    let section = &mut value["sections"][0];
    section["clock"]["bpm"] = 116.0.into();
    section["end_seconds"] = body.into();
    section["musical_end_seconds"] = body.into();
    section["file_duration_seconds"] = (body + 1.0).into();
    section["end_beat"] = 32.0.into();
    section["duration_beats"] = 32.0.into();
    section["tail_end_seconds"] = (body + 1.0).into();
    section["tail_audio_end_seconds"] = (body + 1.0).into();
    section["markers"][1]["beat"] = 32.0.into();
    section["markers"][1]["seconds"] = body.into();
    let end_frame = (body * 8000.0).round_ties_even() as u64;
    section["frame_plan"]["musical_end_frame"] = end_frame.into();
    section["frame_plan"]["file_frames"] = (end_frame + 8000).into();
    section["frame_plan"]["bar_frames"] = (0..=8)
        .map(|index| (index as f64 * period * 8000.0).round_ties_even() as u64)
        .collect::<Vec<_>>()
        .into();
    section["frame_plan"]["markers"][1]["frame"] = end_frame.into();
    section["products"][0]["frames"] = (end_frame + 8000).into();
    let publication = Publication::new();
    let snapshot = publication.select("rounded", value);
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let prepared =
        PreparedMusicCues::prepare(authored(MusicProductSet::MasteredFull, &["full"]), &media)
            .unwrap();
    let clock = &prepared.cue("program").unwrap().sections[0].clock;
    assert!(
        (clock
            .delay_to_boundary((end_frame - 1) as f64 / 8000.0, 1, None)
            .unwrap()
            - 1.0 / 8000.0)
            .abs()
            < 1e-9
    );
    assert_eq!(
        clock.delay_to_boundary((end_frame + 1) as f64 / 8000.0, 1, None),
        None
    );
}

#[test]
fn encoded_boundaries_keep_rounding_ties_the_arithmetic_grid_cannot_reproduce() {
    let mut value = description();
    let section = &mut value["sections"][0];
    let origin = 0.0000625_f64;
    section["start_seconds"] = origin.into();
    for field in ["end_seconds", "tail_end_seconds", "tail_audio_end_seconds"] {
        section[field] = (section[field].as_f64().unwrap() + origin).into();
    }
    let first = (origin * 8000.0).round_ties_even() as u64;
    let end = (section["end_seconds"].as_f64().unwrap() * 8000.0).round_ties_even() as u64 - first;
    let file =
        (section["tail_end_seconds"].as_f64().unwrap() * 8000.0).round_ties_even() as u64 - first;
    let boundary = ((origin + 2.0) * 8000.0).round_ties_even() as u64 - first + 1;
    section["frame_plan"]["score_origin_frame"] = first.into();
    section["frame_plan"]["musical_end_frame"] = end.into();
    section["frame_plan"]["file_frames"] = file.into();
    section["frame_plan"]["tail_source_frames"] = (file - end).into();
    section["frame_plan"]["tail_source_audio_frames"] = (file - end).into();
    section["frame_plan"]["markers"][1]["frame"] = end.into();
    section["products"][0]["frames"] = file.into();
    section["frame_plan"]["bar_frames"] = json!([0, boundary, end]);
    let publication = Publication::new();
    let snapshot = publication.select("ties", value);
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [REFERENCE]).unwrap();
    let prepared =
        PreparedMusicCues::prepare(authored(MusicProductSet::MasteredFull, &["full"]), &media)
            .unwrap();
    let clock = &prepared.cue("program").unwrap().sections[0].clock;
    assert!(
        (clock.delay_to_boundary(1.0, 1, None).unwrap() - (boundary as f64 / 8000.0 - 1.0)).abs()
            < 1e-9
    );
}
