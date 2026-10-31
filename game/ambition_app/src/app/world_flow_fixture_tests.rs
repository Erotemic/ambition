use ambition_platformer2d::world_flow::*;
use bevy::prelude::*;
use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;
use ambition_platformer2d::sprite_sheet::boss::BossSpriteAsset;
use ambition_platformer2d::world::rooms::RoomSpec;

    #[test]
    fn a_room_waits_only_for_its_own_cached_boss_sheets() {
        use ambition_platformer2d::world::prelude::{AuthoredWorld, Vec2};
        use ambition_platformer2d::sprite_sheet::boss::{BossSheetSpec, BossSpritePage};
        let content = ambition_platformer2d::sprite_sheet::SpriteContent::default();
        let catalog = ambition_content::bosses::shipped_boss_catalog(&content);
        let key = catalog.authored_sheet_keys()
            .find(|key| Some(*key) != catalog.fallback_sheet_key()).unwrap().to_string();
        let mut room = RoomSpec::new("boss_room", AuthoredWorld::new(
            "boss room", Vec2::new(640.0, 360.0), Vec2::new(64.0, 256.0), Vec::new(),
        ));
        room.boss_spawns.push(ambition_platformer2d::world::rooms::Authored::new(
            "boss", &key,
            ambition_platformer2d::engine_core::Aabb::new(Vec2::new(320.0, 200.0), Vec2::new(20.0, 20.0)),
            ambition_platformer2d::entity_catalog::placements::BossBrain::PhaseScript { script_id: key.clone() },
        ));
        let mut images = Assets::<Image>::default();
        let mut assets = GameAssets::default();
        for name in [&key, &"unrelated".to_string()] {
            let spec = BossSheetSpec::unauthored();
            assets.boss_sprites.insert(name.clone(), BossSpriteAsset {
                record: spec.synth_record("body.png"), spec,
                pages: vec![BossSpritePage { texture: images.add(Image::default()), layout: Handle::default() }],
                layers: Vec::new(),
            });
        }
        let manifest = build_loaded_room_asset_manifest(&room, &[], &assets, Some(&catalog));
        let mut app = App::new();
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), bevy::asset::AssetPlugin::default()));
        let server = app.world().resource::<AssetServer>();
        let inspect = |images: &Assets<Image>| inspect_room_asset_manifest(server, Some(images),
            ambition_platformer2d::asset_manager::image_stages::RenderWorldPresent(false), None, &manifest);
        let ready = inspect(&images);
        assert_eq!(ready.total, 1, "only the room's boss contributes an image");
        assert!(ready.is_ready());
        images.remove(assets.boss_sprites.get(&key).unwrap().pages[0].texture.id());
        let pending = inspect(&images);
        assert_eq!(pending.pending, vec![format!("boss:{key}:page:0")]);
    }

    /// D166: every shipped room demands its cast by character id. An enemy's
    /// placement `name` is a caption or an id (`"gallery: goblin + striker"`,
    /// `"pg_brute"`) or a display name (`"Ai Slop"`). Demanded, it named no
    /// character, or reached one only through a display-name join that has
    /// been deleted. Poison: demand `enemy.name` and 28 tokens are named.
    #[test]
    fn every_shipped_room_demands_its_characters_by_id() {
        let manifest = ambition_content::worlds::world_manifest();
        let room_set = ambition_platformer2d::ldtk_map::LdtkProject::load_default_for_dev(&manifest)
            .expect("the shipped LDtk project loads")
            .to_room_set(&manifest, &crate::composed_ldtk_vocabulary())
            .expect("the shipped rooms build");
        // The composed host's two authorities, as the materializer reads them.
        // ONE frame first: the prepared registry is filled by a `Startup` system.
        let mut app = crate::app::build_visible_app(crate::app::VisibleRenderMode::NoWindow, true);
        app.update();
        let catalog = app
            .world()
            .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>();
        let registry = app
            .world()
            .resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>();
        let tokens: Vec<(String, String)> = room_set
            .rooms
            .iter()
            .flat_map(|room| {
                room_character_tokens(room, &[])
                    .into_iter()
                    .map(|token| (room.id.clone(), token))
            })
            .collect();
        assert!(
            tokens.len() > 100,
            "premise: the shipped rooms demand only {} characters",
            tokens.len()
        );
        let unknown: Vec<&(String, String)> = tokens
            .iter()
            .filter(|(_, token)| catalog.get(token).is_none() && registry.get(token).is_none())
            .collect();
        assert!(
            unknown.is_empty(),
            "{} (room, token) demands name no character: {unknown:?}",
            unknown.len()
        );
    }

