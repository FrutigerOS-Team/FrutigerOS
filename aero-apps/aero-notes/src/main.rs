//! AeroNotes — simple notes application for FrutigerOS.
//!
//! The current binary is a command-line front end for the same Markdown note
//! backend that the future Aero Glass UI will use. Notes are plain `.md` files
//! with a small YAML-ish front matter block.
//!
//! ```text
//! aero-notes                        list notes
//! aero-notes --new TITLE [BODY...]  create a note
//! aero-notes --show ID              print a note body
//! aero-notes --search QUERY         search titles, bodies, and tags
//! aero-notes --pin ID               toggle the pinned flag
//! aero-notes --delete ID            delete a note
//! ```
//!
//! Set `AERONOTES_DIR` to override the default `~/Documents/AeroNotes`.

mod store;

use anyhow::{anyhow, Result};
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-notes"))
}

fn usage() -> &'static str {
    "usage:\n  \
     aero-notes\n  \
     aero-notes --new TITLE [BODY...]\n  \
     aero-notes --show ID\n  \
     aero-notes --search QUERY\n  \
     aero-notes --pin ID\n  \
     aero-notes --delete ID"
}

fn notes_dir() -> String {
    std::env::var("AERONOTES_DIR").unwrap_or_else(|_| {
        std::env::var("HOME")
            .map(|h| format!("{}/Documents/AeroNotes", h))
            .unwrap_or_else(|_| "./notes".into())
    })
}

fn print_summary<'a>(log: &slog::Logger, notes: impl IntoIterator<Item = &'a store::Note>) {
    for note in notes {
        let pin = if note.pinned { "📌 " } else { "" };
        info!(
            log,
            "  [{}] {}{} — {} chars",
            note.updated.format("%Y-%m-%d"),
            pin,
            note.title,
            note.body.len(),
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroNotes v{}", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return Ok(());
    }

    let dir = notes_dir();
    let mut store = store::NoteStore::open(&dir).await?;

    match args.first().map(String::as_str) {
        None => {
            info!(log, "Notes directory: {}", dir);
            info!(log, "{} note(s) loaded", store.count());
            print_summary(&log, store.list());
        }
        Some("--new") => {
            let title = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            let body = args[2..].join(" ");
            let note = store::Note::new(title, &body);
            let id = note.id.clone();
            store.save(note).await?;
            info!(log, "Created note {} ({})", id, dir);
        }
        Some("--show") => {
            let id = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            let note = store
                .get(id)
                .ok_or_else(|| anyhow!("no note with id {:?}", id))?;
            println!("{}", note.body);
        }
        Some("--search") => {
            let query = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            let hits = store.search(query);
            print_summary(&log, hits.iter().copied());
            info!(log, "{} match(es) for {:?}", hits.len(), query);
        }
        Some("--pin") => {
            let id = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            let mut note = store
                .get(id)
                .cloned()
                .ok_or_else(|| anyhow!("no note with id {:?}", id))?;
            note.pinned = !note.pinned;
            note.updated = chrono::Local::now();
            let pinned = note.pinned;
            store.save(note).await?;
            info!(
                log,
                "{} note {}",
                if pinned { "Pinned" } else { "Unpinned" },
                id
            );
        }
        Some("--delete") => {
            let id = args.get(1).ok_or_else(|| anyhow!("{}", usage()))?;
            store.delete(id).await?;
            info!(log, "Deleted note {}", id);
        }
        Some(flag) if flag.starts_with('-') => {
            return Err(anyhow!("unknown option {}\n{}", flag, usage()));
        }
        Some(other) => return Err(anyhow!("unexpected argument {:?}\n{}", other, usage())),
    }

    Ok(())
}
