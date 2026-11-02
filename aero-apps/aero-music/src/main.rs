//! AeroMusic — music player for FrutigerOS.
//!
//! Nostalgic-futuristic design: visualizer, glass panels, CD art.
//! Phase 6: library scanning, playback queue, metadata.
//! Phase 7: GTK4 + Aero Glass UI with spectrum visualizer.
//! Audio backend: GStreamer (via gst-rs) or PipeWire direct.
//!
//! ```text
//! aero-music [--dir PATH] [--search QUERY]
//!            [--repeat none|one|all] [--shuffle]
//!            [--play] [--pause] [--stop] [--next] [--prev]
//! ```
//!
//! Playback control currently updates the in-memory queue state only; the audio
//! backend is wired in Phase 7.

mod library;
mod player;

use anyhow::{anyhow, Result};
use slog::{info, o, Drain};

fn build_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!("app" => "aero-music"))
}

fn usage() -> &'static str {
    "usage: aero-music [--dir PATH] [--search QUERY] \
     [--repeat none|one|all] [--shuffle] \
     [--play] [--pause] [--stop] [--next] [--prev]"
}

fn default_dir() -> String {
    std::env::var("HOME")
        .map(|h| format!("{}/Music", h))
        .unwrap_or_else(|_| "./Music".into())
}

fn print_track(log: &slog::Logger, track: &library::Track) {
    info!(
        log,
        "  {} — {} ({})",
        track.artist,
        track.title,
        track.duration_str()
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let log = build_logger();
    info!(log, "AeroMusic v{}", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return Ok(());
    }

    // Parse options that take one value, and boolean flags.
    let mut music_dir = default_dir();
    let mut search = None;
    let mut repeat = player::RepeatMode::None;
    let mut shuffle = false;
    let mut control: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                music_dir = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow!("--dir needs a path\n{}", usage()))?;
                i += 2;
            }
            "--search" => {
                search = Some(
                    args.get(i + 1)
                        .cloned()
                        .ok_or_else(|| anyhow!("--search needs a query\n{}", usage()))?,
                );
                i += 2;
            }
            "--repeat" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow!("--repeat needs a mode\n{}", usage()))?;
                repeat = player::RepeatMode::parse(value)
                    .ok_or_else(|| anyhow!("unknown repeat mode {:?}\n{}", value, usage()))?;
                i += 2;
            }
            "--shuffle" => {
                shuffle = true;
                i += 1;
            }
            "--play" => {
                control = Some("play");
                i += 1;
            }
            "--pause" => {
                control = Some("pause");
                i += 1;
            }
            "--stop" => {
                control = Some("stop");
                i += 1;
            }
            "--next" => {
                control = Some("next");
                i += 1;
            }
            "--prev" => {
                control = Some("prev");
                i += 1;
            }
            flag if flag.starts_with('-') => {
                return Err(anyhow!("unknown option {}\n{}", flag, usage()));
            }
            other => return Err(anyhow!("unexpected argument {:?}\n{}", other, usage())),
        }
    }

    let lib = library::MusicLibrary::scan(&music_dir).await;
    info!(
        log,
        "Library: {} tracks in {}",
        lib.track_count(),
        music_dir
    );

    if let Some(query) = search {
        let hits = lib.search(&query);
        for track in &hits {
            print_track(&log, track);
        }
        info!(log, "{} match(es) for {:?}", hits.len(), query);
        return Ok(());
    }

    let mut queue = player::Queue::new();
    for track in lib.tracks() {
        queue.enqueue(track.clone());
    }
    queue.set_repeat(repeat);
    queue.set_shuffle(shuffle);

    if let Some(action) = control {
        match action {
            "play" => queue.play(),
            "pause" => queue.pause(),
            "stop" => queue.stop(),
            "next" => {
                queue.next();
            }
            "prev" => {
                queue.prev();
            }
            _ => unreachable!("validated control action"),
        }
    }

    info!(
        log,
        "Queue: {} track(s){}",
        queue.len(),
        if queue.is_empty() { " (empty)" } else { "" }
    );
    info!(
        log,
        "State: {}, repeat: {}, shuffle: {}",
        queue.state(),
        queue.repeat(),
        queue.shuffle()
    );
    if let Some(t) = queue.current() {
        info!(log, "Now playing: {} — {}", t.artist, t.title);
    } else {
        info!(log, "Nothing queued");
    }
    Ok(())
}
