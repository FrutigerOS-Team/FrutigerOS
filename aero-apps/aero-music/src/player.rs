//! Playback queue and player state machine.

use crate::library::Track;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepeatMode {
    None,
    One,
    All,
}

impl RepeatMode {
    /// Parse a user-facing repeat mode: `none`, `one`, or `all`.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "none" | "off" => Some(Self::None),
            "one" | "track" => Some(Self::One),
            "all" | "queue" => Some(Self::All),
            _ => None,
        }
    }
}

impl std::fmt::Display for RepeatMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::One => "one",
            Self::All => "all",
        })
    }
}

impl std::fmt::Display for PlayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Stopped => "stopped",
            Self::Playing => "playing",
            Self::Paused => "paused",
        })
    }
}

pub struct Queue {
    tracks: Vec<Track>,
    position: Option<usize>,
    state: PlayState,
    repeat: RepeatMode,
    shuffle: bool,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            position: None,
            state: PlayState::Stopped,
            repeat: RepeatMode::None,
            shuffle: false,
        }
    }

    pub fn enqueue(&mut self, track: Track) {
        self.tracks.push(track);
        if self.position.is_none() {
            self.position = Some(0);
        }
    }

    pub fn current(&self) -> Option<&Track> {
        self.position.and_then(|i| self.tracks.get(i))
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
    pub fn state(&self) -> PlayState {
        self.state
    }
    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }
    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn play(&mut self) {
        if self.position.is_some() {
            self.state = PlayState::Playing;
        }
    }
    pub fn pause(&mut self) {
        if self.state == PlayState::Playing {
            self.state = PlayState::Paused;
        }
    }
    pub fn stop(&mut self) {
        self.state = PlayState::Stopped;
        self.position = if self.tracks.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    pub fn next(&mut self) -> bool {
        let Some(pos) = self.position else {
            return false;
        };
        if self.repeat == RepeatMode::One {
            // Repeat the current track.
            return true;
        }
        if pos + 1 < self.tracks.len() {
            self.position = Some(pos + 1);
            true
        } else if self.repeat == RepeatMode::All {
            self.position = Some(0);
            true
        } else {
            self.state = PlayState::Stopped;
            false
        }
    }

    pub fn prev(&mut self) -> bool {
        let Some(pos) = self.position else {
            return false;
        };
        if pos > 0 {
            self.position = Some(pos - 1);
            true
        } else if self.repeat == RepeatMode::All {
            self.position = Some(self.tracks.len().saturating_sub(1));
            true
        } else {
            false
        }
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }
    pub fn set_shuffle(&mut self, on: bool) {
        self.shuffle = on;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_track(title: &str) -> Track {
        Track {
            path: String::new(),
            title: title.into(),
            artist: "Test".into(),
            album: String::new(),
            duration_secs: 180,
            track_number: None,
            year: None,
        }
    }

    #[test]
    fn queue_current() {
        let mut q = Queue::new();
        assert!(q.current().is_none());
        q.enqueue(make_track("A"));
        assert_eq!(q.current().unwrap().title, "A");
    }

    #[test]
    fn queue_next_advances() {
        let mut q = Queue::new();
        q.enqueue(make_track("A"));
        q.enqueue(make_track("B"));
        q.next();
        assert_eq!(q.current().unwrap().title, "B");
    }

    #[test]
    fn queue_repeat_all_wraps() {
        let mut q = Queue::new();
        q.set_repeat(RepeatMode::All);
        q.enqueue(make_track("A"));
        q.next(); // wraps to 0
        assert_eq!(q.current().unwrap().title, "A");
    }

    #[test]
    fn queue_prev_goes_back() {
        let mut q = Queue::new();
        q.enqueue(make_track("A"));
        q.enqueue(make_track("B"));
        q.next();
        assert!(q.prev());
        assert_eq!(q.current().unwrap().title, "A");
    }

    #[test]
    fn repeat_one_keeps_current() {
        let mut q = Queue::new();
        q.set_repeat(RepeatMode::One);
        q.enqueue(make_track("A"));
        q.enqueue(make_track("B"));
        assert!(q.next());
        assert_eq!(q.current().unwrap().title, "A");
    }

    #[test]
    fn repeat_mode_parse() {
        assert_eq!(RepeatMode::parse("ALL"), Some(RepeatMode::All));
        assert_eq!(RepeatMode::parse("track"), Some(RepeatMode::One));
        assert_eq!(RepeatMode::parse("off"), Some(RepeatMode::None));
        assert_eq!(RepeatMode::parse("bogus"), None);
    }

    #[test]
    fn play_pause_stop() {
        let mut q = Queue::new();
        q.enqueue(make_track("X"));
        q.play();
        assert_eq!(q.state(), PlayState::Playing);
        q.pause();
        assert_eq!(q.state(), PlayState::Paused);
        q.stop();
        assert_eq!(q.state(), PlayState::Stopped);
    }
}
