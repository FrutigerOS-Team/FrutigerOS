// AeroWM — Drop shadow fragment shader
//
// Renders a soft Gaussian-approximated drop shadow behind each window.
// Drawn as a pass before the window itself.

#version 300 es
precision mediump float;

uniform vec2  u_window_pos;   // NDC top-left
uniform vec2  u_window_size;  // NDC size
uniform float u_blur_radius;  // pixels
uniform vec2  u_offset;       // pixels (e.g. 0, 4)
uniform vec4  u_color;        // shadow RGBA (alpha carries opacity)
uniform vec2  u_resolution;
uniform float u_corner_radius;

in  vec2 v_uv;
out vec4 frag_color;

float rounded_rect_sdf(vec2 uv, vec2 size, float radius) {
    vec2 d = abs(uv - 0.5) * size - size * 0.5 + radius;
    return length(max(d, 0.0)) - radius;
}

// Approximate Gaussian with repeated box blurs (Welford's method).
float shadow_alpha(vec2 uv) {
    vec2 size_px  = u_window_size  * u_resolution;
    // Offset in UV space.
    vec2 off_uv   = u_offset / u_resolution;
    vec2 shifted  = uv - off_uv / u_window_size;

    float r = u_blur_radius;
    float a = 0.0;
    float total = 0.0;

    for (float dx = -2.0; dx <= 2.0; dx += 1.0) {
        for (float dy = -2.0; dy <= 2.0; dy += 1.0) {
            vec2 sample_uv = shifted + vec2(dx, dy) * (r / size_px);
            float d = rounded_rect_sdf(sample_uv, size_px, u_corner_radius);
            float w = exp(-(dx * dx + dy * dy) * 0.5);
            a += w * (1.0 - smoothstep(-1.0, r, d));
            total += w;
        }
    }

    return a / total;
}

void main() {
    float alpha = shadow_alpha(v_uv) * u_color.a;
    frag_color = vec4(u_color.rgb, alpha);
}
