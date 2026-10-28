
use ambition_asset_manager::art_publication::*;

    #[test]
    fn a_selection_keeps_its_files_when_another_generation_is_published() {
        let dir = tempfile::tempdir().unwrap();
        let publish = |generation: &str, path: &str| {
            std::fs::write(dir.path().join("published.json"), serde_json::json!({
                "schema":SCHEMA,"generation":generation,"files":{"sprites/body.png":{
                    "path":path,"sha256":"digest","role":"runtime","tier":"full"}}
            }).to_string()).unwrap();
        };
        publish("one", "bundles/one/runtime/full/body.png");
        let first = Selection::read(dir.path()).unwrap();
        publish("two", "bundles/two/runtime/full/body.png");
        assert_eq!(first.path("sprites/body.png"), Some(dir.path().join("bundles/one/runtime/full/body.png")));
        assert_ne!(first.path("sprites/body.png"), Selection::read(dir.path()).unwrap().path("sprites/body.png"));
    }
