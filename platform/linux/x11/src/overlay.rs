//! X11 native baseline. Polls pointer without taking keyboard focus; only the visible panel accepts clicks.
//! Local metadata is read off-thread so input and inactivity remain responsive.

use std::time::{Duration, Instant};

use fontdue::{Font, FontSettings};
use verge_core::ports::{OverlayContent, OverlaySurface};

use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::shape::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{
    ColormapAlloc, ConfigureWindowAux, ConnectionExt as _, CreateGCAux, CreateWindowAux, EventMask,
    ImageFormat, PropMode, Rectangle, StackMode, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

use super::interaction::{Interaction, FOOTER, HEIGHT as WINDOW_HEIGHT, WIDTH as WINDOW_WIDTH};
const SCREEN_MARGIN: i16 = 24;
const REFRESH: Duration = Duration::from_millis(3000);
const POLL_SLEEP: Duration = Duration::from_millis(50);

/// ARGB8888, alpha in the top byte — opaque black.
const BACKGROUND: u32 = 0xFF00_0000;
const IDLE_LIGHT: u32 = 0xFFFF_FFFF;

fn work_area(
    conn: &impl Connection,
    root: u32,
    atom: u32,
    fallback: (i32, i32, i32, i32),
) -> (i32, i32, i32, i32) {
    let monitor = conn
        .randr_get_monitors(root, true)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| {
            reply
                .monitors
                .iter()
                .find(|monitor| monitor.primary)
                .or_else(|| reply.monitors.first())
                .map(|monitor| {
                    (
                        i32::from(monitor.x),
                        i32::from(monitor.y),
                        i32::from(monitor.x) + i32::from(monitor.width),
                        i32::from(monitor.y) + i32::from(monitor.height),
                    )
                })
        })
        .unwrap_or(fallback);
    let desktop = conn
        .get_property(
            false,
            root,
            atom,
            x11rb::protocol::xproto::AtomEnum::CARDINAL,
            0,
            4,
        )
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| reply.value32().map(|values| values.collect::<Vec<_>>()))
        .filter(|values| values.len() == 4)
        .map(|values| {
            let left = values[0] as i32;
            let top = values[1] as i32;
            (left, top, left + values[2] as i32, top + values[3] as i32)
        })
        .unwrap_or(fallback);
    let intersection = (
        monitor.0.max(desktop.0),
        monitor.1.max(desktop.1),
        monitor.2.min(desktop.2),
        monitor.3.min(desktop.3),
    );
    if intersection.2 > intersection.0 && intersection.3 > intersection.1 {
        intersection
    } else {
        monitor
    }
}

pub struct X11OverlaySurface;

impl X11OverlaySurface {
    pub fn new() -> Self {
        X11OverlaySurface
    }
}

impl Default for X11OverlaySurface {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlaySurface for X11OverlaySurface {
    fn run(
        self,
        content_source: impl Fn() -> OverlayContent + Send + 'static,
    ) -> std::io::Result<()> {
        run(content_source).map_err(std::io::Error::other)
    }
}

fn run(
    content_source: impl Fn() -> OverlayContent + Send + 'static,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = conn.setup().roots[screen_num].clone();

    // Find a 32-bit-depth (ARGB) visual for real per-pixel transparency —
    // the same requirement the spike's own evidence confirmed necessary
    // for genuine alpha blending, not just "no border".
    let depth32 = screen
        .allowed_depths
        .iter()
        .find(|d| d.depth == 32)
        .ok_or("no 32-bit-depth visual available on this X server")?;
    let visual_id = depth32
        .visuals
        .first()
        .ok_or("32-bit depth has no visuals")?
        .visual_id;

    let colormap = conn.generate_id()?;
    conn.create_colormap(ColormapAlloc::NONE, colormap, screen.root, visual_id)?;

    let net_workarea = conn.intern_atom(false, b"_NET_WORKAREA")?.reply()?.atom;
    let mut area = work_area(
        &conn,
        screen.root,
        net_workarea,
        (
            0,
            0,
            i32::from(screen.width_in_pixels),
            i32::from(screen.height_in_pixels),
        ),
    );
    let window = conn.generate_id()?;
    let mut x = (area.2 - i32::from(WINDOW_WIDTH) - i32::from(SCREEN_MARGIN)) as i16;
    let mut y = (area.3 - i32::from(WINDOW_HEIGHT) - i32::from(SCREEN_MARGIN)) as i16;

    conn.create_window(
        32,
        window,
        screen.root,
        x,
        y,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        0,
        WindowClass::INPUT_OUTPUT,
        visual_id,
        &CreateWindowAux::new()
            .background_pixel(BACKGROUND)
            .border_pixel(0)
            .colormap(colormap)
            .override_redirect(1)
            .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),
    )?;

    // EWMH: ask to be stacked above normal windows. Set before mapping,
    // which the spec permits as the window's requested initial state.
    let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
    let net_wm_state_above = conn
        .intern_atom(false, b"_NET_WM_STATE_ABOVE")?
        .reply()?
        .atom;
    conn.change_property32(
        PropMode::REPLACE,
        window,
        net_wm_state,
        x11rb::protocol::xproto::AtomEnum::ATOM,
        &[net_wm_state_above],
    )?;
    let net_wm_window_type = conn
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE")?
        .reply()?
        .atom;
    let net_wm_window_type_dock = conn
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE_DOCK")?
        .reply()?
        .atom;
    conn.change_property32(
        PropMode::REPLACE,
        window,
        net_wm_window_type,
        x11rb::protocol::xproto::AtomEnum::ATOM,
        &[net_wm_window_type_dock],
    )?;

    conn.change_property8(
        PropMode::REPLACE,
        window,
        x11rb::protocol::xproto::AtomEnum::WM_NAME,
        x11rb::protocol::xproto::AtomEnum::STRING,
        b"Verge",
    )?;
    conn.map_window(window)?;

    let gc = conn.generate_id()?;
    conn.create_gc(gc, window, &CreateGCAux::new())?;
    let font = Font::from_bytes(
        include_bytes!("../../../windows/assets/fonts/Inter-Regular.ttf").as_slice(),
        FontSettings::default(),
    )?;
    let mut idle_bar_color = idle_color();

    conn.flush()?;

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || loop {
        if sender.send(content_source()).is_err() {
            break;
        }
        std::thread::sleep(REFRESH);
    });
    let mut content = OverlayContent::default();
    let mut ui = Interaction::new(Instant::now());
    let mut pointer = None;
    let mut dirty = true;
    let mut last_refresh = Instant::now();
    let mut last_geometry = Instant::now();
    let mut last_theme_refresh = Instant::now();
    loop {
        if let Ok(next) = receiver.try_recv() {
            content = next;
            dirty = true;
        }
        let now = Instant::now();
        if last_geometry.elapsed() >= Duration::from_millis(250) {
            let root = conn.get_geometry(screen.root)?.reply()?;
            area = work_area(
                &conn,
                screen.root,
                net_workarea,
                (0, 0, i32::from(root.width), i32::from(root.height)),
            );
            let next_x = (area.2 - i32::from(WINDOW_WIDTH) - i32::from(SCREEN_MARGIN)) as i16;
            let next_y = (area.3 - i32::from(WINDOW_HEIGHT) - i32::from(SCREEN_MARGIN)) as i16;
            if (x, y) != (next_x, next_y) {
                (x, y) = (next_x, next_y);
                dirty = true;
            }
            last_geometry = now;
        }
        if last_theme_refresh.elapsed() >= REFRESH {
            let next = idle_color();
            dirty |= next != idle_bar_color;
            idle_bar_color = next;
            last_theme_refresh = now;
        }
        let collapsed = ui.collapsed;
        ui.tick(now, &content);
        let position = conn.query_pointer(screen.root)?.reply()?;
        let current = (position.root_x, position.root_y);
        let top = if ui.collapsed {
            y + WINDOW_HEIGHT as i16 - 6
        } else {
            y
        };
        let inside = position.same_screen
            && current.0 >= x
            && current.0 < x + WINDOW_WIDTH as i16
            && current.1 >= top
            && current.1 < y + WINDOW_HEIGHT as i16;
        if pointer != Some(current) && inside {
            ui.touch(now);
        }
        pointer = Some(current);
        dirty |= collapsed != ui.collapsed;
        while let Some(event) = conn.poll_for_event()? {
            match event {
                x11rb::protocol::Event::Expose(_) => dirty = true,
                x11rb::protocol::Event::ButtonPress(event) if event.detail == 1 => {
                    if !ui.collapsed {
                        ui.click(event.event_x, event.event_y, &content, now);
                    } else {
                        ui.touch(now);
                    }
                    dirty = true;
                }
                x11rb::protocol::Event::Error(e) => {
                    return Err(format!("X11 surface: {e:?}").into())
                }
                _ => {}
            }
        }
        if dirty || last_refresh.elapsed() >= REFRESH {
            let height = if ui.collapsed { 6 } else { WINDOW_HEIGHT };
            let top = if ui.collapsed {
                y + WINDOW_HEIGHT as i16 - 6
            } else {
                y
            };
            conn.configure_window(
                window,
                &ConfigureWindowAux::new()
                    .x(i32::from(x))
                    .y(i32::from(top))
                    .height(u32::from(height))
                    .stack_mode(StackMode::ABOVE),
            )?;
            conn.shape_rectangles(
                shape::SO::SET,
                shape::SK::INPUT,
                x11rb::protocol::xproto::ClipOrdering::UNSORTED,
                window,
                0,
                0,
                &[Rectangle {
                    x: 0,
                    y: 0,
                    width: WINDOW_WIDTH,
                    height,
                }],
            )?;
            draw(&conn, window, gc, &font, idle_bar_color, &content, &ui)?;
            last_refresh = now;
            dirty = false;
        }
        std::thread::sleep(POLL_SLEEP);
    }
}

fn draw(
    conn: &impl Connection,
    window: x11rb::protocol::xproto::Window,
    gc: x11rb::protocol::xproto::Gcontext,
    font: &Font,
    idle_color: u32,
    content: &OverlayContent,
    ui: &Interaction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let height = if ui.collapsed { 6 } else { WINDOW_HEIGHT };
    let mut pixels = vec![BACKGROUND; WINDOW_WIDTH as usize * height as usize];

    if ui.collapsed {
        pixels.fill(idle_color);
    } else {
        for (i, glyph) in content.glyphs.iter().enumerate() {
            let x = (i * WINDOW_WIDTH as usize / content.glyphs.len()) as i16 + 8;
            draw_text(&mut pixels, x as i32, 20, &glyph.label, font, 13.0);
        }
        let mut lines = vec![];
        if let Some(g) = content
            .glyphs
            .iter()
            .find(|g| Some(&g.label) == ui.tool.as_ref())
        {
            if let Some((index, session)) = g
                .sessions
                .iter()
                .enumerate()
                .find(|(_, s)| Some(&s.id) == ui.session.as_ref())
            {
                lines.push(session.title.clone());
                lines.extend(session.lines.clone());
                for (x, text) in [
                    (8, "Back".into()),
                    (116, "<".into()),
                    (184, format!("{} / {}", index + 1, g.sessions.len())),
                    (284, ">".into()),
                ] {
                    draw_text(
                        &mut pixels,
                        x as i32,
                        (FOOTER + 20) as i32,
                        &text,
                        font,
                        13.0,
                    );
                }
            } else {
                lines.push(g.activity_label.clone().unwrap_or_else(|| "Unknown".into()));
                for limit in &g.usage_windows {
                    lines.push(limit.label.clone());
                    lines.push(format!(
                        "{:.0}% used  {}",
                        limit.fraction * 100.0,
                        limit.reset
                    ));
                }
                if g.usage_windows.is_empty() {
                    lines.extend(g.detail_lines.clone());
                }
                if !g.sessions.is_empty() {
                    draw_text(
                        &mut pixels,
                        8,
                        (FOOTER + 20) as i32,
                        &format!("{} sessions >", g.sessions.len()),
                        font,
                        13.0,
                    );
                }
            }
        } else {
            lines.push("No local sessions detected".into());
        }
        for (i, line) in lines.iter().take(11).enumerate() {
            draw_text(&mut pixels, 8, 48 + i as i32 * 16, line, font, 13.0);
        }
    }

    let bytes =
        unsafe { std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), pixels.len() * 4) };
    conn.put_image(
        ImageFormat::Z_PIXMAP,
        window,
        gc,
        WINDOW_WIDTH,
        height,
        0,
        0,
        0,
        32,
        bytes,
    )?;
    conn.flush()?;
    Ok(())
}

fn draw_text(pixels: &mut [u32], mut x: i32, baseline: i32, text: &str, font: &Font, size: f32) {
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        let left = x + metrics.xmin;
        let top = baseline - metrics.height as i32 - metrics.ymin;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let px = left + column as i32;
                let py = top + row as i32;
                if px >= 0 && py >= 0 && px < WINDOW_WIDTH as i32 && py < WINDOW_HEIGHT as i32 {
                    let alpha = bitmap[row * metrics.width + column] as u32;
                    let shade = 0xE6 * alpha / 255;
                    pixels[py as usize * WINDOW_WIDTH as usize + px as usize] =
                        0xFF00_0000 | shade << 16 | shade << 8 | shade;
                }
            }
        }
        x += metrics.advance_width.round() as i32;
        if x >= WINDOW_WIDTH as i32 - 8 {
            break;
        }
    }
}

fn idle_color() -> u32 {
    let scheme = std::env::var("VERGE_COLOR_SCHEME").ok().or_else(|| {
        std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
    });
    idle_color_for(scheme.as_deref())
}

fn idle_color_for(scheme: Option<&str>) -> u32 {
    match scheme.map(str::to_ascii_lowercase).as_deref() {
        Some(value) if value.contains("light") => BACKGROUND,
        _ => IDLE_LIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_bar_contrasts_with_light_and_dark_themes() {
        assert_eq!(idle_color_for(Some("prefer-light")), BACKGROUND);
        assert_eq!(idle_color_for(Some("prefer-dark")), IDLE_LIGHT);
        assert_eq!(idle_color_for(None), IDLE_LIGHT);
    }
}
