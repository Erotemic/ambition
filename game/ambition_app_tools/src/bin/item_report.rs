//! Item report: what a save file says about the items in it, as text (A7's
//! agent surface).
//!
//! It reads the file a session writes, not a running world, so it answers
//! what a load would rebuild: each occurrence's whereabouts, the body holding
//! it, how a runtime mint is remade (its provenance and definition), and the
//! bag's counts.
//!
//! ```text
//! cargo run -p ambition_app_tools --bin item_report -- [--save PATH] where ID
//! cargo run -p ambition_app_tools --bin item_report -- [--save PATH] list [--room ROOM] [--body BODY]
//! cargo run -p ambition_app_tools --bin item_report -- [--save PATH] audit
//! cargo run -p ambition_app_tools --bin item_report -- [--save PATH] explain ID
//! ```
//!
//! `--save` defaults to the sandbox save under the platform data directory.
//! `audit` exits 1 when it finds a problem, so a script can gate on it.

use ambition_platformer2d::persistence::save::{load_save, save_path};
use ambition_platformer2d::persistence::save_data::{
    AmbitionGameSaveData, PersistedCustody, PersistedMintedItem, PersistedOccurrence, PersistedWhereabouts,
};

const USAGE: &str = "usage: item_report [--save PATH] (where ID | list [--room ROOM] [--body BODY] | audit | explain ID)";

enum Command {
    Where(String),
    List { room: Option<String>, body: Option<String> },
    Audit,
    Explain(String),
}

fn parse() -> Result<(std::path::PathBuf, Command), String> {
    let mut save = None;
    let mut words = Vec::new();
    let mut room = None;
    let mut body = None;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--save" => save = Some(it.next().ok_or("--save needs a path")?.into()),
            "--room" => room = Some(it.next().ok_or("--room needs a room id")?),
            "--body" => body = Some(it.next().ok_or("--body needs a body id")?),
            "-h" | "--help" => return Err(USAGE.to_owned()),
            _ => words.push(arg),
        }
    }
    let command = match words.as_slice() {
        [verb, id] if verb == "where" => Command::Where(id.clone()),
        [verb, id] if verb == "explain" => Command::Explain(id.clone()),
        [verb] if verb == "list" => Command::List { room, body },
        [verb] if verb == "audit" => Command::Audit,
        _ => return Err(USAGE.to_owned()),
    };
    Ok((save.unwrap_or_else(save_path), command))
}

/// The save's rows for one occurrence.
struct Rows<'a> {
    occurrence: Option<&'a PersistedOccurrence>,
    custody: Option<&'a PersistedCustody>,
    minted: Option<&'a PersistedMintedItem>,
}

fn rows_of<'a>(save: &'a AmbitionGameSaveData, id: &str) -> Rows<'a> {
    Rows {
        occurrence: save.occurrences().iter().find(|row| row.id == id),
        custody: save.custody().iter().find(|row| row.occurrence == id),
        minted: save.minted_items().iter().find(|row| row.occurrence == id),
    }
}

fn whereabouts(row: &PersistedWhereabouts) -> String {
    match row {
        PersistedWhereabouts::InCustody => "in custody".to_owned(),
        PersistedWhereabouts::Placed { room, x, y } => format!("placed in {room} at ({x}, {y})"),
        PersistedWhereabouts::Consumed => "consumed".to_owned(),
        PersistedWhereabouts::Spent => "spent".to_owned(),
    }
}

/// One line: `ID  placed in ROOM at (x, y)  held by BODY  minted from PARENT #n (DEF)`.
fn line(id: &str, rows: &Rows<'_>) -> String {
    let mut out = id.to_owned();
    match rows.occurrence {
        Some(row) => out.push_str(&format!("  {}", whereabouts(&row.whereabouts))),
        None => out.push_str("  (no occurrence row)"),
    }
    if let Some(custody) = rows.custody {
        out.push_str(&format!("  held by {}", custody.custodian));
    }
    if let Some(minted) = rows.minted {
        out.push_str(&format!("  minted from {} #{} ({})", minted.parent, minted.sequence, minted.held_item));
    }
    out
}

fn explain(id: &str, rows: &Rows<'_>) -> Vec<String> {
    let mut out = vec![line(id, rows)];
    let authored = rows.minted.is_none() && !rows.occurrence.is_some_and(|row| row.mint);
    out.push(match (rows.occurrence, authored) {
        (None, _) => "No row: a load builds it from its room's authored record, where the record puts it.".to_owned(),
        (Some(row), _) => match &row.whereabouts {
            PersistedWhereabouts::InCustody => match rows.custody {
                Some(custody) => format!("A load puts it back in {}'s hand.", custody.custodian),
                None => "Held, but no custody row says by whom: a load keeps any room from building a second one, \
                         and nobody holds it."
                    .to_owned(),
            },
            PersistedWhereabouts::Placed { room, .. } if authored => {
                format!("A load builds it in {room} where the row puts it, not where its record does.")
            }
            PersistedWhereabouts::Placed { room, .. } => format!("A load builds it in {room} where the row puts it."),
            PersistedWhereabouts::Consumed => "Gone for good: no load or room brings it back.".to_owned(),
            PersistedWhereabouts::Spent => {
                "Its room builds it where its record puts it, used up (an opened chest stays open).".to_owned()
            }
        },
    });
    out.push(match rows.minted {
        Some(minted) => format!(
            "A runtime mint: made by {} as its mint #{}, remade from the definition {:?}.",
            minted.parent, minted.sequence, minted.held_item
        ),
        None if authored => "Authored: its definition is its room's record.".to_owned(),
        None => "A runtime mint with no minted row: a load cannot remake it.".to_owned(),
    });
    out
}

/// What the save says that a load cannot honour, one line each.
fn audit(save: &AmbitionGameSaveData) -> Vec<String> {
    let mut problems = Vec::new();
    let known = |id: &str| save.occurrences().iter().any(|row| row.id == id);
    for custody in save.custody() {
        match save.occurrences().iter().find(|row| row.id == custody.occurrence) {
            None => problems.push(format!("custody row for {} names no occurrence", custody.occurrence)),
            Some(row) if row.whereabouts != PersistedWhereabouts::InCustody => problems.push(format!(
                "{} is held by {} but its row says {}",
                custody.occurrence,
                custody.custodian,
                whereabouts(&row.whereabouts)
            )),
            Some(_) => {}
        }
    }
    for minted in save.minted_items() {
        if !known(&minted.occurrence) {
            problems.push(format!("minted row for {} names no occurrence", minted.occurrence));
        }
    }
    for row in save.occurrences() {
        let has_minted = save.minted_items().iter().any(|minted| minted.occurrence == row.id);
        let live = !matches!(row.whereabouts, PersistedWhereabouts::Consumed | PersistedWhereabouts::Spent);
        if row.mint && live && !has_minted {
            problems.push(format!("{} is a mint, {}, and no minted row remakes it", row.id, whereabouts(&row.whereabouts)));
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in save.occurrences() {
        if !seen.insert(row.id.as_str()) {
            problems.push(format!("{} has two occurrence rows", row.id));
        }
    }
    problems
}

fn main() {
    let (path, command) = match parse() {
        Ok(parsed) => parsed,
        Err(usage) => {
            eprintln!("{usage}");
            std::process::exit(2);
        }
    };
    let loaded = load_save(&path);
    if !loaded.present {
        eprintln!("no save at {}", path.display());
        std::process::exit(2);
    }
    let save = &loaded.data;
    match command {
        Command::Where(id) => println!("{}", line(&id, &rows_of(save, &id))),
        Command::Explain(id) => explain(&id, &rows_of(save, &id)).iter().for_each(|line| println!("{line}")),
        Command::List { room, body } => {
            let mut ids: Vec<&str> = save
                .occurrences()
                .iter()
                .map(|row| row.id.as_str())
                .chain(save.custody().iter().map(|row| row.occurrence.as_str()))
                .chain(save.minted_items().iter().map(|row| row.occurrence.as_str()))
                .collect();
            ids.sort();
            ids.dedup();
            for id in ids {
                let rows = rows_of(save, id);
                let in_room = room.as_deref().is_none_or(|room| {
                    matches!(rows.occurrence.map(|row| &row.whereabouts), Some(PersistedWhereabouts::Placed { room: at, .. }) if at == room)
                });
                let held = body.as_deref().is_none_or(|body| rows.custody.is_some_and(|custody| custody.custodian == body));
                if in_room && held {
                    println!("{}", line(id, &rows));
                }
            }
            if room.is_none() && body.is_none() {
                let bag: Vec<String> = save.items().iter().map(|item| format!("{} x{}", item.id, item.count)).collect();
                println!("bag: {}  wallet: {}", if bag.is_empty() { "empty".to_owned() } else { bag.join(", ") }, save.wallet());
            }
        }
        Command::Audit => {
            let problems = audit(save);
            for problem in &problems {
                println!("{problem}");
            }
            if problems.is_empty() {
                println!(
                    "ok: {} occurrence rows, {} custody rows, {} minted rows",
                    save.occurrences().len(),
                    save.custody().len(),
                    save.minted_items().len()
                );
            } else {
                std::process::exit(1);
            }
        }
    }
}
