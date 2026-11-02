//! Window animation system — Phase 4.
//!
//! Every window has an `AnimationState`. The compositor calls `tick()` each
//! frame to advance all animations, then reads the resulting scale/opacity
//! values to drive the glass shader uniforms.

use std::time::Duration;

/// Per-window animation state.
#[derive(Debug, Clone)]
pub struct WindowAnimation {
    /// 0.0 = fully closed/invisible, 1.0 = fully open.
    pub progress: f32,
    pub phase: AnimPhase,
    /// Duration of open/close animations.
    pub duration: Duration,
    /// Speed multiplier from config (0.0 = instant).
    pub speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimPhase {
    Idle,
    Opening,
    Closing,
}

impl WindowAnimation {
    pub fn new(speed: f32) -> Self {
        Self {
            progress: 0.0,
            phase: AnimPhase::Opening,
            duration: Duration::from_millis(220),
            speed,
        }
    }

    /// Advance by one frame delta. Returns true if the animation just finished.
    pub fn tick(&mut self, delta: Duration) -> bool {
        if self.phase == AnimPhase::Idle {
            return false;
        }
        if self.speed == 0.0 {
            self.progress = if self.phase == AnimPhase::Opening {
                1.0
            } else {
                0.0
            };
            self.phase = if self.progress >= 1.0 {
                AnimPhase::Idle
            } else {
                AnimPhase::Closing
            };
            return true;
        }

        let step = delta.as_secs_f32() / self.duration.as_secs_f32() * self.speed;
        match self.phase {
            AnimPhase::Opening => {
                self.progress = (self.progress + step).min(1.0);
                if self.progress >= 1.0 {
                    self.phase = AnimPhase::Idle;
                    return true;
                }
            }
            AnimPhase::Closing => {
                self.progress = (self.progress - step).max(0.0);
                if self.progress <= 0.0 {
                    return true; // signal: window can be destroyed
                }
            }
            AnimPhase::Idle => {}
        }
        false
    }

    /// Begin a close animation. The window should be destroyed when tick() returns true.
    pub fn start_close(&mut self) {
        self.phase = AnimPhase::Closing;
    }

    /// Window scale derived from the spring easing curve.
    /// Used as a transform multiplier (0.85 → 1.0 on open).
    pub fn scale(&self) -> f32 {
        let t = spring_ease(self.progress);
        0.85 + 0.15 * t
    }

    /// Window opacity (fades in on open, out on close).
    pub fn opacity(&self) -> f32 {
        spring_ease(self.progress)
    }
}

/// Spring ease — slight overshoot at progress ≈ 0.7.
fn spring_ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let base = 1.0 - (1.0 - t).powi(3);
    let overshoot = if t < 0.7 {
        0.0
    } else {
        0.05 * (1.0 - (t - 0.7) / 0.3)
    };
    (base + overshoot).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_animation_reaches_idle() {
        let mut a = WindowAnimation::new(1.0);
        for _ in 0..100 {
            a.tick(Duration::from_millis(16));
        }
        assert_eq!(a.phase, AnimPhase::Idle);
        assert_eq!(a.progress, 1.0);
    }

    #[test]
    fn closing_animation_reaches_zero() {
        let mut a = WindowAnimation {
            progress: 1.0,
            phase: AnimPhase::Idle,
            duration: Duration::from_millis(100),
            speed: 1.0,
        };
        a.start_close();
        let mut finished = false;
        for _ in 0..100 {
            if a.tick(Duration::from_millis(16)) {
                finished = true;
                break;
            }
        }
        assert!(finished);
        assert!(a.progress <= 0.01);
    }

    #[test]
    fn instant_open_when_speed_zero() {
        let mut a = WindowAnimation::new(0.0);
        a.tick(Duration::from_millis(16));
        assert_eq!(a.progress, 1.0);
        assert_eq!(a.phase, AnimPhase::Idle);
    }

    #[test]
    fn scale_starts_small_ends_one() {
        let a_start = WindowAnimation::new(1.0);
        let mut a_end = WindowAnimation::new(1.0);
        a_end.progress = 1.0;
        assert!(a_start.scale() < a_end.scale());
        assert!((a_end.scale() - 1.0).abs() < 0.01);
    }
}
