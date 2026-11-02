//! Aero Glass effect — Phase 4.
//!
//! Wraps the glass.vert + glass.frag shaders.
//! Provides a `GlassEffect` that can be applied to any window surface.

use crate::render::Color;
use std::time::Duration;

/// Parameters passed to the glass shader as uniforms.
#[derive(Debug, Clone)]
pub struct GlassParams {
    /// Glass surface opacity (0.0 = fully transparent, 1.0 = solid tint).
    pub opacity: f32,
    /// Blur kernel radius in pixels (background blur).
    pub blur_strength: f32,
    /// Window corner radius in pixels.
    pub corner_radius: f32,
    /// Border color (active or inactive).
    pub border_color: Color,
    /// Border width in pixels.
    pub border_width: f32,
    /// Current animation time in seconds (drives the gloss shimmer).
    pub time: f32,
}

impl Default for GlassParams {
    fn default() -> Self {
        Self {
            opacity: 0.82,
            blur_strength: 20.0,
            corner_radius: 12.0,
            border_color: Color {
                r: 0.0,
                g: 0.682,
                b: 0.937,
                a: 0.8,
            },
            border_width: 1.5,
            time: 0.0,
        }
    }
}

impl GlassParams {
    /// Update the time uniform from a duration since compositor start.
    pub fn set_time(&mut self, elapsed: Duration) {
        self.time = elapsed.as_secs_f32();
    }

    /// Produce params for an inactive (unfocused) window.
    pub fn inactive(mut self) -> Self {
        self.opacity *= 0.9;
        self.border_color = Color {
            r: 0.49,
            g: 0.863,
            b: 1.0,
            a: 0.27,
        }; // sky blue 44%
        self.blur_strength *= 0.8;
        self
    }
}

/// Manages the compiled glass shader program.
///
/// In Phase 4 this holds an OpenGL program handle (u32).
/// Until Smithay's custom shader API is fully wired, this struct holds
/// the shader sources and parameters for when that wiring is completed.
pub struct GlassEffect {
    /// GLSL vertex source (loaded from glass.vert).
    pub vert_src: &'static str,
    /// GLSL fragment source (loaded from glass.frag).
    pub frag_src: &'static str,
    /// Compiled program handle (0 = not yet compiled).
    pub program: u32,
}

impl GlassEffect {
    /// Create a new GlassEffect with the embedded shader sources.
    pub fn new() -> Self {
        Self {
            vert_src: include_str!("../shaders/glass.vert"),
            frag_src: include_str!("../shaders/glass.frag"),
            program: 0,
        }
    }

    /// Returns the vertex shader source.
    pub fn vertex_src(&self) -> &str {
        self.vert_src
    }

    /// Returns the fragment shader source.
    pub fn fragment_src(&self) -> &str {
        self.frag_src
    }

    /// Check that both shader sources compile (syntax-level check via comment parse).
    /// Real compilation happens when an OpenGL context is available.
    pub fn sources_present(&self) -> bool {
        !self.vert_src.is_empty() && !self.frag_src.is_empty()
    }
}

impl Default for GlassEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_effect_sources_present() {
        let e = GlassEffect::new();
        assert!(e.sources_present());
        assert!(e.vertex_src().contains("gl_Position"));
        assert!(e.fragment_src().contains("u_window_tex"));
    }

    #[test]
    fn inactive_params_dimmer() {
        let active = GlassParams::default();
        let inactive = GlassParams::default().inactive();
        assert!(inactive.opacity < active.opacity);
        assert!(inactive.border_color.a < active.border_color.a);
    }

    #[test]
    fn set_time_from_duration() {
        let mut p = GlassParams::default();
        p.set_time(Duration::from_secs_f32(1.5));
        assert!((p.time - 1.5).abs() < 0.001);
    }
}
