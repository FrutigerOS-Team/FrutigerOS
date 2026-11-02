//! AeroGlass widget primitives.
//!
//! Defines the visual language of FrutigerOS UI components.
//! Phase 4 will implement pixel rendering of these using tiny-skia.
//! For now these are pure data types used for layout and theming.

use serde::{Deserialize, Serialize};

// ── Colors (from design-tokens.toml) ─────────────────────────────────────────

pub const AERO_BLUE: Color = Color {
    r: 0,
    g: 174,
    b: 239,
    a: 255,
};
pub const SKY_BLUE: Color = Color {
    r: 125,
    g: 220,
    b: 255,
    a: 255,
};
pub const AQUA: Color = Color {
    r: 0,
    g: 245,
    b: 212,
    a: 255,
};
pub const GLASS_WHITE: Color = Color {
    r: 234,
    g: 251,
    b: 255,
    a: 255,
};
pub const NATURE_GREEN: Color = Color {
    r: 126,
    g: 217,
    b: 87,
    a: 255,
};
pub const DEEP_OCEAN: Color = Color {
    r: 6,
    g: 59,
    b: 92,
    a: 255,
};

/// RGBA color (u8 channels).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub fn as_f32(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

// ── Glass surface ──────────────────────────────────────────────────────────────

/// Visual style for a glass surface.
#[derive(Debug, Clone)]
pub struct GlassSurface {
    /// Base tint color (usually a semi-transparent deep ocean or sky blue).
    pub tint: Color,
    /// Border color.
    pub border: Color,
    /// Corner radius in pixels.
    pub corner_radius: u32,
    /// Blur radius (backend-dependent, used as metadata in Phase 4).
    pub blur_radius: u32,
}

impl Default for GlassSurface {
    fn default() -> Self {
        Self {
            tint: DEEP_OCEAN.with_alpha(180),
            border: AERO_BLUE.with_alpha(100),
            corner_radius: 12,
            blur_radius: 20,
        }
    }
}

impl GlassSurface {
    pub fn panel() -> Self {
        Self {
            tint: DEEP_OCEAN.with_alpha(200),
            border: AERO_BLUE.with_alpha(60),
            corner_radius: 0, // panels are full-edge, no rounded corners
            blur_radius: 24,
        }
    }

    pub fn launcher() -> Self {
        Self {
            tint: DEEP_OCEAN.with_alpha(220),
            border: AERO_BLUE.with_alpha(140),
            corner_radius: 16,
            blur_radius: 32,
        }
    }

    pub fn notification() -> Self {
        Self {
            tint: DEEP_OCEAN.with_alpha(210),
            border: AERO_BLUE.with_alpha(120),
            corner_radius: 12,
            blur_radius: 20,
        }
    }
}

// ── Button ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub background: Color,
    pub background_hover: Color,
    pub background_pressed: Color,
    pub text: Color,
    pub corner_radius: u32,
}

impl ButtonStyle {
    pub fn primary() -> Self {
        Self {
            background: AERO_BLUE.with_alpha(200),
            background_hover: SKY_BLUE.with_alpha(220),
            background_pressed: AERO_BLUE,
            text: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            corner_radius: 8,
        }
    }

    pub fn ghost() -> Self {
        Self {
            background: Color::TRANSPARENT,
            background_hover: AERO_BLUE.with_alpha(30),
            background_pressed: AERO_BLUE.with_alpha(60),
            text: GLASS_WHITE,
            corner_radius: 8,
        }
    }
}

// ── Typography ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font_family: String,
    pub size: f32,
    pub weight: FontWeight,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub enum FontWeight {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

impl TextStyle {
    pub fn ui() -> Self {
        Self {
            font_family: "Inter".to_string(),
            size: 15.0,
            weight: FontWeight::Regular,
            color: GLASS_WHITE,
        }
    }

    pub fn label() -> Self {
        Self {
            font_family: "Inter".to_string(),
            size: 13.0,
            weight: FontWeight::Medium,
            color: GLASS_WHITE.with_alpha(200),
        }
    }

    pub fn heading() -> Self {
        Self {
            font_family: "Inter".to_string(),
            size: 21.0,
            weight: FontWeight::SemiBold,
            color: GLASS_WHITE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_alpha() {
        let c = AERO_BLUE.with_alpha(128);
        assert_eq!(c.a, 128);
        assert_eq!(c.r, AERO_BLUE.r);
    }

    #[test]
    fn color_as_f32_bounds() {
        let c = AERO_BLUE;
        let f = c.as_f32();
        for v in f.iter() {
            assert!(*v >= 0.0 && *v <= 1.0);
        }
    }

    #[test]
    fn glass_surface_panel_no_radius() {
        assert_eq!(GlassSurface::panel().corner_radius, 0);
    }
}
