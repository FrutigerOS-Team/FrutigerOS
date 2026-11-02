//! AeroPanel — the FrutigerOS taskbar.
//!
//! The panel is a normal xdg-shell toplevel. AeroWM recognises it by its
//! `app_id` (`frutigeros-aeropanel`) and pins it to the bottom edge, full
//! width, above all other windows.
//!
//! Rendering happens in software: pixels are drawn into a `tiny-skia` pixmap,
//! copied into a shared-memory `ARGB8888` buffer and committed to the
//! compositor. This keeps the shell independent of GL drivers and font
//! configuration, which matters on a freshly booted live ISO.

use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use slog::{debug, error, info, warn};
use wayland_client::{
    protocol::{wl_buffer, wl_compositor, wl_shm},
    Display, GlobalManager,
};
use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

use crate::config::ShellConfig;
use crate::panel_render;

/// Application id that identifies the panel to AeroWM.
pub const APP_ID: &str = "frutigeros-aeropanel";

/// Fallback width used before the compositor sends its first configure event.
const FALLBACK_WIDTH: u32 = 1920;

/// Panel component identity — used for layout calculations.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelItem {
    StartButton,
    WorkspaceButton {
        index: usize,
        active: bool,
    },
    /// Populated once the taskbar lists running windows.
    #[allow(dead_code)]
    TaskEntry {
        title: String,
        app_id: String,
    },
    Spacer,
    Clock,
    Tray,
    QuickSettings,
}

/// Computed layout of panel items, left to right.
#[derive(Debug)]
pub struct PanelLayout {
    pub items: Vec<(PanelItem, PanelItemGeometry)>,
    pub total_width: u32,
    pub height: u32,
}

/// Position and size of a single panel item.
#[derive(Debug, Clone)]
pub struct PanelItemGeometry {
    pub x: u32,
    pub width: u32,
    pub height: u32,
}

/// Compute a panel layout for a given screen width and config.
///
/// Items are laid out left-to-right:
///   [Start] [WS1 WS2 WS3 WS4] [────tasks────] [Clock] [Tray] [QuickSettings]
pub fn compute_layout(
    screen_width: u32,
    config: &ShellConfig,
    workspace_count: usize,
) -> PanelLayout {
    let h = config.panel.height;
    let mut items = Vec::new();
    let mut x: u32 = 0;

    let pad = 4u32;

    // Start button
    let start_w = h + pad * 2; // square-ish
    items.push((
        PanelItem::StartButton,
        PanelItemGeometry {
            x,
            width: start_w,
            height: h,
        },
    ));
    x += start_w;

    // Workspace buttons
    if config.panel.show_workspaces {
        let ws_w = 36u32;
        for i in 0..workspace_count {
            items.push((
                PanelItem::WorkspaceButton {
                    index: i,
                    active: i == 0,
                },
                PanelItemGeometry {
                    x,
                    width: ws_w,
                    height: h,
                },
            ));
            x += ws_w;
        }
    }

    // Right-side items: measure right-to-left first.
    let mut right_x = screen_width;

    {
        let qs_w = 120u32;
        right_x = right_x.saturating_sub(qs_w);
        items.push((
            PanelItem::QuickSettings,
            PanelItemGeometry {
                x: right_x,
                width: qs_w,
                height: h,
            },
        ));
    }

    if config.panel.show_tray {
        let tray_w = 80u32;
        right_x = right_x.saturating_sub(tray_w);
        items.push((
            PanelItem::Tray,
            PanelItemGeometry {
                x: right_x,
                width: tray_w,
                height: h,
            },
        ));
    }

    if config.panel.show_clock {
        let clock_w = 120u32;
        right_x = right_x.saturating_sub(clock_w);
        items.push((
            PanelItem::Clock,
            PanelItemGeometry {
                x: right_x,
                width: clock_w,
                height: h,
            },
        ));
    }

    // Spacer fills the middle (task area — dynamic in Phase 4).
    let spacer_w = right_x.saturating_sub(x);
    items.push((
        PanelItem::Spacer,
        PanelItemGeometry {
            x,
            width: spacer_w,
            height: h,
        },
    ));

    items.sort_by_key(|(_, g)| g.x);

    PanelLayout {
        items,
        total_width: screen_width,
        height: h,
    }
}

// ── Wayland client state ──────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct ClientState {
    /// Received at least one xdg_surface.configure.
    configured: bool,
    /// Size requested by the compositor.
    pending_size: Option<(u32, u32)>,
    /// Size currently being rendered.
    width: u32,
    height: u32,
    /// The compositor asked us to close.
    close: bool,
}

/// A wl_shm buffer plus the file backing it.
struct ShmBuffer {
    width: u32,
    height: u32,
    file: File,
    buffer: wayland_client::Main<wl_buffer::WlBuffer>,
    released: Rc<Cell<bool>>,
}

impl ShmBuffer {
    fn write_pixels(&mut self, pixels: &[u8]) -> std::io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(pixels)?;
        self.file.flush()?;
        Ok(())
    }
}

fn create_shm_buffer(shm: &wl_shm::WlShm, width: u32, height: u32) -> Result<ShmBuffer> {
    if width == 0 || height == 0 {
        return Err(anyhow!("cannot create a zero-sized shm buffer"));
    }
    let stride = width
        .checked_mul(4)
        .ok_or_else(|| anyhow!("panel width overflows stride"))?;
    let size = stride
        .checked_mul(height)
        .ok_or_else(|| anyhow!("panel size overflows shm buffer"))?;

    let file = tempfile::tempfile().context("create shared-memory file")?;
    file.set_len(size as u64)?;

    let pool = shm.create_pool(file.as_raw_fd(), size as i32);
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        wl_shm::Format::Argb8888,
    );
    pool.destroy();

    let released = Rc::new(Cell::new(false));
    let released_cb = released.clone();
    buffer.quick_assign(move |_, event, _| {
        if let wl_buffer::Event::Release = event {
            released_cb.set(true);
        }
    });

    Ok(ShmBuffer {
        width,
        height,
        file,
        buffer,
        released,
    })
}

fn next_second_delay() -> Duration {
    let now = Local::now();
    let millis = now.timestamp_subsec_millis();
    Duration::from_millis(1000 - millis as u64)
}

fn wait_for_events(
    display: &Display,
    queue: &mut wayland_client::EventQueue,
    timeout: Duration,
    log: &slog::Logger,
) {
    let mut pfd = libc::pollfd {
        fd: display.get_connection_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;
    let ready = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
    if ready > 0 && (pfd.revents & libc::POLLIN) != 0 {
        if let Some(guard) = queue.prepare_read() {
            if let Err(err) = guard.read_events() {
                warn!(log, "Wayland read failed: {}", err);
            }
        }
    }
}

/// Workspace state published by AeroWM.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ShellState {
    active_workspace: usize,
    workspace_count: usize,
}

impl ShellState {
    fn parse(text: &str) -> Self {
        let mut state = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "active_workspace" => {
                    state.active_workspace = value.trim().parse().unwrap_or(0);
                }
                "workspace_count" => {
                    state.workspace_count = value.trim().parse().unwrap_or(0);
                }
                _ => {}
            }
        }
        state
    }

    /// Read the file published by AeroWM. Missing/unreadable files fall back
    /// to a single-workspace panel rather than failing the whole shell.
    fn read() -> Self {
        let path = runtime_state_path();
        std::fs::read_to_string(path)
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    fn effective_count(&self, fallback: usize) -> usize {
        if self.workspace_count > 0 {
            self.workspace_count
        } else {
            fallback
        }
    }
}

fn runtime_state_path() -> std::path::PathBuf {
    let dir = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) => std::path::PathBuf::from(dir).join("frutigeros"),
        None => {
            let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
            std::env::temp_dir().join(format!("frutigeros-{user}"))
        }
    };
    dir.join("aerowm-state")
}

/// Run the panel. Connects to Wayland and enters the render loop.
pub fn run(config: ShellConfig, log: slog::Logger) -> Result<()> {
    info!(
        log,
        "AeroPanel starting (position: {}, height: {}px)",
        config.panel.position,
        config.panel.height
    );

    let display = Display::connect_to_env()
        .map_err(|e| anyhow!("connect to Wayland display: {}", e))
        .context("is AeroWM running and WAYLAND_DISPLAY set?")?;

    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());
    let globals = GlobalManager::new(&attached_display);

    event_queue
        .sync_roundtrip(&mut (), |_, _, _| {})
        .map_err(|e| anyhow!("initial Wayland roundtrip: {}", e))?;

    let compositor = globals
        .instantiate_exact::<wl_compositor::WlCompositor>(4)
        .map_err(|e| anyhow!("wl_compositor not available: {}", e))?;
    let shm = globals
        .instantiate_exact::<wl_shm::WlShm>(1)
        .map_err(|e| anyhow!("wl_shm not available: {}", e))?;
    let wm_base = globals
        .instantiate_exact::<xdg_wm_base::XdgWmBase>(2)
        .map_err(|e| anyhow!("xdg_wm_base not available: {}", e))?;

    wm_base.quick_assign(|wm_base, event, _| {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    });

    let surface = compositor.create_surface();
    let xdg_surface = wm_base.get_xdg_surface(&surface);
    let xdg_toplevel = xdg_surface.get_toplevel();
    xdg_toplevel.set_app_id(APP_ID.to_string());
    xdg_toplevel.set_title("AeroPanel".to_string());

    let state = Rc::new(RefCell::new(ClientState {
        configured: false,
        pending_size: None,
        width: FALLBACK_WIDTH,
        height: config.panel.height.max(24),
        close: false,
    }));

    let state_for_surface = state.clone();
    let xdg_surface_for_ack = xdg_surface.clone();
    xdg_surface.quick_assign(move |_, event, _| {
        if let xdg_surface::Event::Configure { serial } = event {
            xdg_surface_for_ack.ack_configure(serial);
            state_for_surface.borrow_mut().configured = true;
        }
    });

    let state_for_toplevel = state.clone();
    xdg_toplevel.quick_assign(move |_, event, _| match event {
        xdg_toplevel::Event::Configure { width, height, .. } => {
            let mut st = state_for_toplevel.borrow_mut();
            let cur_w = st.width;
            let cur_h = st.height;
            let w = if width > 0 { width as u32 } else { cur_w };
            let h = if height > 0 { height as u32 } else { cur_h };
            st.pending_size = Some((w, h));
        }
        xdg_toplevel::Event::Close => {
            state_for_toplevel.borrow_mut().close = true;
        }
        _ => {}
    });

    // Initial empty commit requests the first configure.
    surface.commit();
    event_queue
        .sync_roundtrip(&mut (), |_, _, _| {})
        .map_err(|e| anyhow!("Wayland configure roundtrip: {}", e))?;

    info!(log, "Connected as {} (app_id={})", APP_ID, APP_ID);

    let mut buffers: Vec<ShmBuffer> = Vec::new();
    let mut needs_redraw = true;
    let mut last_time = String::new();
    let mut shell_state = ShellState::read();

    loop {
        event_queue
            .dispatch_pending(&mut (), |_, _, _| {})
            .map_err(|e| anyhow!("Wayland dispatch: {}", e))?;

        {
            let mut st = state.borrow_mut();
            if let Some((w, h)) = st.pending_size.take() {
                if w != st.width || h != st.height {
                    debug!(log, "Panel resized to {}×{}", w, h);
                    st.width = w;
                    st.height = h;
                    needs_redraw = true;
                }
            }
            if st.close {
                info!(log, "Compositor requested panel close");
                break;
            }
        }

        let (width, height, configured) = {
            let st = state.borrow();
            (st.width, st.height, st.configured)
        };

        // Follow workspace changes published by AeroWM.
        let current_state = ShellState::read();
        if current_state != shell_state {
            shell_state = current_state;
            needs_redraw = true;
        }

        // Redraw when the size changed, or when the visible minute changed.
        let now = Local::now();
        let time_label = now.format("%H:%M").to_string();
        if time_label != last_time {
            last_time = time_label;
            needs_redraw = true;
        }

        if configured && needs_redraw && width > 0 && height > 0 {
            let workspace_count = shell_state.effective_count(4);
            let active_workspace = shell_state.active_workspace.min(workspace_count - 1);
            let layout = compute_layout(width, &config, workspace_count);
            let Some(pixmap) =
                panel_render::render(width, height, &layout, active_workspace, now)
            else {
                error!(
                    log,
                    "Panel renderer returned no pixmap for {}×{}", width, height
                );
                needs_redraw = false;
                continue;
            };
            let pixels = panel_render::pixmap_to_argb8888(&pixmap);

            let free = buffers
                .iter()
                .position(|b| b.released.get() && b.width == width && b.height == height);
            let index = match free {
                Some(i) => i,
                None => {
                    buffers.push(create_shm_buffer(&shm, width, height)?);
                    buffers.len() - 1
                }
            };

            let buffer = &mut buffers[index];
            if let Err(err) = buffer.write_pixels(&pixels) {
                error!(log, "Failed to write panel pixels: {}", err);
                needs_redraw = false;
                continue;
            }

            surface.attach(Some(&buffer.buffer), 0, 0);
            surface.damage(0, 0, width as i32, height as i32);
            surface.commit();
            display.flush().ok();

            // Drop released buffers that no longer match the panel size.
            buffers.retain(|b| !b.released.get() || (b.width == width && b.height == height));
            needs_redraw = false;
        }

        let timeout = next_second_delay().min(Duration::from_millis(200));
        wait_for_events(&display, &mut event_queue, timeout, &log);
    }

    info!(log, "AeroPanel stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ShellConfig;

    #[test]
    fn layout_covers_full_width() {
        let cfg = ShellConfig::default();
        let layout = compute_layout(1920, &cfg, 4);
        // The last item's right edge should not exceed screen width.
        for (_, geo) in &layout.items {
            assert!(
                geo.x + geo.width <= 1920,
                "item overflows: x={} w={}",
                geo.x,
                geo.width
            );
        }
    }

    #[test]
    fn layout_has_start_and_clock() {
        let cfg = ShellConfig::default();
        let layout = compute_layout(1920, &cfg, 4);
        let has_start = layout
            .items
            .iter()
            .any(|(i, _)| *i == PanelItem::StartButton);
        let has_clock = layout.items.iter().any(|(i, _)| *i == PanelItem::Clock);
        assert!(has_start);
        assert!(has_clock);
    }

    #[test]
    fn layout_has_four_workspaces() {
        let cfg = ShellConfig::default();
        let layout = compute_layout(1920, &cfg, 4);
        let ws_count = layout
            .items
            .iter()
            .filter(|(i, _)| matches!(i, PanelItem::WorkspaceButton { .. }))
            .count();
        assert_eq!(ws_count, 4);
    }

    #[test]
    fn app_id_is_stable() {
        assert_eq!(APP_ID, "frutigeros-aeropanel");
    }
}
