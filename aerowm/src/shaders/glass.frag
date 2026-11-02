// AeroWM — Aero Glass fragment shader
//
// Renders a window surface with:
//   1. The window's own texture (app content).
//   2. A blurred background sample (glassmorphism effect).
//   3. A gradient border (Aero Blue).
//   4. Rounded corner clipping.
//   5. A subtle gloss reflection at the top edge.
//
// The blur is approximated with a 9-tap box blur on the background texture.
// A proper separable Gaussian requires two passes — Phase 4b will add that.

#version 300 es
precision mediump float;

// ── Uniforms ───────────────────────────────────────────────────────────────
// Window content texture (what the Wayland client drew).
uniform sampler2D u_window_tex;

// Background texture (the composited scene behind this window).
// Must be captured before this window is drawn.
uniform sampler2D u_background_tex;

// Window geometry in normalised screen coords.
uniform vec2  u_window_pos;   // top-left corner (NDC)
uniform vec2  u_window_size;  // width, height (NDC)

// Appearance parameters (from AeroWM config).
uniform float u_opacity;       // glass surface opacity 0..1
uniform float u_blur_strength; // blur radius in pixels
uniform float u_corner_radius; // corner radius in pixels
uniform vec2  u_resolution;    // output resolution in pixels
uniform vec4  u_border_color;  // RGBA, active or inactive
uniform float u_border_width;  // pixels
uniform float u_time;          // seconds, for animations

in  vec2 v_uv;
in  vec2 v_pos;
out vec4 frag_color;

// ── Rounded corner SDF ─────────────────────────────────────────────────────
// Returns signed distance from the edge of a rounded rectangle.
// Negative = inside, positive = outside.
float rounded_rect_sdf(vec2 uv, vec2 size, float radius) {
    vec2 d = abs(uv - 0.5) * size - size * 0.5 + radius;
    return length(max(d, 0.0)) - radius;
}

// ── 9-tap box blur ─────────────────────────────────────────────────────────
vec4 box_blur(sampler2D tex, vec2 uv, float radius_px, vec2 resolution) {
    vec2 texel = 1.0 / resolution;
    vec2 r = texel * radius_px;

    vec4 c = vec4(0.0);
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            c += texture(tex, uv + vec2(float(x), float(y)) * r);
        }
    }
    return c / 9.0;
}

// ── Gloss highlight ────────────────────────────────────────────────────────
// A soft linear gradient from white-ish at top to transparent at ~30%.
float gloss(vec2 uv) {
    float t = 1.0 - smoothstep(0.0, 0.3, uv.y);
    return t * 0.12;
}

void main() {
    // ── 1. Clip to rounded rect ──────────────────────────────────────────
    vec2 size_px = u_window_size * u_resolution;
    float dist = rounded_rect_sdf(v_uv, size_px, u_corner_radius);
    if (dist > 0.0) {
        discard;
    }
    float aa = 1.0 - smoothstep(-1.0, 0.0, dist); // anti-aliased edge

    // ── 2. Sample blurred background ────────────────────────────────────
    // Map window UV to screen UV for the background sample.
    vec2 screen_uv = u_window_pos + v_uv * u_window_size;
    vec4 bg = box_blur(u_background_tex, screen_uv, u_blur_strength, u_resolution);

    // ── 3. Glass tint ────────────────────────────────────────────────────
    // Deep Ocean tint: #063B5C at partial opacity.
    vec4 tint = vec4(0.024, 0.231, 0.361, 0.72);
    vec4 glass = mix(bg, tint, tint.a * u_opacity);

    // ── 4. Window content ────────────────────────────────────────────────
    vec4 content = texture(u_window_tex, v_uv);
    vec4 result = mix(glass, content, content.a);

    // ── 5. Gloss overlay ─────────────────────────────────────────────────
    result.rgb += gloss(v_uv);

    // ── 6. Border ────────────────────────────────────────────────────────
    float border_dist = dist + u_border_width;
    if (border_dist > 0.0 && dist < 0.0) {
        float t = smoothstep(0.0, 1.0, (border_dist) / u_border_width);
        result = mix(result, u_border_color, t * u_border_color.a);
    }

    // ── 7. Final alpha ───────────────────────────────────────────────────
    frag_color = vec4(result.rgb, result.a * aa);
}
