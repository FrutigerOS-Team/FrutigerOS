// AeroWM — Aero Glass vertex shader
// Passes UV coordinates and position to the fragment shader.
// Used for window glass surfaces.
#version 300 es
precision mediump float;

in  vec2 position;   // NDC position (-1..1)
in  vec2 uv_coord;   // texture UV (0..1)

out vec2 v_uv;
out vec2 v_pos;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_uv  = uv_coord;
    v_pos = position;
}
