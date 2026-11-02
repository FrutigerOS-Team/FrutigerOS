//! Software renderer for the AeroPanel.
//!
//! The panel is drawn into a premultiplied RGBA `tiny-skia` pixmap. The
//! result is copied into a wl_shm `ARGB8888` buffer by `panel.rs`.
//!
//! Visual language: deep-ocean glass, Aero-blue highlights, aqua accents and
//! a subtle top gloss — the Frutiger Aero look described in `design-tokens.toml`.

use chrono::{DateTime, Local};
use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point,
    RadialGradient, Rect, SpreadMode, Stroke, Transform,
};

use crate::glyphs;
use crate::panel::{PanelItem, PanelLayout};
use crate::widgets::{AERO_BLUE, AQUA, NATURE_GREEN, SKY_BLUE};

const TOP_GLOSS_ALPHA: u8 = 14;
const BORDER_ALPHA: u8 = 90;
const TEXT_PRIMARY_ALPHA: u8 = 240;
const TEXT_SECONDARY_ALPHA: u8 = 160;

fn rgba(c: crate::widgets::Color, alpha: u8) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, alpha)
}

fn solid(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Option<tiny_skia::Path> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let r = radius.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

fn fill_rounded(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, paint: &Paint) {
    if let Some(path) = rounded_rect(x, y, w, h, r) {
        pixmap.fill_path(&path, paint, FillRule::Winding, Transform::identity(), None);
    }
}

#[allow(clippy::too_many_arguments)]
fn stroke_rounded(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    width: f32,
    color: Color,
) {
    if let Some(path) = rounded_rect(x, y, w, h, r) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn fill_circle(pixmap: &mut Pixmap, cx: f32, cy: f32, radius: f32, color: Color) {
    if let Some(path) = PathBuilder::from_circle(cx, cy, radius) {
        pixmap.fill_path(
            &path,
            &solid(color),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

fn item_rect(layout: &PanelLayout, predicate: impl Fn(&PanelItem) -> bool) -> Option<Rect> {
    layout
        .items
        .iter()
        .find(|(item, _)| predicate(item))
        .and_then(|(_, geo)| {
            Rect::from_xywh(geo.x as f32, 0.0, geo.width as f32, geo.height as f32)
        })
}

/// Render one frame of the panel.
pub fn render(
    width: u32,
    height: u32,
    layout: &PanelLayout,
    active_workspace: usize,
    now: DateTime<Local>,
) -> Option<Pixmap> {
    if width == 0 || height == 0 {
        return None;
    }
    debug_assert_eq!(layout.total_width, width);
    debug_assert_eq!(layout.height, height);
    let mut pixmap = Pixmap::new(width, height)?;

    // ── Glass background: vertical deep-ocean gradient ──────────────────────
    let background = LinearGradient::new(
        Point::from_xy(0.0, 0.0),
        Point::from_xy(0.0, height as f32),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(11, 74, 110, 246)),
            GradientStop::new(0.55, Color::from_rgba8(6, 59, 92, 242)),
            GradientStop::new(1.0, Color::from_rgba8(3, 38, 59, 248)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    );
    if let Some(shader) = background {
        let paint = Paint {
            shader,
            ..Paint::default()
        };
        if let Some(rect) = Rect::from_xywh(0.0, 0.0, width as f32, height as f32) {
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }

    // Top gloss and a crisp Aero-blue edge light.
    if let Some(rect) = Rect::from_xywh(0.0, 0.0, width as f32, (height as f32 * 0.42).max(1.0)) {
        pixmap.fill_rect(
            rect,
            &solid(Color::from_rgba8(255, 255, 255, TOP_GLOSS_ALPHA)),
            Transform::identity(),
            None,
        );
    }
    if let Some(rect) = Rect::from_xywh(0.0, 0.0, width as f32, 1.0) {
        pixmap.fill_rect(
            rect,
            &solid(rgba(SKY_BLUE, BORDER_ALPHA)),
            Transform::identity(),
            None,
        );
    }
    if let Some(rect) = Rect::from_xywh(0.0, height as f32 - 1.0, width as f32, 1.0) {
        pixmap.fill_rect(
            rect,
            &solid(Color::from_rgba8(0, 0, 0, 40)),
            Transform::identity(),
            None,
        );
    }

    // ── Start orb ───────────────────────────────────────────────────────────
    if let Some(rect) = item_rect(layout, |i| matches!(i, PanelItem::StartButton)) {
        let cx = rect.x() + rect.width() / 2.0;
        let cy = rect.y() + rect.height() / 2.0;
        let radius = (rect.height() * 0.34).max(6.0);

        fill_circle(&mut pixmap, cx, cy, radius + 4.0, rgba(AQUA, 28));

        if let Some(shader) = RadialGradient::new(
            Point::from_xy(cx, cy),
            0.0,
            Point::from_xy(cx, cy),
            radius,
            vec![
                GradientStop::new(0.0, Color::from_rgba8(245, 253, 255, 255)),
                GradientStop::new(0.45, rgba(SKY_BLUE, 255)),
                GradientStop::new(1.0, rgba(AERO_BLUE, 255)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        ) {
            let paint = Paint {
                shader,
                ..Paint::default()
            };
            if let Some(path) = PathBuilder::from_circle(cx, cy, radius) {
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }

        if let Some(path) = PathBuilder::from_circle(cx, cy, radius) {
            let mut paint = Paint::default();
            paint.set_color(Color::from_rgba8(255, 255, 255, 150));
            let stroke = Stroke {
                width: 1.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // Stylised "F" mark in the centre of the orb.
        let scale = 2;
        let text_w = glyphs::text_width("F", scale) as f32;
        let text_h = (glyphs::GLYPH_HEIGHT * scale) as f32;
        glyphs::draw_text(
            &mut pixmap,
            cx - text_w / 2.0,
            cy - text_h / 2.0,
            scale,
            Color::from_rgba8(255, 255, 255, 255),
            "F",
        );
    }

    // ── Workspace switcher ──────────────────────────────────────────────────
    let accents = [AERO_BLUE, AQUA, NATURE_GREEN, SKY_BLUE];
    for (item, geo) in &layout.items {
        if let PanelItem::WorkspaceButton { index, .. } = item {
            let cx = geo.x as f32 + geo.width as f32 / 2.0;
            let cy = geo.height as f32 / 2.0;
            let accent = accents[*index % accents.len()];
            if *index == active_workspace {
                fill_circle(&mut pixmap, cx, cy, 10.0, rgba(accent, 38));
                fill_circle(&mut pixmap, cx, cy, 5.0, rgba(accent, 255));
                fill_circle(
                    &mut pixmap,
                    cx - 1.5,
                    cy - 1.5,
                    1.6,
                    Color::from_rgba8(255, 255, 255, 220),
                );
            } else {
                fill_circle(
                    &mut pixmap,
                    cx,
                    cy,
                    3.5,
                    Color::from_rgba8(234, 251, 255, 120),
                );
            }
        }
    }

    // ── Tagline in the flexible middle area ─────────────────────────────────
    if let Some(rect) = item_rect(layout, |i| matches!(i, PanelItem::Spacer)) {
        if rect.width() > 240.0 {
            let label = "TOMORROW, REIMAGINED";
            let text_w = glyphs::text_width(label, 1) as f32;
            let x = rect.x() + 14.0;
            if x + text_w < rect.x() + rect.width() - 8.0 {
                glyphs::draw_text(
                    &mut pixmap,
                    x,
                    rect.y() + (rect.height() - glyphs::GLYPH_HEIGHT as f32) / 2.0,
                    1,
                    Color::from_rgba8(234, 251, 255, 92),
                    label,
                );
            }
        }
    }

    // ── Clock ───────────────────────────────────────────────────────────────
    if let Some(rect) = item_rect(layout, |i| matches!(i, PanelItem::Clock)) {
        let time = now.format("%H:%M").to_string();
        let date = now.format("%a %d %b").to_string().to_uppercase();

        let time_scale = if rect.height() >= 42.0 { 2 } else { 1 };
        let time_w = glyphs::text_width(&time, time_scale) as f32;
        let time_h = (glyphs::GLYPH_HEIGHT * time_scale) as f32;
        let time_y = rect.y() + 7.0;
        glyphs::draw_text(
            &mut pixmap,
            rect.x() + (rect.width() - time_w) / 2.0,
            time_y,
            time_scale,
            Color::from_rgba8(255, 255, 255, TEXT_PRIMARY_ALPHA),
            &time,
        );

        let date_w = glyphs::text_width(&date, 1) as f32;
        let date_y = (time_y + time_h + 3.0).min(rect.y() + rect.height() - 9.0);
        glyphs::draw_text(
            &mut pixmap,
            rect.x() + (rect.width() - date_w) / 2.0,
            date_y,
            1,
            Color::from_rgba8(234, 251, 255, TEXT_SECONDARY_ALPHA),
            &date,
        );
    }

    // ── Tray icons ──────────────────────────────────────────────────────────
    if let Some(rect) = item_rect(layout, |i| matches!(i, PanelItem::Tray)) {
        let cy = rect.y() + rect.height() / 2.0;
        let spacing = rect.width() / 4.0;

        // Wi-Fi: three stacked arcs approximated by horizontal strokes.
        let wifi_x = rect.x() + spacing;
        for (i, (w, dy)) in [(13.0, -4.0), (9.0, 0.0), (5.0, 4.0)].iter().enumerate() {
            let y = cy + *dy;
            if let Some(rect) = Rect::from_xywh(wifi_x - w / 2.0, y, *w, 1.6) {
                let color = if i == 2 {
                    rgba(AERO_BLUE, 255)
                } else {
                    Color::from_rgba8(234, 251, 255, 190)
                };
                pixmap.fill_rect(rect, &solid(color), Transform::identity(), None);
            }
        }

        // Volume: a small speaker body plus two sound waves.
        let vol_x = rect.x() + spacing * 2.0;
        if let Some(path) = rounded_rect(vol_x - 5.0, cy - 4.0, 5.0, 8.0, 1.0) {
            pixmap.fill_path(
                &path,
                &solid(Color::from_rgba8(234, 251, 255, 200)),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        for (i, w) in [5.0, 9.0].iter().enumerate() {
            if let Some(rect) = Rect::from_xywh(vol_x + 1.0, cy - w / 2.0, 1.5, *w) {
                pixmap.fill_rect(
                    rect,
                    &solid(rgba(AQUA, if i == 0 { 230 } else { 150 })),
                    Transform::identity(),
                    None,
                );
            }
        }

        // Battery: outline with a 70% fill.
        let bat_x = rect.x() + spacing * 3.0;
        stroke_rounded(
            &mut pixmap,
            bat_x - 8.0,
            cy - 5.0,
            15.0,
            10.0,
            2.0,
            1.2,
            Color::from_rgba8(234, 251, 255, 210),
        );
        if let Some(rect) = Rect::from_xywh(bat_x - 6.0, cy - 3.0, 9.0, 6.0) {
            pixmap.fill_rect(
                rect,
                &solid(rgba(NATURE_GREEN, 220)),
                Transform::identity(),
                None,
            );
        }
        if let Some(rect) = Rect::from_xywh(bat_x + 7.0, cy - 2.0, 2.0, 4.0) {
            pixmap.fill_rect(
                rect,
                &solid(Color::from_rgba8(234, 251, 255, 190)),
                Transform::identity(),
                None,
            );
        }
    }

    // ── Quick settings pill ─────────────────────────────────────────────────
    if let Some(rect) = item_rect(layout, |i| matches!(i, PanelItem::QuickSettings)) {
        let px = rect.x() + 8.0;
        let py = rect.y() + 10.0;
        let pw = rect.width() - 16.0;
        let ph = rect.height() - 20.0;
        if pw > 20.0 && ph > 8.0 {
            fill_rounded(
                &mut pixmap,
                px,
                py,
                pw,
                ph,
                10.0,
                &solid(Color::from_rgba8(125, 220, 255, 26)),
            );
            stroke_rounded(
                &mut pixmap,
                px,
                py,
                pw,
                ph,
                10.0,
                1.0,
                Color::from_rgba8(125, 220, 255, 70),
            );

            let dots = [AERO_BLUE, AQUA, NATURE_GREEN];
            let step = pw / 4.0;
            for (i, color) in dots.iter().enumerate() {
                fill_circle(
                    &mut pixmap,
                    px + step * (i as f32 + 1.0),
                    py + ph / 2.0,
                    3.2,
                    rgba(*color, 235),
                );
            }
        }
    }

    Some(pixmap)
}

/// Convert a premultiplied RGBA pixmap to little-endian wl_shm `ARGB8888`
/// bytes (memory order B, G, R, A).
pub fn pixmap_to_argb8888(pixmap: &Pixmap) -> Vec<u8> {
    let src = pixmap.data();
    let mut out = Vec::with_capacity(src.len());
    for px in src.chunks_exact(4) {
        out.push(px[2]); // B
        out.push(px[1]); // G
        out.push(px[0]); // R
        out.push(px[3]); // A
    }
    out
}

/// Return the RGBA value of a pixel. Used by tests.
#[cfg(test)]
fn pixel(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let idx = ((y * pixmap.width() + x) * 4) as usize;
    let d = pixmap.data();
    [d[idx], d[idx + 1], d[idx + 2], d[idx + 3]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ShellConfig;
    use crate::panel::compute_layout;

    #[test]
    fn renders_panel_at_configured_size() {
        let cfg = ShellConfig::default();
        let layout = compute_layout(1280, &cfg, 4);
        let pm = render(1280, 48, &layout, 0, Local::now()).unwrap();
        assert_eq!(pm.width(), 1280);
        assert_eq!(pm.height(), 48);
        // The background gradient must be opaque-ish everywhere.
        assert!(pixel(&pm, 640, 24)[3] > 200);
    }

    #[test]
    fn start_orb_is_bright() {
        let cfg = ShellConfig::default();
        let layout = compute_layout(1280, &cfg, 4);
        let pm = render(1280, 48, &layout, 0, Local::now()).unwrap();
        // Centre of the start orb should contain a bright highlight.
        let p = pixel(&pm, 24, 24);
        assert!(p[0] > 150 && p[1] > 180 && p[2] > 200, "pixel={p:?}");
    }

    #[test]
    fn active_workspace_is_brighter_than_inactive() {
        let cfg = ShellConfig::default();
        let layout = compute_layout(1280, &cfg, 4);
        let pm = render(1280, 48, &layout, 1, Local::now()).unwrap();
        let active = layout
            .items
            .iter()
            .find_map(|(i, g)| match i {
                PanelItem::WorkspaceButton { index: 1, .. } => Some(g),
                _ => None,
            })
            .unwrap();
        let inactive = layout
            .items
            .iter()
            .find_map(|(i, g)| match i {
                PanelItem::WorkspaceButton { index: 0, .. } => Some(g),
                _ => None,
            })
            .unwrap();
        let a = pixel(&pm, active.x + active.width / 2, 24);
        let i = pixel(&pm, inactive.x + inactive.width / 2, 24);
        assert!(a[3] >= i[3] || a[1] > i[1]);
    }

    #[test]
    fn argb_conversion_swaps_red_and_blue() {
        let mut pm = Pixmap::new(1, 1).unwrap();
        pm.fill_rect(
            Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
            &solid(Color::from_rgba8(10, 20, 30, 255)),
            Transform::identity(),
            None,
        );
        let out = pixmap_to_argb8888(&pm);
        assert_eq!(out, vec![30, 20, 10, 255]);
    }
}
