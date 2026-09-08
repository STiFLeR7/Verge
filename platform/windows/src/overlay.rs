//! Windows `OverlaySurface` implementation: the right-edge vertical capsule
//! from `docs/design/VERGE_AMBIENT_DESIGN.md`, rendered with real
//! per-pixel alpha via `UpdateLayeredWindow` (not the first slice's
//! color-key transparency) so the material can have soft rounded corners,
//! a translucent gradient, and a hairline edge highlight instead of a hard
//! rectangular cutout. See `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md`
//! for the full mapping from design spec to this file.
//!
//! Win32 mechanics preserved from the first vertical slice's validated
//! findings (`SPIKE_RESULTS.md` §2/§10): `WS_EX_LAYERED | WS_EX_TOPMOST |
//! WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`, Per-Monitor-V2 DPI awareness, and
//! periodic `HWND_TOPMOST` re-assertion using `SWP_NOMOVE | SWP_NOSIZE`
//! (the real bug the first slice found and fixed).
//!
//! **Interaction model, and why the validated `WS_EX_TRANSPARENT` toggle is
//! not re-engaged this pass:** the design's compact/expanded behavior
//! (§17) is driven by hover alone — nothing here needs to intercept a
//! click yet. Hover detection is a plain `GetCursorPos` poll against
//! `GetWindowRect`, which works regardless of `WS_EX_TRANSPARENT` (that
//! flag only affects whether *our window* receives mouse messages, not
//! whether we can query the cursor's position globally). `WS_EX_TRANSPARENT`
//! is therefore held on permanently, exactly as the first slice did when it
//! had no interactive region — this is not a regression of the validated
//! toggle mechanism, it is the same "nothing to click yet" case, now reused
//! for a surface that also happens to expand on hover. The mechanism the
//! spike proved (toggling driven by an independent cursor poll) is the
//! direct ancestor of the hover poll below; it becomes necessary again,
//! unchanged in principle, the day a real click target is added.

use std::cell::Cell;
use std::sync::Mutex;

use verge_core::ports::{OverlayContent, OverlaySurface, StateTint};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject, SetBkColor, SetBkMode, SetTextColor, TextOutW, AC_SRC_ALPHA, AC_SRC_OVER,
    ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    FF_DONTCARE, FW_SEMIBOLD, HBITMAP, HDC, HFONT, HGDIOBJ, OPAQUE, OUT_DEFAULT_PRECIS,
    PROOF_QUALITY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, GetWindowRect, LoadCursorW, PostQuitMessage, RegisterClassExW, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, UpdateLayeredWindow,
    HWND_TOPMOST, IDC_ARROW, MSG, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SW_SHOWNOACTIVATE, ULW_ALPHA, WM_DESTROY, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

// ---- Geometry constants (design spec §3, §18) ----

/// Diameter of a tool's identity badge circle.
const GLYPH_DIAMETER: i32 = 40;
/// Outer radius of the static "there is a metric here" ring (design §9),
/// drawn around the badge, not as a fill.
const RING_RADIUS: i32 = 26;
const RING_THICKNESS: i32 = 2;
/// Vertical footprint reserved per glyph, including its ring.
const GLYPH_SLOT_HEIGHT: i32 = 58;
const GLYPH_GAP_V: i32 = 8;
/// Width of the compact (collapsed) capsule — just enough for one glyph
/// column, flush against the screen edge.
const GLYPH_COLUMN_WIDTH: i32 = 74;
/// Left-corner-only radius (design §3: "screen-facing edge is flush").
const CORNER_RADIUS: f32 = 22.0;
const CAPSULE_PAD_V: i32 = 16;

/// Idle-state sliver (design §4) — no glyphs, nearly invisible.
const IDLE_WIDTH: i32 = 5;
const IDLE_HEIGHT: i32 = 64;
const IDLE_ALPHA: u8 = 60;

const TEXT_COLUMN_MAX_WIDTH: i32 = 210;
const TEXT_COLUMN_PAD: i32 = 14;
const OVERFLOW_ROW_HEIGHT: i32 = 24;

/// Material base color — neutral, near-black (design §14: Verge's own
/// material stays colorless so tool identity and state tints remain the
/// only chromatic information).
const MATERIAL_RGB: (u8, u8, u8) = (14, 14, 16);
/// Horizontal translucency gradient: denser near the screen edge (right),
/// lighter toward the inner edge (left) — design §14's "very slightly more
/// translucent toward the inner edge."
const MATERIAL_ALPHA_EDGE: f32 = 238.0;
const MATERIAL_ALPHA_INNER: f32 = 205.0;
const NEUTRAL_RING_RGB: (u8, u8, u8) = (255, 255, 255);
const NEUTRAL_RING_ALPHA: f32 = 0.22;
const HIGHLIGHT_RGB: (u8, u8, u8) = (255, 255, 255);
const HIGHLIGHT_ALPHA: f32 = 0.10;
const TEXT_PRIMARY_RGB: (u8, u8, u8) = (235, 235, 235);
const TEXT_SECONDARY_RGB: (u8, u8, u8) = (150, 150, 150);
const DIM_MULTIPLIER: f32 = 0.55;

const TIMER_REFRESH: usize = 1;
const TIMER_HOVER: usize = 2;
const REFRESH_MS: u32 = 3000;
const HOVER_POLL_MS: u32 = 50;

fn tint_rgb(state: StateTint) -> Option<(u8, u8, u8)> {
    match state {
        StateTint::Neutral => None,
        StateTint::Working => Some((79, 209, 139)),
        StateTint::Waiting => Some((255, 157, 61)),
        StateTint::Completed => Some((255, 77, 77)),
    }
}

struct WindowState<F: Fn() -> OverlayContent> {
    content_source: F,
    current: Mutex<OverlayContent>,
    expanded: Cell<bool>,
}

pub struct WindowsOverlaySurface;

impl WindowsOverlaySurface {
    pub fn new() -> Self {
        WindowsOverlaySurface
    }
}

impl Default for WindowsOverlaySurface {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlaySurface for WindowsOverlaySurface {
    fn run(
        self,
        content_source: impl Fn() -> OverlayContent + Send + 'static,
    ) -> std::io::Result<()> {
        unsafe { run_message_loop(content_source) }
    }
}

unsafe fn run_message_loop(
    content_source: impl Fn() -> OverlayContent + Send + 'static,
) -> std::io::Result<()> {
    // Per-Monitor-V2 DPI awareness: without this, Windows silently
    // virtualizes coordinates for this process (the same bug the first
    // slice's own PowerShell test harness hit before declaring this
    // context).
    let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

    let instance = GetModuleHandleW(PCWSTR::null())
        .map_err(|e| std::io::Error::other(format!("GetModuleHandleW: {e}")))?;

    let class_name = to_wide("VergeAmbientSurface");
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        ..Default::default()
    };
    if RegisterClassExW(&wc) == 0 {
        return Err(std::io::Error::last_os_error());
    }

    // Boxed into a concrete `Box<dyn Fn...>` *before* going into
    // `WindowState`, so the struct's actual memory layout matches the type
    // `window_proc` casts the raw pointer back to below. Skipping this
    // step is a real, genuine bug this implementation hit during its own
    // bring-up: without it, `WindowState<F>` (F = the real, opaque closure
    // type) and `WindowState<Box<dyn Fn() -> OverlayContent>>` have
    // different field layouts, and reading through the wrong one is
    // undefined behavior — it manifested as nonsensical `Cell<bool>`
    // values and an eventual silent crash, not a clean panic.
    let boxed_source: Box<dyn Fn() -> OverlayContent> = Box::new(content_source);
    let state = std::sync::Arc::new(WindowState {
        content_source: boxed_source,
        current: Mutex::new(OverlayContent::default()),
        expanded: Cell::new(false),
    });
    let state_ptr = std::sync::Arc::into_raw(state);

    // Created at a nominal idle size; the first content refresh (fired
    // immediately below) repositions/resizes it via UpdateLayeredWindow to
    // whatever the real content actually requires.
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        PCWSTR(class_name.as_ptr()),
        PCWSTR(to_wide("Verge").as_ptr()),
        WS_POPUP,
        0,
        0,
        IDLE_WIDTH,
        IDLE_HEIGHT,
        None,
        None,
        instance,
        Some(state_ptr as *const std::ffi::c_void),
    )
    .map_err(|e| std::io::Error::other(format!("CreateWindowExW: {e}")))?;

    SetWindowLongPtrW(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        state_ptr as isize,
    );

    // Held on permanently — see the module doc comment's "Interaction
    // model" note on why the validated dynamic toggle isn't re-engaged
    // this pass.
    let current_ex_style =
        GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE);
    SetWindowLongPtrW(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE,
        current_ex_style | (WS_EX_TRANSPARENT.0 as isize),
    );

    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
    );
    let _ = SetTimer(hwnd, TIMER_REFRESH, REFRESH_MS, None);
    let _ = SetTimer(hwnd, TIMER_HOVER, HOVER_POLL_MS, None);

    // Render once immediately rather than waiting for the first timer tick.
    let state_ref = &*(state_ptr);
    let fresh = (state_ref.content_source)();
    *state_ref.current.lock().unwrap() = fresh;
    present(hwnd, state_ref);

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    drop(std::sync::Arc::from_raw(state_ptr));

    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TIMER => {
            let ptr =
                GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA);
            if ptr == 0 {
                return LRESULT(0);
            }
            let state = &*(ptr as *const WindowState<Box<dyn Fn() -> OverlayContent>>);

            if wparam.0 == TIMER_REFRESH {
                let fresh = (state.content_source)();
                *state.current.lock().unwrap() = fresh;
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
                );
                present(hwnd, state);
            } else if wparam.0 == TIMER_HOVER {
                let mut cursor = POINT::default();
                let _ = GetCursorPos(&mut cursor);
                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                let inside = cursor.x >= rect.left
                    && cursor.x < rect.right
                    && cursor.y >= rect.top
                    && cursor.y < rect.bottom;
                if inside != state.expanded.get() {
                    state.expanded.set(inside);
                    present(hwnd, state);
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr =
                GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA);
            if ptr != 0 {
                drop(std::sync::Arc::from_raw(
                    ptr as *const WindowState<Box<dyn Fn() -> OverlayContent>>,
                ));
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ---- Layout ----

struct Layout {
    width: i32,
    height: i32,
    text_column_width: i32,
    /// Top-left of each visible glyph's slot, glyph-column-relative.
    glyph_tops: Vec<i32>,
    overflow_row_top: Option<i32>,
}

fn compute_layout(content: &OverlayContent, expanded: bool, hdc_for_measure: HDC) -> Layout {
    if content.glyphs.is_empty() {
        return Layout {
            width: IDLE_WIDTH,
            height: IDLE_HEIGHT,
            text_column_width: 0,
            glyph_tops: vec![],
            overflow_row_top: None,
        };
    }

    let n = content.glyphs.len() as i32;
    let mut height = CAPSULE_PAD_V * 2 + n * GLYPH_SLOT_HEIGHT + (n - 1).max(0) * GLYPH_GAP_V;
    let overflow_row_top = if content.overflow_count.is_some() {
        let top = CAPSULE_PAD_V + n * GLYPH_SLOT_HEIGHT + n * GLYPH_GAP_V;
        height += OVERFLOW_ROW_HEIGHT + GLYPH_GAP_V;
        Some(top)
    } else {
        None
    };

    let glyph_tops: Vec<i32> = (0..content.glyphs.len())
        .map(|i| CAPSULE_PAD_V + i as i32 * (GLYPH_SLOT_HEIGHT + GLYPH_GAP_V))
        .collect();

    let text_column_width = if expanded {
        let longest = content
            .glyphs
            .iter()
            .flat_map(|g| std::iter::once(&g.label).chain(g.detail_lines.iter()))
            .map(|s| text_width(hdc_for_measure, s, 14, false))
            .max()
            .unwrap_or(0);
        (longest + TEXT_COLUMN_PAD * 2).min(TEXT_COLUMN_MAX_WIDTH)
    } else {
        0
    };

    Layout {
        width: text_column_width + GLYPH_COLUMN_WIDTH,
        height,
        text_column_width,
        glyph_tops,
        overflow_row_top,
    }
}

// ---- Presentation ----

fn present<F: Fn() -> OverlayContent>(hwnd: HWND, state: &WindowState<F>) {
    unsafe {
        let screen_dc = GetDC(None);
        let content = state.current.lock().unwrap();
        let layout = compute_layout(&content, state.expanded.get(), screen_dc);

        let (bitmap, mem_dc, pixels) = match create_argb_dib(screen_dc, layout.width, layout.height)
        {
            Some(v) => v,
            None => {
                let _ = ReleaseDC(None, screen_dc);
                return;
            }
        };

        let buf = std::slice::from_raw_parts_mut(pixels, (layout.width * layout.height) as usize);
        buf.fill(0);

        if content.glyphs.is_empty() {
            paint_idle(buf, layout.width, layout.height);
        } else {
            paint_capsule(buf, &layout, &content, mem_dc);
        }

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = screen_w - layout.width;
        // Vertically centered with a slight upward bias (design §3).
        let y = (screen_h - layout.height) / 2 - (screen_h / 12);

        let src_point = POINT { x: 0, y: 0 };
        let dst_point = POINT { x, y };
        let size = SIZE {
            cx: layout.width,
            cy: layout.height,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let _ = UpdateLayeredWindow(
            hwnd,
            screen_dc,
            Some(&dst_point),
            Some(&size),
            mem_dc,
            Some(&src_point),
            windows::Win32::Foundation::COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        let _ = SelectObject(mem_dc, HGDIOBJ::default());
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
    }
}

unsafe fn create_argb_dib(
    screen_dc: HDC,
    width: i32,
    height: i32,
) -> Option<(HBITMAP, HDC, *mut u32)> {
    let mem_dc = CreateCompatibleDC(screen_dc);
    if mem_dc.is_invalid() {
        return None;
    }

    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        // Negative height: top-down DIB, so row 0 is the top row (matches
        // how the layout math above addresses `buf` top-to-bottom).
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };

    let mut pixels: *mut std::ffi::c_void = std::ptr::null_mut();
    let bitmap = match CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut pixels, None, 0) {
        Ok(b) => b,
        Err(_) => {
            let _ = DeleteDC(mem_dc);
            return None;
        }
    };
    if pixels.is_null() {
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        return None;
    }
    SelectObject(mem_dc, bitmap);

    Some((bitmap, mem_dc, pixels as *mut u32))
}

fn paint_idle(buf: &mut [u32], w: i32, h: i32) {
    for y in 0..h {
        for x in 0..w {
            let cov = rounded_rect_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                w as f32,
                h as f32,
                2.0,
                0.0,
                0.0,
                2.0,
            );
            let alpha = cov * (IDLE_ALPHA as f32 / 255.0);
            buf[(y * w + x) as usize] = composite(0, MATERIAL_RGB, alpha);
        }
    }
}

fn paint_capsule(buf: &mut [u32], layout: &Layout, content: &OverlayContent, mem_dc: HDC) {
    let w = layout.width;
    let h = layout.height;

    // 1. Material: rounded on the two LEFT corners only, flush/square on
    // the right (screen-facing) edge — design §3.
    for y in 0..h {
        for x in 0..w {
            let cov = rounded_rect_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                w as f32,
                h as f32,
                CORNER_RADIUS,
                0.0,
                0.0,
                CORNER_RADIUS,
            );
            if cov <= 0.0 {
                continue;
            }
            let t = x as f32 / w.max(1) as f32; // 0 at inner/left edge, 1 at screen edge
            let alpha =
                (MATERIAL_ALPHA_INNER + (MATERIAL_ALPHA_EDGE - MATERIAL_ALPHA_INNER) * t) / 255.0;
            let idx = (y * w + x) as usize;
            buf[idx] = composite(buf[idx], MATERIAL_RGB, cov * alpha);
        }
    }

    // 2. Hairline highlight on the outer (right) edge only.
    for y in 0..h {
        let x = w - 1;
        let idx = (y * w + x) as usize;
        buf[idx] = composite(buf[idx], HIGHLIGHT_RGB, HIGHLIGHT_ALPHA);
    }

    let glyph_col_left = layout.text_column_width;

    for (i, glyph) in content.glyphs.iter().enumerate() {
        let top = layout.glyph_tops[i];
        let cx = glyph_col_left + GLYPH_COLUMN_WIDTH / 2;
        let cy = top + GLYPH_SLOT_HEIGHT / 2;

        // Neutral identity ring — never a percentage arc unless a real
        // `Fraction` reading exists (design §9's anti-fabrication rule;
        // enforced upstream in `ui/ambient/windows`, this file just draws
        // whatever `has_metric` says).
        if glyph.has_metric {
            paint_ring(
                buf,
                w,
                h,
                cx,
                cy,
                RING_RADIUS,
                RING_THICKNESS,
                NEUTRAL_RING_RGB,
                NEUTRAL_RING_ALPHA,
            );
        }

        // State tint: a thin ring just outside the badge, localized to
        // this one glyph only (design §13 — never a full-surface wash).
        if let Some(tint) = tint_rgb(glyph.state) {
            paint_ring(
                buf,
                w,
                h,
                cx,
                cy,
                RING_RADIUS + RING_THICKNESS + 3,
                2,
                tint,
                0.85,
            );
        }

        // Identity badge — the tool's own real brand color.
        let badge_alpha = if glyph.dimmed { DIM_MULTIPLIER } else { 1.0 };
        paint_filled_circle(
            buf,
            w,
            h,
            cx,
            cy,
            GLYPH_DIAMETER / 2,
            glyph.brand_color,
            badge_alpha,
        );

        // Identity mark, centered in the badge, via the luminance-alpha
        // GDI text trick (no vector logo assets exist yet — see
        // docs/design/VERGE_AMBIENT_IMPLEMENTATION.md).
        let mark = glyph.mark.to_string();
        draw_text_alpha(
            buf,
            w,
            h,
            mem_dc,
            &mark,
            cx - 12,
            cy - 12,
            24,
            24,
            20,
            true,
            (255, 255, 255),
            badge_alpha,
        );

        if layout.text_column_width > 0 {
            let text_x = TEXT_COLUMN_PAD;
            let mut text_y = top + 6;
            draw_text_alpha(
                buf,
                w,
                h,
                mem_dc,
                &glyph.label,
                text_x,
                text_y,
                layout.text_column_width - TEXT_COLUMN_PAD,
                16,
                14,
                true,
                TEXT_PRIMARY_RGB,
                1.0,
            );
            text_y += 20;
            for line in &glyph.detail_lines {
                let alpha = if glyph.dimmed { DIM_MULTIPLIER } else { 1.0 };
                draw_text_alpha(
                    buf,
                    w,
                    h,
                    mem_dc,
                    line,
                    text_x,
                    text_y,
                    layout.text_column_width - TEXT_COLUMN_PAD,
                    16,
                    11,
                    false,
                    TEXT_SECONDARY_RGB,
                    alpha,
                );
                text_y += 15;
            }
        }
    }

    if let (Some(count), Some(row_top)) = (content.overflow_count, layout.overflow_row_top) {
        let cx = glyph_col_left + GLYPH_COLUMN_WIDTH / 2;
        let cy = row_top + OVERFLOW_ROW_HEIGHT / 2;
        let label = format!("+{count}");
        draw_text_alpha(
            buf,
            w,
            h,
            mem_dc,
            &label,
            cx - 16,
            cy - 8,
            32,
            16,
            12,
            true,
            TEXT_SECONDARY_RGB,
            1.0,
        );
    }
}

// ---- Rasterization primitives ----

/// Rounded-rectangle coverage (0.0..=1.0) at pixel center `(x, y)`, with
/// independent per-corner radii (top-left, top-right, bottom-right,
/// bottom-left) — used with the right two radii at 0 to get the
/// flush-screen-edge shape design spec §3 asks for.
fn rounded_rect_coverage(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r_tl: f32,
    r_tr: f32,
    r_br: f32,
    r_bl: f32,
) -> f32 {
    let (cx, cy) = (w * 0.5, h * 0.5);
    let (px, py) = (x - cx, y - cy);
    let r = if px > 0.0 {
        if py > 0.0 {
            r_br
        } else {
            r_tr
        }
    } else if py > 0.0 {
        r_bl
    } else {
        r_tl
    };
    let qx = px.abs() - (cx - r);
    let qy = py.abs() - (cy - r);
    let dist = qx.max(qy).min(0.0) + (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() - r;
    (0.5 - dist).clamp(0.0, 1.0)
}

fn circle_coverage(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> f32 {
    let dist = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r;
    (0.5 - dist).clamp(0.0, 1.0)
}

fn ring_coverage(x: f32, y: f32, cx: f32, cy: f32, r: f32, thickness: f32) -> f32 {
    let dist = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    let d = (dist - r).abs();
    (thickness / 2.0 - d + 0.5).clamp(0.0, 1.0)
}

fn paint_filled_circle(
    buf: &mut [u32],
    w: i32,
    h: i32,
    cx: i32,
    cy: i32,
    r: i32,
    color: (u8, u8, u8),
    alpha_mult: f32,
) {
    let (x0, x1) = ((cx - r - 2).max(0), (cx + r + 2).min(w));
    let (y0, y1) = ((cy - r - 2).max(0), (cy + r + 2).min(h));
    for y in y0..y1 {
        for x in x0..x1 {
            let cov = circle_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                cx as f32,
                cy as f32,
                r as f32,
            );
            if cov <= 0.0 {
                continue;
            }
            let idx = (y * w + x) as usize;
            buf[idx] = composite(buf[idx], color, cov * alpha_mult);
        }
    }
}

fn paint_ring(
    buf: &mut [u32],
    w: i32,
    h: i32,
    cx: i32,
    cy: i32,
    r: i32,
    thickness: i32,
    color: (u8, u8, u8),
    alpha_mult: f32,
) {
    let pad = thickness + 2;
    let (x0, x1) = ((cx - r - pad).max(0), (cx + r + pad).min(w));
    let (y0, y1) = ((cy - r - pad).max(0), (cy + r + pad).min(h));
    for y in y0..y1 {
        for x in x0..x1 {
            let cov = ring_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                cx as f32,
                cy as f32,
                r as f32,
                thickness as f32,
            );
            if cov <= 0.0 {
                continue;
            }
            let idx = (y * w + x) as usize;
            buf[idx] = composite(buf[idx], color, cov * alpha_mult);
        }
    }
}

/// Composites a solid `color` at fractional `coverage` onto premultiplied
/// pixel `dst` using the standard premultiplied "over" operator.
fn composite(dst: u32, color: (u8, u8, u8), coverage: f32) -> u32 {
    if coverage <= 0.0 {
        return dst;
    }
    let cov = coverage.min(1.0);
    let da = ((dst >> 24) & 0xFF) as f32;
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    let sa = cov * 255.0;
    let sr = color.0 as f32 * cov;
    let sg = color.1 as f32 * cov;
    let sb = color.2 as f32 * cov;
    let inv = 1.0 - cov;
    let oa = (sa + da * inv).clamp(0.0, 255.0) as u32;
    let or = (sr + dr * inv).clamp(0.0, 255.0) as u32;
    let og = (sg + dg * inv).clamp(0.0, 255.0) as u32;
    let ob = (sb + db * inv).clamp(0.0, 255.0) as u32;
    (oa << 24) | (or << 16) | (og << 8) | ob
}

// ---- Text (luminance-as-alpha trick, so anti-aliased GDI text can be
// composited onto our own alpha buffer without GDI ever knowing about
// alpha itself) ----

fn make_font(size_px: i32, bold: bool) -> HFONT {
    unsafe {
        let name = to_wide("Segoe UI");
        CreateFontW(
            size_px,
            0,
            0,
            0,
            if bold { FW_SEMIBOLD.0 as i32 } else { 400 },
            0,
            0,
            0,
            windows::Win32::Graphics::Gdi::DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            windows::Win32::Graphics::Gdi::CLIP_DEFAULT_PRECIS.0 as u32,
            PROOF_QUALITY.0 as u32 | ANTIALIASED_QUALITY.0 as u32,
            FF_DONTCARE.0 as u32,
            PCWSTR(name.as_ptr()),
        )
    }
}

fn text_width(hdc: HDC, text: &str, size_px: i32, bold: bool) -> i32 {
    unsafe {
        let font = make_font(size_px, bold);
        let prev = SelectObject(hdc, font);
        let wide = to_wide(text);
        let mut size = SIZE::default();
        let _ = windows::Win32::Graphics::Gdi::GetTextExtentPoint32W(
            hdc,
            &wide[..wide.len() - 1],
            &mut size,
        );
        SelectObject(hdc, prev);
        let _ = DeleteObject(font);
        size.cx
    }
}

/// Renders `text` at `size_px` into a scratch black/white bitmap, then
/// composites it onto `buf` at `(x, y)` in `color`, using the scratch
/// bitmap's luminance as the per-pixel alpha — the standard technique for
/// getting anti-aliased GDI text onto a manually alpha-blended surface.
#[allow(clippy::too_many_arguments)]
fn draw_text_alpha(
    buf: &mut [u32],
    dst_w: i32,
    dst_h: i32,
    mem_dc: HDC,
    text: &str,
    x: i32,
    y: i32,
    box_w: i32,
    box_h: i32,
    size_px: i32,
    bold: bool,
    color: (u8, u8, u8),
    alpha_mult: f32,
) {
    if text.is_empty() || box_w <= 0 || box_h <= 0 {
        return;
    }
    unsafe {
        let (scratch_bitmap, scratch_dc, scratch_ptr) = match create_argb_dib(mem_dc, box_w, box_h)
        {
            Some(v) => v,
            None => return,
        };
        let scratch = std::slice::from_raw_parts_mut(scratch_ptr, (box_w * box_h) as usize);
        scratch.fill(0); // black background => 0 luminance => 0 alpha by default

        let font = make_font(size_px, bold);
        let prev_font = SelectObject(scratch_dc, font);
        SetBkColor(scratch_dc, windows::Win32::Foundation::COLORREF(0x00000000));
        SetBkMode(scratch_dc, OPAQUE);
        SetTextColor(scratch_dc, windows::Win32::Foundation::COLORREF(0x00FFFFFF));
        let wide = to_wide(text);
        let _ = TextOutW(scratch_dc, 0, 0, &wide[..wide.len() - 1]);
        SelectObject(scratch_dc, prev_font);
        let _ = DeleteObject(font);

        for row in 0..box_h {
            for col in 0..box_w {
                let px = scratch[(row * box_w + col) as usize];
                let luminance = (px & 0xFF) as f32; // R, G, B are equal for white-on-black text
                if luminance <= 0.0 {
                    continue;
                }
                let dst_x = x + col;
                let dst_y = y + row;
                if dst_x < 0 || dst_y < 0 || dst_x >= dst_w || dst_y >= dst_h {
                    continue;
                }
                let idx = (dst_y * dst_w + dst_x) as usize;
                let coverage = (luminance / 255.0) * alpha_mult;
                buf[idx] = composite(buf[idx], color, coverage);
            }
        }

        let _ = DeleteObject(scratch_bitmap);
        let _ = DeleteDC(scratch_dc);
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
