//! Write a shipped move table out as the authored content file the host loads.
//!
//! This is the builder half of fast-iteration I2: the host reads a move table
//! instead of compiling one, and this exports a Rust table to that file. It
//! is a migration tool and a regenerator: run it after editing the Rust table,
//! until the Rust table is no longer the source.
//!
//! ```text
//! cargo run -p ambition_app_tools --bin moveset_source_export -- officer
//! ```
//!
//! It reads `authored_movesets::tables()`, the source, not a live host. The
//! prepared registry's copy differs: the `Unauthored` arm of character
//! preparation calls `revoke_host_owned_ranged`, which strips ranged verbs.
//! Exporting that would write a file whose reimport changes the game.
//! (`moveset_export`, the JSON balance bundle, reads the composed host on
//! purpose, because balance is about what the game resolves.)
//!
//! No App, no Bevy graph, no renderer: `tables()` is a pure function.

use std::io::Write as _;

const USAGE: &str = "\
moveset_source_export — write a shipped move table out as an authored content file

USAGE:
    moveset_source_export <character-id>... [--out DIR]
    moveset_source_export --list

    --out DIR   where the files go (default: game/ambition_content/assets/data/movesets)
    --list      print every character id `authored_movesets::tables()` carries
";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 || args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    let tables = ambition_content::authored_movesets::tables();
    if args.iter().any(|a| a == "--list") {
        for (id, contract) in &tables {
            // The character ids, not only the table name, so `--list` shows
            // which spelling a file will be keyed by.
            println!(
                "{id}\t{} move(s)\t{} verb(s)\t{:?}",
                contract.moves.len(),
                contract.verbs.len(),
                ambition_content::authored_movesets::characters_for(id)
                    .unwrap_or(&[])
            );
        }
        return;
    }
    // An unknown flag is a refusal; a typo must not export the default
    // silently.
    if let Some(bad) = args
        .iter()
        .skip(1)
        .filter(|a| a.starts_with("--"))
        .find(|a| *a != "--out")
    {
        eprintln!("moveset_source_export: unknown option '{bad}'\n");
        print!("{USAGE}");
        std::process::exit(2);
    }
    let out_dir = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "game/ambition_content/assets/data/movesets".to_string());
    let wanted: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .filter(|a| Some(*a) != args.windows(2).find(|w| w[0] == "--out").map(|w| &w[1]))
        .collect();

    std::fs::create_dir_all(&out_dir).expect("the output directory is writable");
    for id in wanted {
        let Some((_, contract)) = tables.iter().find(|(name, _)| name == id) else {
            // A name that matches nothing is a refusal. An empty file for a
            // typo would silently remove a fighter's moveset.
            eprintln!(
                "moveset_source_export: no shipped table named '{id}'. Known: {}",
                tables
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            std::process::exit(2);
        };
        // The entity id is the character's, not the table's, and they often
        // differ. `tables()` keys by a file name, while the host looks up a
        // move table by character id (`alice` is `npc_alice`; `patent_clerk`
        // is `special_patent_clerk`). A file under the table's name would be
        // missed by `authored_intrinsics`.
        //
        // One table may serve several characters (the two cellular automatons
        // share one body), so the document holds an entity per id, as
        // `EntityCatalogDoc` expects.
        let Some(characters) = ambition_content::authored_movesets::characters_for(id) else {
            eprintln!(
                "moveset_source_export: `{id}` is a table no cast id claims, so a \
                 content file for it would be loaded by nobody. See \
                 `TABLE_CHARACTERS`."
            );
            std::process::exit(2);
        };
        let doc = ambition_entity_catalog::EntityCatalogDoc {
            schema_version: ambition_entity_catalog::ENTITY_CATALOG_SCHEMA_VERSION,
            entities: characters
                .iter()
                .map(|character| ambition_entity_catalog::EntityDef {
                    id: (*character).to_string(),
                    contracts: ambition_entity_catalog::EntityContracts {
                        body: None,
                        hurtboxes: None,
                        presentation: None,
                        moveset: Some(contract.clone()),
                    },
                })
                .collect(),
        };
        let text = doc.to_ron().expect("a shipped table serializes");
        let path = std::path::Path::new(&out_dir).join(format!("{id}.ron"));
        // Write, then rename. An interrupted write would leave a truncated
        // file that the pack compiler reads as malformed content.
        let tmp = path.with_extension("ron.tmp");
        let mut file = std::fs::File::create(&tmp).expect("the output file is writable");
        writeln!(
            file,
            "// GENERATED by `cargo run -p ambition_app_tools --bin moveset_source_export -- {id}`.\n\
             // While the Rust table in `game/ambition_content/src/` is still the source, hand edits\n\
             // here are overwritten by the next export. Edit the Rust, re-export, commit both."
        )
        .and_then(|()| file.write_all(text.as_bytes()))
        .expect("the file writes");
        drop(file);
        std::fs::rename(&tmp, &path).expect("the rename lands");
        println!(
            "{} — {} move(s), {} verb(s), {} bytes, for {:?}",
            path.display(),
            contract.moves.len(),
            contract.verbs.len(),
            text.len(),
            characters
        );
    }
}
