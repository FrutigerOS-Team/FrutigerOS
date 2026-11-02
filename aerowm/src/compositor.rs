//! AeroWM compositor.
//!
//! Winit backend (renders into a host window for development/testing).
//!
//! What this implements:
//! - Wayland display socket (clients can connect via $WAYLAND_DISPLAY)
//! - wl_compositor + wl_shm protocols
//! - XDG shell — maps/unmaps toplevel windows
//! - Keyboard input → AeroWM key actions (workspace switch, close, quit…)
//! - Pointer hit-testing, click-to-focus and pointer event forwarding
//! - Frame rendering: clear background + render each SHM surface texture
//! - AeroPanel integration: recognised by app_id, pinned full-width at the
//!   bottom of the screen and excluded from normal window management
//! - Frame callbacks so clients can throttle their own rendering
//!
//! What is still future work:
//! - Blur / glass shader passes (the shader sources exist but are not wired
//!   into Smithay's renderer yet)
//! - Interactive move/resize
//! - udev/DRM backend for real displays
//! - XWayland

use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use slog::{debug, info, o, warn, Logger};

use crate::statefile::{self, RuntimeState};

use smithay::{
    backend::{
        input::{
            ButtonState as BackendButtonState, Event as InputEventTrait, InputBackend, InputEvent,
            KeyState, KeyboardKeyEvent, MouseButton, PointerButtonEvent,
            PointerMotionAbsoluteEvent,
        },
        renderer::{
            buffer_dimensions,
            gles2::{Gles2Error, Gles2Renderer, Gles2Texture},
            Frame, ImportShm, Transform,
        },
        winit,
    },
    reexports::wayland_protocols::xdg_shell::server::xdg_toplevel,
    reexports::wayland_server::{
        protocol::{wl_pointer::ButtonState as WlButtonState, wl_surface::WlSurface},
        Display,
    },
    utils::{Logical, Point, Rectangle, Size},
    wayland::{
        compositor::{
            compositor_init, with_states, BufferAssignment, SurfaceAttributes, SurfaceData,
        },
        seat::{keysyms, KeyboardHandle, ModifiersState, PointerHandle, Seat, XkbConfig},
        shell::xdg::{
            xdg_shell_init, ToplevelSurface, XdgRequest, XdgToplevelSurfaceRoleAttributes,
        },
        shm::init_shm_global,
        SERIAL_COUNTER,
    },
};

use crate::config::Config;
use crate::input::{self, Action, Modifiers};
use crate::workspace::{Layout, WindowId, WorkspaceManager};

// ── Constants ─────────────────────────────────────────────────────────────────

/// app_id the AeroShell panel advertises.
const PANEL_APP_ID: &str = "frutigeros-aeropanel";

/// Background colour (Frutiger Aero deep ocean).
const BG: [f32; 4] = [0.024, 0.231, 0.361, 1.0]; // #063B5C

/// Cascade offset between newly mapped windows, in logical pixels.
const CASCADE_STEP: i32 = 32;

// ── Window ────────────────────────────────────────────────────────────────────

struct Window {
    id: WindowId,
    toplevel: ToplevelSurface,
    surface: WlSurface,
    app_id: Option<String>,
    title: Option<String>,
    /// True for the AeroShell panel (or another surface with the panel app_id).
    is_panel: bool,
    position: Point<i32, Logical>,
    /// Last imported texture (refreshed on every commit).
    texture: Option<Gles2Texture>,
    /// Size of the last committed buffer, in logical pixels.
    size: Size<i32, Logical>,
    /// Saved geometry used when leaving fullscreen.
    restore_position: Point<i32, Logical>,
    restore_size: Size<i32, Logical>,
    fullscreen: bool,
}

// ── State ─────────────────────────────────────────────────────────────────────

struct AeroState {
    log: Logger,
    config: Config,
    workspaces: WorkspaceManager,
    windows: HashMap<u64, Window>,
    next_id: u64,
    cascade: i32,
    keyboard: Option<KeyboardHandle>,
    pointer: Option<PointerHandle>,
    pointer_location: Point<f64, Logical>,
    screen: Size<i32, Logical>,
    start: Instant,
    should_quit: bool,
}

impl AeroState {
    fn new(config: Config, log: Logger, screen: Size<i32, Logical>) -> Self {
        let layout = Layout::from_str(&config.workspaces.default_layout);
        let workspaces = WorkspaceManager::new(config.workspaces.count, layout);
        info!(
            log,
            "Workspaces: {} configured, starting on {}",
            workspaces.count(),
            workspaces.active_index()
        );
        Self {
            log,
            config,
            workspaces,
            windows: HashMap::new(),
            next_id: 1,
            cascade: 0,
            keyboard: None,
            pointer: None,
            pointer_location: (0.0, 0.0).into(),
            screen,
            start: Instant::now(),
            should_quit: false,
        }
    }

    fn alloc_id(&mut self) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Publish the small runtime state file consumed by AeroShell.
    fn publish_state(&self) {
        let state = RuntimeState {
            active_workspace: self.workspaces.active_index(),
            workspace_count: self.workspaces.count(),
        };
        if let Err(err) = statefile::publish(state) {
            debug!(self.log, "Could not publish runtime state: {}", err);
        }
    }

    fn panel_height(&self) -> i32 {
        self.config.appearance.panel_height.max(24) as i32
    }

    fn panel_size(&self) -> Size<i32, Logical> {
        Size::from((self.screen.w.max(1), self.panel_height()))
    }

    fn panel_position(&self) -> Point<i32, Logical> {
        Point::from((0, (self.screen.h - self.panel_height()).max(0)))
    }

    fn cascade_position(&mut self) -> Point<i32, Logical> {
        let offset = self.cascade * CASCADE_STEP;
        self.cascade = (self.cascade + 1) % 8;
        Point::from((48 + offset, 48 + offset))
    }

    // ── Window lifecycle ────────────────────────────────────────────────────

    fn map_toplevel(&mut self, toplevel: ToplevelSurface) {
        let Some(surface) = toplevel.get_surface().cloned() else {
            warn!(self.log, "New toplevel has no wl_surface");
            return;
        };

        let id = self.alloc_id();
        let (app_id, title) = toplevel_metadata(&surface);
        let is_panel = app_id.as_deref() == Some(PANEL_APP_ID);

        let (position, size) = if is_panel {
            (self.panel_position(), self.panel_size())
        } else {
            (self.cascade_position(), Size::from((800, 600)))
        };

        let window = Window {
            id,
            toplevel: toplevel.clone(),
            surface,
            app_id: app_id.clone(),
            title,
            is_panel,
            position,
            texture: None,
            size,
            restore_position: position,
            restore_size: size,
            fullscreen: false,
        };

        if !is_panel {
            self.workspaces.active_mut().add_window(id);
        }
        self.windows.insert(id.0, window);

        if is_panel {
            self.configure_panel(id.0);
        } else {
            // Send the initial configure so the client can attach its first buffer.
            toplevel.send_configure();
            self.apply_focus();
        }

        info!(
            self.log,
            "Mapped window {:?} (app_id={:?}, panel={})", id, app_id, is_panel
        );
        self.publish_state();
    }

    fn remove_window(&mut self, key: u64) {
        let id = WindowId(key);
        let was_panel = self.windows.get(&key).map(|w| w.is_panel).unwrap_or(false);
        self.workspaces.remove_window_from_all(id);
        self.windows.remove(&key);
        if !was_panel {
            self.apply_focus();
        }
        info!(self.log, "Unmapped window {}", key);
        self.publish_state();
    }

    fn configure_panel(&mut self, key: u64) {
        let size = self.panel_size();
        let position = self.panel_position();
        if let Some(win) = self.windows.get_mut(&key) {
            win.is_panel = true;
            win.position = position;
            win.size = size;
            win.restore_position = position;
            win.restore_size = size;
            if win
                .toplevel
                .with_pending_state(|state| {
                    state.size = Some(size);
                })
                .is_err()
            {
                warn!(self.log, "Cannot configure panel {}: surface is dead", key);
                return;
            }
            win.toplevel.send_configure();
        }
    }

    /// Re-read shell metadata, promote panels and drop dead/unmapped surfaces.
    fn sync_windows(&mut self) {
        let keys: Vec<u64> = self.windows.keys().copied().collect();
        let mut remove = Vec::new();
        let mut promote = Vec::new();

        for key in keys {
            let Some(win) = self.windows.get(&key) else {
                continue;
            };

            if !win.surface.as_ref().is_alive() || !win.toplevel.alive() {
                remove.push(key);
                continue;
            }

            let unmapped = with_states(&win.surface, |states| {
                let attrs = states.cached_state.current::<SurfaceAttributes>();
                matches!(attrs.buffer, Some(BufferAssignment::Removed))
            })
            .unwrap_or(false);

            if unmapped {
                remove.push(key);
                continue;
            }

            let (app_id, title) = toplevel_metadata(&win.surface);
            let is_panel = app_id.as_deref() == Some(PANEL_APP_ID);

            if let Some(win) = self.windows.get_mut(&key) {
                win.app_id = app_id;
                win.title = title;
                if is_panel && !win.is_panel {
                    promote.push(key);
                }
            }
        }

        for key in remove {
            self.remove_window(key);
        }
        for key in promote {
            info!(self.log, "Promoting surface {} to AeroPanel", key);
            self.configure_panel(key);
        }
    }

    /// Import pending SHM buffers for all surfaces into GPU textures.
    fn import_buffers(&mut self, renderer: &mut Gles2Renderer) {
        let keys: Vec<u64> = self.windows.keys().copied().collect();
        for key in keys {
            let Some(win) = self.windows.get_mut(&key) else {
                continue;
            };
            if !win.surface.as_ref().is_alive() {
                continue;
            }

            let result = with_states(&win.surface, |states| {
                let mut attrs = states.cached_state.current::<SurfaceAttributes>();
                if let Some(BufferAssignment::NewBuffer { buffer, .. }) = attrs.buffer.take() {
                    let surf_data = states.data_map.get::<SurfaceData>();
                    let texture = renderer.import_shm_buffer(&buffer, surf_data, &[]);
                    Some((buffer, texture))
                } else {
                    None
                }
            });

            if let Ok(Some((buffer, Ok(tex)))) = result {
                if let Some(dims) = buffer_dimensions(&buffer) {
                    win.size = Size::from((dims.w, dims.h));
                }
                buffer.release();
                win.texture = Some(tex);
            }
        }
    }

    // ── Focus ───────────────────────────────────────────────────────────────

    fn focused_key(&self) -> Option<u64> {
        self.workspaces
            .active()
            .focused
            .map(|id| id.0)
            .filter(|key| self.windows.get(key).map(|w| !w.is_panel).unwrap_or(false))
    }

    fn apply_focus(&self) {
        let Some(keyboard) = &self.keyboard else {
            return;
        };
        let surface = self
            .focused_key()
            .and_then(|key| self.windows.get(&key))
            .map(|w| w.surface.clone());
        keyboard.set_focus(surface.as_ref(), SERIAL_COUNTER.next_serial());
    }

    fn focus_window(&mut self, key: u64) {
        if self.windows.get(&key).map(|w| w.is_panel).unwrap_or(true) {
            return;
        }
        let id = WindowId(key);
        if self.workspaces.active().windows.contains(&id) {
            self.workspaces.active_mut().focus_window(id);
            self.apply_focus();
        }
    }

    // ── Actions ─────────────────────────────────────────────────────────────

    fn dispatch_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::SwitchWorkspace(i) => {
                if self.workspaces.switch_to(i) {
                    let ws = self.workspaces.active();
                    info!(
                        self.log,
                        "Workspace {} of {} active (id {}, name {}, layout {})",
                        ws.index + 1,
                        self.workspaces.count(),
                        self.workspaces.active_index(),
                        ws.name,
                        ws.layout.as_str()
                    );
                    self.apply_focus();
                    self.publish_state();
                }
            }
            Action::FocusNext => {
                self.workspaces.active_mut().focus_next();
                self.apply_focus();
            }
            Action::FocusPrev => {
                self.workspaces.active_mut().focus_prev();
                self.apply_focus();
            }
            Action::CloseWindow => {
                if let Some(key) = self.focused_key() {
                    if let Some(win) = self.windows.get(&key) {
                        info!(self.log, "Closing window {} ({:?})", key, win.title);
                        win.toplevel.send_close();
                    }
                }
            }
            Action::ToggleFullscreen => self.toggle_fullscreen(),
            Action::OpenTerminal => spawn_app(&self.log, "aero-terminal"),
            Action::OpenLauncher => {
                debug!(
                    self.log,
                    "AeroStart popup is not implemented yet; press Super+Space in the shell"
                );
            }
            Action::ToggleFloat | Action::None => {}
        }
    }

    fn toggle_fullscreen(&mut self) {
        let Some(key) = self.focused_key() else {
            return;
        };
        let screen = self.screen;
        let Some(win) = self.windows.get_mut(&key) else {
            return;
        };

        let result = if !win.fullscreen {
            win.toplevel.with_pending_state(|state| {
                state.size = Some(screen);
                state.states.set(xdg_toplevel::State::Fullscreen);
            })
        } else {
            let restore = win.restore_size;
            win.toplevel.with_pending_state(|state| {
                state.size = Some(restore);
                state.states.unset(xdg_toplevel::State::Fullscreen);
            })
        };

        if result.is_err() {
            warn!(self.log, "Cannot toggle fullscreen: surface is dead");
            return;
        }

        if !win.fullscreen {
            win.restore_position = win.position;
            win.restore_size = win.size;
            win.position = Point::from((0, 0));
            win.size = screen;
            win.fullscreen = true;
        } else {
            win.position = win.restore_position;
            win.size = win.restore_size;
            win.fullscreen = false;
        }
        win.toplevel.send_configure();
    }

    // ── Pointer ─────────────────────────────────────────────────────────────

    fn hit_test(
        &self,
        location: Point<f64, Logical>,
    ) -> Option<(u64, WlSurface, Point<i32, Logical>)> {
        let mut order: Vec<u64> = self
            .workspaces
            .active()
            .windows
            .iter()
            .rev()
            .map(|id| id.0)
            .collect();
        // Panels sit above normal windows.
        order.extend(self.windows.values().filter(|w| w.is_panel).map(|w| w.id.0));

        for key in order {
            let Some(win) = self.windows.get(&key) else {
                continue;
            };
            let (w, h) = if win.is_panel {
                (self.screen.w, self.panel_height())
            } else {
                (win.size.w, win.size.h)
            };
            let x = location.x as i32;
            let y = location.y as i32;
            if x >= win.position.x
                && x < win.position.x + w
                && y >= win.position.y
                && y < win.position.y + h
            {
                return Some((key, win.surface.clone(), win.position));
            }
        }
        None
    }

    fn pointer_motion(&mut self, location: Point<f64, Logical>, time: u32) {
        self.pointer_location = location;
        let focus = self
            .hit_test(location)
            .map(|(_, surface, origin)| (surface, origin));
        if let Some(pointer) = &self.pointer {
            pointer.motion(location, focus, SERIAL_COUNTER.next_serial(), time);
        }
    }

    fn pointer_button(&mut self, button: MouseButton, state: BackendButtonState, time: u32) {
        if state == BackendButtonState::Pressed {
            if let Some((key, _, _)) = self.hit_test(self.pointer_location) {
                self.focus_window(key);
            }
        }

        let button_code = match button {
            MouseButton::Left => 0x110,
            MouseButton::Right => 0x111,
            MouseButton::Middle => 0x112,
            MouseButton::Other(n) => 0x110 + n as u32,
        };
        let button_state = if state == BackendButtonState::Pressed {
            WlButtonState::Pressed
        } else {
            WlButtonState::Released
        };

        if let Some(pointer) = &self.pointer {
            pointer.button(
                button_code,
                button_state,
                SERIAL_COUNTER.next_serial(),
                time,
            );
        }
    }

    // ── Rendering helpers ───────────────────────────────────────────────────

    /// Windows in bottom-to-top order. The panel is always last (on top).
    fn render_order(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self
            .workspaces
            .active()
            .windows
            .iter()
            .map(|id| id.0)
            .collect();
        ids.extend(self.windows.values().filter(|w| w.is_panel).map(|w| w.id.0));
        ids
    }

    fn send_frame_callbacks(&self) {
        let time = self.start.elapsed().as_millis() as u32;
        for win in self.windows.values() {
            let callbacks = with_states(&win.surface, |states| {
                let mut attrs = states.cached_state.current::<SurfaceAttributes>();
                std::mem::take(&mut attrs.frame_callbacks)
            });
            if let Ok(callbacks) = callbacks {
                for callback in callbacks {
                    callback.done(time);
                }
            }
        }
    }
}

// ── Metadata helpers ──────────────────────────────────────────────────────────

fn toplevel_metadata(surface: &WlSurface) -> (Option<String>, Option<String>) {
    with_states(surface, |states| {
        if let Some(attrs) = states
            .data_map
            .get::<Mutex<XdgToplevelSurfaceRoleAttributes>>()
        {
            let attrs = attrs.lock().unwrap();
            (attrs.app_id.clone(), attrs.title.clone())
        } else {
            (None, None)
        }
    })
    .unwrap_or((None, None))
}

fn spawn_app(log: &Logger, program: &str) {
    match Command::new(program).spawn() {
        Ok(_) => info!(log, "Launched {}", program),
        Err(err) => warn!(log, "Could not launch {}: {}", program, err),
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

fn render(backend: &mut winit::WinitGraphicsBackend, state: &AeroState) {
    let scale = backend.window_size().scale_factor;
    let order = state.render_order();

    let result = backend.render(|_renderer, frame| -> Result<(), Gles2Error> {
        frame.clear(BG)?;

        for key in order {
            let Some(win) = state.windows.get(&key) else {
                continue;
            };
            let Some(tex) = &win.texture else {
                continue;
            };
            let dst = Rectangle::from_loc_and_size(
                win.position.to_f64().to_physical(scale).to_i32_round(),
                win.size.to_f64().to_physical(scale).to_i32_round(),
            );
            frame.render_texture_at(
                tex,
                dst.loc,
                1, // buffer_scale
                scale,
                Transform::Normal,
                1.0, // alpha
            )?;
        }
        Ok(())
    });

    if let Err(err) = result {
        debug!(state.log, "Frame render failed: {:?}", err);
    }
}

// ── Main loop ─────────────────────────────────────────────────────────────────

pub fn run(config: Config, log: Logger) -> Result<()> {
    info!(log, "Initialising AeroWM (winit backend)");

    // ── Winit backend ───────────────────────────────────────────────────────
    let (mut graphics, mut input) = winit::init(log.new(o!("module" => "winit")))
        .map_err(|e| anyhow!("winit init: {:?}", e))?;

    let window_size = graphics.window_size();
    info!(
        log,
        "Window: {}×{} @ {}x",
        window_size.physical_size.w,
        window_size.physical_size.h,
        window_size.scale_factor,
    );

    let screen = window_size
        .physical_size
        .to_f64()
        .to_logical(window_size.scale_factor);
    let screen: Size<i32, Logical> = screen.to_i32_round();

    // ── State ───────────────────────────────────────────────────────────────
    let mut state = AeroState::new(config, log.new(o!()), screen);
    state.publish_state();

    // ── Wayland display ─────────────────────────────────────────────────────
    let mut display = Display::new();
    init_shm_global(&mut display, vec![], log.new(o!("module" => "shm")));

    compositor_init(
        &mut display,
        |_surface, _dispatch_data| {},
        log.new(o!("module" => "compositor")),
    );

    // ── Seat ────────────────────────────────────────────────────────────────
    let (mut seat, _) = Seat::new(
        &mut display,
        "aerowm-0".into(),
        log.new(o!("module" => "seat")),
    );
    let keyboard = seat
        .add_keyboard(XkbConfig::default(), 200, 25, |_seat, _surface| {})
        .map_err(|e| anyhow!("keyboard init: {:?}", e))?;
    let pointer = seat.add_pointer(|_status| {});
    state.keyboard = Some(keyboard.clone());
    state.pointer = Some(pointer.clone());

    // ── XDG shell ───────────────────────────────────────────────────────────
    // SAFETY: state outlives both the display and the shell handler; the
    // closures are dropped before state at the end of this function.
    let state_ptr = &mut state as *mut AeroState;

    xdg_shell_init(
        &mut display,
        move |request: XdgRequest, _data| {
            // SAFETY: state lives for the duration of this function.
            let s = unsafe { &mut *state_ptr };
            match request {
                XdgRequest::NewToplevel { surface } => {
                    s.map_toplevel(surface);
                }
                XdgRequest::Move { serial, .. } => {
                    debug!(s.log, "Move requested (serial={:?})", serial);
                }
                XdgRequest::Resize { serial, .. } => {
                    debug!(s.log, "Resize requested (serial={:?})", serial);
                }
                XdgRequest::Maximize { .. } | XdgRequest::Fullscreen { .. } => {
                    s.toggle_fullscreen();
                }
                XdgRequest::UnMaximize { .. } | XdgRequest::UnFullscreen { .. } => {
                    s.toggle_fullscreen();
                }
                _ => {}
            }
        },
        log.new(o!("module" => "xdg_shell")),
    );

    // Expose a Wayland socket so clients can connect.
    let socket = display
        .add_socket_auto()
        .map_err(|e| anyhow!("socket: {}", e))?
        .into_string()
        .unwrap_or_else(|_| "wayland-aerowm".into());
    info!(log, "WAYLAND_DISPLAY={}", socket);
    info!(log, "Clients can connect: WAYLAND_DISPLAY={} <app>", socket);
    info!(
        log,
        "Keybindings: Super+Shift+Q quit | Super+1-9 workspace | Super+W close | Alt+Tab focus"
    );

    let modifier = state.config.keybindings.modifier.clone();
    let mut last_frame = Instant::now();
    const FRAME_TIME: Duration = Duration::from_micros(1_000_000 / 60);

    // ── Event loop ──────────────────────────────────────────────────────────
    loop {
        input
            .dispatch_new_events(|event| match event {
                InputEvent::Keyboard { event } => {
                    let keycode = event.key_code();
                    let ks = event.state();
                    let mut action = Action::None;

                    keyboard.input(
                        keycode,
                        ks,
                        SERIAL_COUNTER.next_serial(),
                        0,
                        |mods: &ModifiersState, keysym: u32| {
                            if ks == KeyState::Pressed {
                                let our_mods = Modifiers {
                                    shift: mods.shift,
                                    ctrl: mods.ctrl,
                                    alt: mods.alt,
                                    super_key: mods.logo,
                                };
                                action =
                                    input::handle_key(&keysym_name(keysym), our_mods, &modifier);
                            }
                            // false = intercepted (don't forward to client)
                            // true  = forward to the focused client
                            action == Action::None
                        },
                    );

                    if action != Action::None {
                        state.dispatch_action(action);
                    }
                }
                InputEvent::PointerMotionAbsolute { event } => {
                    let ws = graphics.window_size();
                    let logical = ws.physical_size.to_f64().to_logical(ws.scale_factor);
                    let location: Point<f64, Logical> = (
                        event.x_transformed(logical.w.round() as i32),
                        event.y_transformed(logical.h.round() as i32),
                    )
                        .into();
                    state.pointer_motion(location, event.time());
                }
                InputEvent::PointerButton { event } => {
                    state.pointer_button(event.button(), event.state(), event.time());
                }
                InputEvent::DeviceRemoved { .. } => {
                    // Winit signals window close via DeviceRemoved.
                    info!(log, "Window close requested");
                    state.should_quit = true;
                }
                _ => {}
            })
            .map_err(|e| anyhow!("input: {:?}", e))?;

        if state.should_quit {
            break;
        }

        // Dispatch Wayland client requests.
        display.dispatch(Duration::ZERO, &mut ()).ok();
        display.flush_clients(&mut ());

        // Pick up new metadata, panels, and unmapped/destroyed surfaces.
        state.sync_windows();

        // Import any new SHM buffers into GPU textures.
        state.import_buffers(graphics.renderer());

        // Render at target FPS.
        let now = Instant::now();
        if now.duration_since(last_frame) >= FRAME_TIME {
            last_frame = now;
            render(&mut graphics, &state);
            state.send_frame_callbacks();
        } else {
            std::thread::sleep(FRAME_TIME.saturating_sub(now.duration_since(last_frame)));
        }
    }

    info!(log, "AeroWM shut down cleanly");
    Ok(())
}

// ── Keysym → name ─────────────────────────────────────────────────────────────

fn keysym_name(sym: u32) -> String {
    match sym {
        keysyms::KEY_Return | keysyms::KEY_KP_Enter => "return".into(),
        keysyms::KEY_space => "space".into(),
        keysyms::KEY_Tab => "tab".into(),
        keysyms::KEY_q | keysyms::KEY_Q => "q".into(),
        keysyms::KEY_w | keysyms::KEY_W => "w".into(),
        keysyms::KEY_f | keysyms::KEY_F => "f".into(),
        keysyms::KEY_v | keysyms::KEY_V => "v".into(),
        keysyms::KEY_1 => "1".into(),
        keysyms::KEY_2 => "2".into(),
        keysyms::KEY_3 => "3".into(),
        keysyms::KEY_4 => "4".into(),
        keysyms::KEY_5 => "5".into(),
        keysyms::KEY_6 => "6".into(),
        keysyms::KEY_7 => "7".into(),
        keysyms::KEY_8 => "8".into(),
        keysyms::KEY_9 => "9".into(),
        _ => format!("sym:{}", sym),
    }
}
