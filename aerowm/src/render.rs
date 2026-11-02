//! Render pipeline for AeroWM.
//!
//! This module defines the *types and traits* for the Aero Glass rendering
//! system. Actual GPU rendering via Smithay's GL renderer is wired in
//! compositor.rs once the backend is initialised.
//!
//! Design goals:
//! - Every window surface gets a glass material (blur + transparency).
//! - Rounded corners clipped via a per-surface corner-radius mask.
//! - Soft drop shadows rendered as a pre-pass blurred solid.
//! - Smooth open/close animations driven by a spring curve.

use std::time::Duration;

// ── Color types ───────────────────────────────────────────────────────────────

/// RGBA color with components in [0.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Parse a #RRGGBBAA or #RRGGBB hex string.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        let (r, g, b, a) = match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                (r, g, b, 255u8)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                (r, g, b, a)
            }
            _ => return None,
        };
        Some(Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        })
    }
}

// ── Glass material ────────────────────────────────────────────────────────────

/// Describes the glass surface appearance of a window.
#[derive(Debug, Clone)]
pub struct GlassMaterial {
    /// Opacity of the window surface (0.0 = invisible, 1.0 = opaque).
    pub opacity: f32,
    /// Background blur kernel radius in pixels.
    pub blur_radius: u32,
    /// Tint color applied on top of the blurred background.
    pub tint: Color,
    /// Border color (gradient start).
    pub border_color: Color,
    /// Corner radius in pixels.
    pub corner_radius: u32,
}

impl Default for GlassMaterial {
    fn default() -> Self {
        Self {
            opacity: 0.82,
            blur_radius: 20,
            tint: Color {
                r: 0.0,
                g: 0.68,
                b: 0.94,
                a: 0.15,
            }, // light Aero blue tint
            border_color: Color {
                r: 0.0,
                g: 0.68,
                b: 0.94,
                a: 0.8,
            },
            corner_radius: 12,
        }
    }
}

// ── Shadow ────────────────────────────────────────────────────────────────────

/// Drop shadow parameters.
#[derive(Debug, Clone)]
pub struct Shadow {
    pub blur_radius: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub color: Color,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            blur_radius: 24,
            offset_x: 0,
            offset_y: 4,
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.35,
            },
        }
    }
}

// ── Animation ─────────────────────────────────────────────────────────────────

/// State of a window open/close animation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationState {
    /// Window is fully visible; no animation running.
    Idle,
    /// Window is animating open (scale + fade in).
    Opening { progress: f32 },
    /// Window is animating closed (scale + fade out).
    Closing { progress: f32 },
}

/// Simple spring-based animation curve.
///
/// `progress` is in [0.0, 1.0]. Returns the eased value.
pub fn spring_ease(progress: f32) -> f32 {
    // Approximation of a spring with slight overshoot.
    // For a real spring: integrate mass-spring-damper per frame.
    let t = progress.clamp(0.0, 1.0);
    let base = 1.0 - (1.0 - t).powi(3); // ease-out cubic
                                        // Small overshoot at t ≈ 0.7
    let overshoot = if t < 0.7 {
        0.0
    } else {
        0.06 * (1.0 - (t - 0.7) / 0.3)
    };
    (base + overshoot).min(1.0)
}

/// Advance an animation by `delta` given a total `duration`.
/// Returns the new state.
pub fn tick_animation(
    state: AnimationState,
    delta: Duration,
    duration: Duration,
    speed: f32,
) -> AnimationState {
    if speed == 0.0 {
        // Animations disabled — snap to end state immediately.
        return match state {
            AnimationState::Opening { .. } => AnimationState::Idle,
            AnimationState::Closing { .. } => AnimationState::Closing { progress: 1.0 },
            s => s,
        };
    }

    let step = delta.as_secs_f32() / duration.as_secs_f32() * speed;

    match state {
        AnimationState::Opening { progress } => {
            let next = progress + step;
            if next >= 1.0 {
                AnimationState::Idle
            } else {
                AnimationState::Opening { progress: next }
            }
        }
        AnimationState::Closing { progress } => AnimationState::Closing {
            progress: (progress + step).min(1.0),
        },
        s => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_parse_hex6() {
        let c = Color::from_hex("#00AEEF").unwrap();
        assert!((c.r - 0.0).abs() < 0.01);
        assert!((c.g - 0.682).abs() < 0.01);
        assert!((c.b - 0.937).abs() < 0.01);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn color_parse_hex8() {
        let c = Color::from_hex("#00AEEFCC").unwrap();
        assert!((c.a - 0.8).abs() < 0.01);
    }

    #[test]
    fn color_parse_invalid() {
        assert!(Color::from_hex("nothex").is_none());
    }

    #[test]
    fn spring_ease_bounds() {
        assert_eq!(spring_ease(0.0), 0.0);
        let end = spring_ease(1.0);
        assert!(end <= 1.0);
        assert!(end > 0.95);
    }

    #[test]
    fn animation_snap_when_speed_zero() {
        let state = AnimationState::Opening { progress: 0.1 };
        let result = tick_animation(
            state,
            Duration::from_millis(16),
            Duration::from_millis(250),
            0.0,
        );
        assert_eq!(result, AnimationState::Idle);
    }
}
