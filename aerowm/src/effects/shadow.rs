//! Drop shadow effect — Phase 4.

use crate::render::Color;

/// Parameters for the drop shadow shader.
#[derive(Debug, Clone)]
pub struct ShadowParams {
    pub blur_radius: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub color: Color,
    pub corner_radius: f32,
}

impl Default for ShadowParams {
    fn default() -> Self {
        Self {
            blur_radius: 24.0,
            offset_x: 0.0,
            offset_y: 4.0,
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.35,
            },
            corner_radius: 12.0,
        }
    }
}

pub struct ShadowEffect {
    pub frag_src: &'static str,
}

impl ShadowEffect {
    pub fn new() -> Self {
        Self {
            frag_src: include_str!("../shaders/shadow.frag"),
        }
    }

    pub fn sources_present(&self) -> bool {
        !self.frag_src.is_empty()
    }
}

impl Default for ShadowEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_source_present() {
        let e = ShadowEffect::new();
        assert!(e.sources_present());
        assert!(e.frag_src.contains("shadow_alpha"));
    }
}
