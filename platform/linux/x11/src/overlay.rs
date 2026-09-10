//! X11 native baseline. Polls pointer without taking keyboard focus; only the visible panel accepts clicks.
//! Local metadata is read off-thread so input and inactivity remain responsive.

use std::time::{Duration, Instant};

use verge_core::ports::{OverlayContent, OverlaySurface};

use x11rb::connection::Connection;
use x11rb::protocol::shape::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{
    ColormapAlloc, ConfigureWindowAux, ConnectionExt as _, CreateGCAux, CreateWindowAux, EventMask,
    PropMode, Rectangle, StackMode, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

use super::interaction::{Interaction, FOOTER, HEIGHT as WINDOW_HEIGHT, WIDTH as WINDOW_WIDTH};
const SCREEN_MARGIN: i16 = 24;
const REFRESH: Duration = Duration::from_millis(3000);
const POLL_SLEEP: Duration = Duration::from_millis(50);

/// ARGB8888, alpha in the top byte — opaque black.
const BACKGROUND: u32 = 0xFF00_0000;
/// Fully opaque light gray, matching `platform/windows`'s `TEXT_COLOR`.
const TEXT_COLOR: u32 = 0xFFE6_E6E6;

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

    let window = conn.generate_id()?;
    let x = screen.width_in_pixels as i16 - WINDOW_WIDTH as i16 - SCREEN_MARGIN;
    let y = screen.height_in_pixels as i16 - WINDOW_HEIGHT as i16 - SCREEN_MARGIN;

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

    conn.change_property8(
        PropMode::REPLACE,
        window,
        x11rb::protocol::xproto::AtomEnum::WM_NAME,
        x11rb::protocol::xproto::AtomEnum::STRING,
        b"Verge",
    )?;
    conn.map_window(window)?;

    let gc_clear = conn.generate_id()?;
    conn.create_gc(gc_clear, window, &CreateGCAux::new().foreground(BACKGROUND))?;

    let font = conn.generate_id()?;
    conn.open_font(font, b"fixed")?;
    let gc_text = conn.generate_id()?;
    conn.create_gc(
        gc_text,
        window,
        &CreateGCAux::new().foreground(TEXT_COLOR).font(font),
    )?;

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
    loop {
        if let Ok(next) = receiver.try_recv() {
            content = next;
            dirty = true;
        }
        let now = Instant::now();
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
            draw(&conn, window, gc_clear, gc_text, &content, &ui)?;
            last_refresh = now;
            dirty = false;
        }
        std::thread::sleep(POLL_SLEEP);
    }
}

fn draw(
    conn: &impl Connection,
    window: x11rb::protocol::xproto::Window,
    gc_clear: x11rb::protocol::xproto::Gcontext,
    gc_text: x11rb::protocol::xproto::Gcontext,
    content: &OverlayContent,
    ui: &Interaction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    conn.poly_fill_rectangle(
        window,
        gc_clear,
        &[Rectangle {
            x: 0,
            y: 0,
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        }],
    )?;

    if ui.collapsed {
        conn.poly_fill_rectangle(
            window,
            gc_text,
            &[Rectangle {
                x: 0,
                y: 0,
                width: WINDOW_WIDTH,
                height: 6,
            }],
        )?;
    } else {
        for (i, glyph) in content.glyphs.iter().enumerate() {
            let x = (i * WINDOW_WIDTH as usize / content.glyphs.len()) as i16 + 8;
            conn.poly_text8(window, gc_text, x, 20, &text_item8(&glyph.label))?;
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
                    conn.poly_text8(window, gc_text, x, FOOTER + 20, &text_item8(&text))?;
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
                    conn.poly_text8(
                        window,
                        gc_text,
                        8,
                        FOOTER + 20,
                        &text_item8(&format!("{} sessions >", g.sessions.len())),
                    )?;
                }
            }
        } else {
            lines.push("No local sessions detected".into());
        }
        for (i, line) in lines.iter().take(11).enumerate() {
            conn.poly_text8(window, gc_text, 8, 48 + i as i16 * 16, &text_item8(line))?;
        }
    }

    conn.flush()?;
    Ok(())
}

/// Encodes one `PolyText8` `TEXTITEM8`: a length-prefixed 8-bit string with
/// a (zero) horizontal delta — `x11rb` has no builder for this, its
/// `items` field is the raw wire bytes (confirmed the hard way: passing
/// UTF-8 text directly there produces a `BadLength` X error, since the
/// first byte is read as the item's declared length, not text). `core`
/// fonts are Latin-1, so non-Latin-1 characters (e.g. the `·` this
/// surface's own `render()` uses) are re-encoded per `char`, not as raw
/// UTF-8 bytes, or they render as garbage/mismatched-length glyphs.
fn text_item8(s: &str) -> Vec<u8> {
    let latin1: Vec<u8> = s
        .chars()
        .map(|c| if (c as u32) <= 0xFF { c as u8 } else { b'?' })
        .take(254)
        .collect();
    let mut item = Vec::with_capacity(latin1.len() + 2);
    item.push(latin1.len() as u8);
    item.push(0); // delta
    item.extend_from_slice(&latin1);
    item
}
