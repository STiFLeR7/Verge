//! Linux/X11 `OverlaySurface` implementation.
//!
//! Mechanism validated empirically by the disposable overlay-capability
//! spike (`D:\overlay-capability-spike\linux-x11`, results summarized in
//! `docs/design/SECOND_VERTICAL_SLICE.md` and the spike's own
//! `SPIKE_RESULTS.md` §3) before being reimplemented here as production
//! code: an override-redirect window on a 32-bit (ARGB) visual for real
//! per-pixel transparency, the X11 Shape extension's input shape to make
//! the whole surface click-through (there is no interactive region in this
//! vertical slice — same deliberate simplification as
//! `platform/windows/src/overlay.rs`), and `_NET_WM_STATE_ABOVE` via EWMH.
//!
//! `core/domain` and `core/ports` know nothing about X11 — no window IDs,
//! atoms, or event masks appear outside this file.
//!
//! **A genuine finding from this slice's own bring-up, not from the
//! spike:** `_NET_WM_STATE_ABOVE` has no effect on an override-redirect
//! window (confirmed by the spike itself — no WM manages such a window at
//! all), and X11's default stacking rule puts whatever is mapped *most
//! recently* on top of older sibling windows, regardless of
//! override-redirect status. In practice this means a window opened after
//! the overlay (e.g. a new terminal) silently ends up drawn over it. The
//! fix, mirroring `platform/windows`'s periodic `HWND_TOPMOST`
//! re-assertion, is a periodic `ConfigureWindow` with `StackMode::ABOVE` on
//! the same refresh cadence as content redraw — not a one-time hint set at
//! creation.

use std::time::{Duration, Instant};

use verge_core::ports::{OverlayContent, OverlaySurface};

use x11rb::connection::Connection;
use x11rb::protocol::shape::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{
    ColormapAlloc, ConfigureWindowAux, ConnectionExt as _, CreateGCAux, CreateWindowAux, EventMask,
    PropMode, Rectangle, StackMode, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

const WINDOW_WIDTH: u16 = 320;
const WINDOW_HEIGHT: u16 = 72;
const SCREEN_MARGIN: i16 = 24;
const REFRESH: Duration = Duration::from_millis(3000);
const POLL_SLEEP: Duration = Duration::from_millis(50);

/// ARGB8888, alpha in the top byte — fully transparent.
const TRANSPARENT: u32 = 0x0000_0000;
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
            .background_pixel(TRANSPARENT)
            .border_pixel(0)
            .colormap(colormap)
            .override_redirect(1)
            .event_mask(EventMask::EXPOSURE),
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

    conn.map_window(window)?;

    // Shape extension: an empty input region makes the entire surface
    // click-through by construction, mirroring this slice's Windows
    // implementation, which also has no interactive region yet.
    conn.shape_rectangles(
        shape::SO::SET,
        shape::SK::INPUT,
        x11rb::protocol::xproto::ClipOrdering::UNSORTED,
        window,
        0,
        0,
        &[] as &[Rectangle],
    )?;

    let gc_clear = conn.generate_id()?;
    conn.create_gc(
        gc_clear,
        window,
        &CreateGCAux::new().foreground(TRANSPARENT),
    )?;

    let font = conn.generate_id()?;
    conn.open_font(font, b"fixed")?;
    let gc_text = conn.generate_id()?;
    conn.create_gc(
        gc_text,
        window,
        &CreateGCAux::new().foreground(TEXT_COLOR).font(font),
    )?;

    conn.flush()?;

    let mut last_refresh = Instant::now() - REFRESH;
    loop {
        while let Some(event) = conn.poll_for_event()? {
            match event {
                x11rb::protocol::Event::Expose(_) => {
                    let content = content_source();
                    draw(&conn, window, gc_clear, gc_text, &content)?;
                }
                x11rb::protocol::Event::Error(e) => eprintln!("[overlay] X error: {e:?}"),
                _ => {}
            }
        }

        if last_refresh.elapsed() >= REFRESH {
            // Re-assert top-of-stack on every refresh tick — see the
            // module doc comment on why a one-time EWMH hint at creation
            // is not enough for an override-redirect window.
            conn.configure_window(
                window,
                &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
            )?;
            let content = content_source();
            draw(&conn, window, gc_clear, gc_text, &content)?;
            last_refresh = Instant::now();
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

    for (i, line) in content.lines.iter().enumerate() {
        let baseline_y = 16 + (i as i16 * 16);
        conn.poly_text8(window, gc_text, 8, baseline_y, &text_item8(line))?;
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
