//! Visual effects for AeroWM — Phase 4.
//!
//! This module wires the GLSL shaders into Smithay's GLES2 renderer.
//! Each effect is a lightweight struct that holds the compiled shader
//! program handle and exposes a `draw` method.
//!
//! ## Current state
//!
//! The shader sources are compiled at runtime from the .frag/.vert files
//! embedded via `include_str!`. The shader programs are compiled once at
//! compositor startup and cached.
//!
//! ## What requires Phase 4b
//! - Two-pass separable Gaussian blur (better quality, same cost)
//! - Background capture (render scene to FBO, sample in glass shader)
//! - Animation interpolation driving shader uniforms per frame

pub mod animation;
pub mod glass;
pub mod shadow;
