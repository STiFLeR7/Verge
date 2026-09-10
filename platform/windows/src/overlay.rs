//! THESIS: One black instrument grows from the display edge.
//! OWN-WORLD: Inverse edge curves, bare tool marks, local state light, Segoe UI.
//! STORY: Glance at identity and state; hover to reveal only that identity's detail.
//! FIRST VIEWPORT: An 84 DIP rail, 52 DIP instrument, fixed right anchor; idle is a sliver.
//! FORM: User-pinned right-edge composition, continuous contextual lobe, no spring.
//! FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, and DESIGN.md
//!
//! Layered, topmost, no-activate Win32 window. The cursor poll dynamically toggles
//! WS_EX_TRANSPARENT using the same silhouette that is painted, never HTTRANSPARENT.

use std::cell::Cell;
use std::f32::consts::TAU;
use std::sync::Mutex;
use std::time::Instant;

use verge_core::ports::{Metric, OverlayContent, OverlaySurface, StateTint};

use crate::tokens::{self, MotionToken};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW, GdiFlush,
    GetDC, ReleaseDC, SelectObject, SetBkColor, SetBkMode, SetTextColor, AC_SRC_ALPHA, AC_SRC_OVER,
    ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, FF_DONTCARE, FW_SEMIBOLD, HBITMAP, HDC, HFONT,
    OPAQUE, OUT_DEFAULT_PRECIS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, GetWindowRect, KillTimer, LoadCursorW, PostQuitMessage, RegisterClassExW,
    SetTimer, SetWindowLongPtrW, SetWindowPos, SystemParametersInfoW, TranslateMessage,
    UpdateLayeredWindow, HWND_TOPMOST, IDC_ARROW, MSG, SM_CXSCREEN, SM_CYSCREEN,
    SPI_GETCLIENTAREAANIMATION, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, ULW_ALPHA, WM_DESTROY, WM_TIMER, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

fn tint_rgb(state: StateTint) -> Option<(u8, u8, u8)> {
    match state {
        StateTint::Neutral => None,
        StateTint::Working => Some(tokens::STATE_WORKING),
        StateTint::Waiting => Some(tokens::STATE_WAITING),
        StateTint::Completed => Some(tokens::STATE_COMPLETED),
        StateTint::Stopped => Some(tokens::STATE_STOPPED),
    }
}

fn relevant_usage(glyph: &verge_core::ports::ToolGlyph) -> Option<&verge_core::ports::UsageDetail> {
    glyph
        .usage_windows
        .iter()
        .max_by(|a, b| a.fraction.total_cmp(&b.fraction))
}
fn usage_rows(
    g: &verge_core::ports::ToolGlyph,
) -> Vec<(&str, Option<&verge_core::ports::UsageDetail>)> {
    if g.label == "Claude" {
        vec![
            (
                "5-hour limit",
                g.usage_windows
                    .iter()
                    .find(|w| w.label == "Current session" || w.label == "5-hour limit"),
            ),
            (
                "Weekly limit",
                g.usage_windows
                    .iter()
                    .find(|w| w.label == "All models" || w.label == "Weekly limit"),
            ),
        ]
    } else {
        relevant_usage(g)
            .map(|w| vec![(w.label.as_str(), Some(w))])
            .unwrap_or_default()
    }
}

// Interpolate only readings of the same identity/window. Missing data never animates from zero.
fn interpolate_usage(from: &OverlayContent, to: &OverlayContent, t: f32) -> OverlayContent {
    let mut result = to.clone();
    let t = t.clamp(0.0, 1.0);
    if t == 1.0 {
        return result;
    }
    for (old, new) in from.glyphs.iter().zip(&mut result.glyphs) {
        if old.label != new.label {
            continue;
        }
        if let (Metric::Fraction(a), Metric::Fraction(b)) = (old.metric, new.metric) {
            new.metric = Metric::Fraction(a + (b - a) * t);
        }
        for window in &mut new.usage_windows {
            if let Some(previous) = old.usage_windows.iter().find(|w| w.label == window.label) {
                window.fraction = previous.fraction + (window.fraction - previous.fraction) * t;
            }
        }
    }
    result
}

/// True when Windows' "Ease of Access > Show animations" setting is
/// switched off — the standard system-wide reduced-motion signal on this
/// platform (design spec §16/§20's "respect OS reduce motion", a hard
/// requirement, not a nice-to-have). Read once at startup: nothing in this
/// surface's own lifetime needs to react to the user flipping it live.
fn reduced_motion_enabled() -> bool {
    unsafe {
        let mut animations_enabled = BOOL(1);
        let ptr = &mut animations_enabled as *mut BOOL as *mut std::ffi::c_void;
        let ok = SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(ptr),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        ok.is_ok() && !animations_enabled.as_bool()
    }
}

struct WindowState<F: Fn() -> OverlayContent> {
    content_source: F,
    permissions: Option<std::sync::Arc<dyn verge_core::ports::PermissionService>>,
    painted_permission: Cell<Option<u64>>,
    resolved_permission: Cell<Option<u64>>,
    pressed_permission: Cell<Option<(u64, u8)>>,
    keyboard_approve: Cell<bool>,
    keyboard_mode: Cell<bool>,
    permission_visible_since: Cell<Instant>,
    current: Mutex<OverlayContent>,
    usage_from: Mutex<OverlayContent>,
    usage_start: Cell<Instant>,
    painted_layout: std::cell::RefCell<Option<Layout>>,
    /// True while the cursor is over the surface's current rect.
    hovered: Cell<bool>,
    open_requested: Cell<bool>,
    /// Compact/expanded morph progress, 0.0 (collapsed) .. 1.0 (expanded).
    anim_from: Cell<f32>,
    anim_to: Cell<f32>,
    anim_start: Cell<Instant>,
    reduced_motion: bool,
    hover_since: Cell<Instant>,
    last_interaction: Cell<Instant>,
    last_cursor: Cell<Option<(i32, i32)>>,
    selected: Cell<usize>,
    pending_selected: Cell<Option<usize>>,
    session_selection: std::cell::RefCell<Option<(String, String)>>,
    started: Instant,
    content_since: Cell<Instant>,
}

impl<F: Fn() -> OverlayContent> WindowState<F> {
    fn fresh_content(&self) -> OverlayContent {
        let mut content = (self.content_source)();
        // A worker snapshot can predate a confirmed decision. Never reopen that request.
        for glyph in &mut content.glyphs {
            if glyph
                .permission
                .as_ref()
                .is_some_and(|r| Some(r.id) == self.resolved_permission.get())
            {
                glyph.permission = None;
            }
        }
        content
    }

    fn display_content(&self) -> OverlayContent {
        if collapse_for_inactivity(
            &self.current.lock().unwrap(),
            self.last_interaction.get().elapsed(),
        ) {
            return OverlayContent::default();
        }
        let t = if self.reduced_motion {
            1.0
        } else {
            tokens::ease_out_cubic(
                self.usage_start.get().elapsed().as_secs_f32()
                    / MotionToken::Standard.duration().as_secs_f32(),
            )
        };
        let mut content = interpolate_usage(
            &self.usage_from.lock().unwrap(),
            &self.current.lock().unwrap(),
            t,
        );
        if let Some((tool, id)) = self.session_selection.borrow().as_ref() {
            if let Some(g) = content
                .glyphs
                .iter_mut()
                .find(|g| &g.label == tool && g.permission.is_none())
            {
                g.selected_session = g.sessions.iter().position(|s| &s.id == id);
            }
        }
        content
    }
    /// Current eased morph progress, evaluated at "now" against the
    /// in-flight `anim_from -> anim_to` transition (design spec §16 /
    /// `VERGE_DESIGN_SYSTEM.md`'s `MotionToken::Standard`: a smooth morph,
    /// not the discrete jump this surface originally shipped with).
    fn progress(&self) -> f32 {
        if self.reduced_motion {
            return self.anim_to.get();
        }
        let duration = MotionToken::Standard.duration().as_secs_f32();
        if duration <= 0.0 {
            return self.anim_to.get();
        }
        let t = (self.anim_start.get().elapsed().as_secs_f32() / duration).min(1.0);
        let eased = tokens::ease_out_cubic(t);
        let from = self.anim_from.get();
        let to = self.anim_to.get();
        from + (to - from) * eased
    }

    fn animation_settled(&self) -> bool {
        self.reduced_motion
            || self.anim_from.get() == self.anim_to.get()
            || self.anim_start.get().elapsed() >= MotionToken::Standard.duration()
    }
}

pub struct WindowsOverlaySurface;

impl WindowsOverlaySurface {
    pub fn new() -> Self {
        WindowsOverlaySurface
    }
}

impl WindowsOverlaySurface {
    pub fn run_with_permissions(
        self,
        source: impl Fn() -> OverlayContent + Send + 'static,
        permissions: std::sync::Arc<dyn verge_core::ports::PermissionService>,
    ) -> std::io::Result<()> {
        unsafe { run_message_loop(source, Some(permissions)) }
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
        unsafe { run_message_loop(content_source, None) }
    }
}

unsafe fn run_message_loop(
    content_source: impl Fn() -> OverlayContent + Send + 'static,
    permissions: Option<std::sync::Arc<dyn verge_core::ports::PermissionService>>,
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
    // Polling may touch disk, processes, or a network. Never block the window's motion clock.
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || loop {
        if sender.send(content_source()).is_err() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(tokens::REFRESH_MS as u64));
    });
    let latest = std::cell::RefCell::new(OverlayContent::default());
    let boxed_source: Box<dyn Fn() -> OverlayContent> = Box::new(move || {
        if let Ok(fresh) = receiver.try_recv() {
            *latest.borrow_mut() = fresh;
        }
        latest.borrow().clone()
    });
    let state = std::sync::Arc::new(WindowState {
        content_source: boxed_source,
        permissions,
        painted_permission: Cell::new(None),
        resolved_permission: Cell::new(None),
        pressed_permission: Cell::new(None),
        keyboard_approve: Cell::new(false),
        keyboard_mode: Cell::new(false),
        permission_visible_since: Cell::new(Instant::now()),
        current: Mutex::new(OverlayContent::default()),
        usage_from: Mutex::new(OverlayContent::default()),
        usage_start: Cell::new(Instant::now()),
        painted_layout: std::cell::RefCell::new(None),
        hovered: Cell::new(false),
        open_requested: Cell::new(std::env::args().any(|arg| arg == "--open")),
        anim_from: Cell::new(0.0),
        anim_to: Cell::new(0.0),
        anim_start: Cell::new(Instant::now()),
        reduced_motion: reduced_motion_enabled(),
        hover_since: Cell::new(Instant::now()),
        last_interaction: Cell::new(Instant::now()),
        last_cursor: Cell::new(None),
        selected: Cell::new(0),
        pending_selected: Cell::new(None),
        session_selection: std::cell::RefCell::new(None),
        started: Instant::now(),
        content_since: Cell::new(Instant::now()),
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
        tokens::IDLE_WIDTH.round() as i32,
        tokens::IDLE_HEIGHT.round() as i32,
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

    // Begin transparent; the cursor poll enables interaction over the painted silhouette.
    let current_ex_style =
        GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE);
    SetWindowLongPtrW(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE,
        current_ex_style | (WS_EX_TRANSPARENT.0 as isize),
    );

    // Show the surface even when STARTUPINFO hides the process's console.
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
    );
    let _ = SetTimer(hwnd, tokens::TIMER_REFRESH, tokens::REFRESH_MS, None);
    let _ = SetTimer(hwnd, tokens::TIMER_HOVER, tokens::HOVER_POLL_MS, None);

    // Render once immediately rather than waiting for the first timer tick.
    let state_ref = &*(state_ptr);
    let fresh = state_ref.fresh_content();
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

/// Starts (or redirects) the compact/expanded morph toward `target`
/// (0.0 or 1.0), preserving whatever progress the previous transition had
/// already reached — so reversing direction mid-animation eases from where
/// the surface visually is, not from a snapped-back starting point.
unsafe fn start_transition<F: Fn() -> OverlayContent>(
    hwnd: HWND,
    state: &WindowState<F>,
    target: f32,
) {
    let current = state.progress();
    state.anim_from.set(current);
    state.anim_to.set(target);
    state.anim_start.set(Instant::now());
    if state.reduced_motion {
        present(hwnd, state);
    } else {
        let _ = SetTimer(hwnd, tokens::TIMER_ANIM, tokens::ANIM_TICK_MS, None);
        present(hwnd, state);
    }
}

unsafe fn permission_input(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    use verge_core::domain::PermissionDecision;
    let ptr = GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA);
    if ptr == 0 {
        return LRESULT(0);
    }
    let state = &*(ptr as *const WindowState<Box<dyn Fn() -> OverlayContent>>);
    state.last_interaction.set(Instant::now());
    let content = state.current.lock().unwrap().clone();
    let Some(l) = state.painted_layout.borrow().clone() else {
        return LRESULT(0);
    };
    let Some(request) = content
        .glyphs
        .get(l.selected)
        .and_then(|g| g.permission.as_ref())
    else {
        return LRESULT(0);
    };
    let Some(service) = &state.permissions else {
        return LRESULT(0);
    };
    let mut action = None;
    if msg == 0x0100 {
        match wparam.0 {
            0x1b => action = Some(2),
            0x09 => {
                state
                    .keyboard_approve
                    .set(state.keyboard_mode.get() && !state.keyboard_approve.get());
                state.keyboard_mode.set(true);
                present(hwnd, state);
            }
            0x0d | 0x20 => action = Some(if state.keyboard_approve.get() { 1 } else { 0 }),
            _ => {}
        }
    } else {
        let x = (lparam.0 as u16 as i16) as i32;
        let y = ((lparam.0 >> 16) as u16 as i16) as i32;
        let button = permission_buttons(&l)
            .iter()
            .position(|(bx, by, bw, bh)| {
                let radius = l.p(tokens::BUTTON_RADIUS) as f32;
                rounded_rect_coverage(
                    (x - bx) as f32 + 0.5,
                    (y - by) as f32 + 0.5,
                    *bw as f32,
                    *bh as f32,
                    radius,
                    radius,
                    radius,
                    radius,
                ) > 0.5
            })
            .map(|i| i as u8);
        let close_x =
            l.text_column_width - l.p(tokens::CONNECTOR_WIDTH + tokens::TEXT_COLUMN_PAD + 16.0);
        let close_y = card_bounds(&l).0 as i32 + l.p(tokens::TEXT_COLUMN_PAD);
        let button =
            if x >= close_x && x < close_x + l.p(24) && y >= close_y && y < close_y + l.p(24) {
                Some(2)
            } else {
                button
            };
        if msg == 0x0201 {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(hwnd);
            state.keyboard_mode.set(false);
            state
                .pressed_permission
                .set(button.and_then(|b| state.painted_permission.get().map(|id| (id, b))));
        } else if let Some((id, pressed)) = state.pressed_permission.take() {
            if id == request.id && Some(pressed) == button {
                action = Some(pressed);
            }
        }
    }
    if state.painted_permission.get() != Some(request.id)
        || !state.animation_settled()
        || state.progress() < 0.999
        || state.permission_visible_since.get().elapsed().as_millis() < tokens::PERMISSION_ARM_MS
    {
        return LRESULT(0);
    }
    if let Some(action) = action {
        let mut decision = match action {
            0 => PermissionDecision::Deny,
            1 => PermissionDecision::Approve,
            _ => PermissionDecision::Dismiss,
        };
        if action == 1 && permission_lines(request).1 {
            decision = if crate::permission_review::review(
                hwnd,
                &wide_permission_review(request),
                request.expires,
            ) {
                PermissionDecision::Approve
            } else {
                PermissionDecision::Deny
            };
        }
        if service.decide(request.id, &request.session_id, decision) {
            state.resolved_permission.set(Some(request.id));
            state.painted_permission.set(None);
            start_transition(hwnd, state, 0.0);
        }
        state.pressed_permission.set(None);
        // A failed/stale decision can never be retargeted. The worker supplies the next snapshot.
        *state.current.lock().unwrap() = state.fresh_content();
        present(hwnd, state);
    }
    LRESULT(0)
}
fn wide_permission_review(r: &verge_core::domain::PermissionRequest) -> Vec<u16> {
    to_wide(&format!("Approve this single Claude action?\n\nWorking directory: {}\n\n{}\n\nApprove once allows only this request. Deny or closing this review declines it.",safe_display(&r.cwd),safe_display(&r.detail)))
}

fn session_footer(l: &Layout) -> (i32, i32) {
    let (top, height) = card_bounds(l);
    (
        (top + height).round() as i32 - l.p(tokens::TEXT_COLUMN_PAD + 24.0),
        l.p(24.0),
    )
}

// Shared by painting and hit testing: Back | previous | counter | next.
fn session_navigation_bounds(width: i32) -> [i32; 5] {
    [0, width / 3, width / 2, width * 5 / 6, width]
}

fn session_footer_hit(l: &Layout, x: i32, y: i32, detail: bool) -> Option<i32> {
    let (top, height) = session_footer(l);
    let left = l.p(tokens::TEXT_COLUMN_PAD);
    let width = l.p(tokens::CONTENT_WIDTH);
    // Include the leading padding in Back's target, matching the full footer cell.
    if x < 0 || x >= left + width || y < top || y >= top + height {
        return None;
    }
    if !detail {
        return Some(2);
    }
    let bounds = session_navigation_bounds(width);
    let local = (x - left).max(0);
    match bounds
        .windows(2)
        .position(|b| local >= b[0] && local < b[1])?
    {
        0 => Some(0),
        1 => Some(1),
        3 => Some(2),
        _ => None, // The session count is a label, not a second navigation control.
    }
}

unsafe fn session_input(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> bool {
    if !matches!(msg, 0x0202 | 0x0100) {
        return false;
    }
    let ptr = GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA);
    if ptr == 0 {
        return false;
    }
    let state = &*(ptr as *const WindowState<Box<dyn Fn() -> OverlayContent>>);
    let Some(l) = state.painted_layout.borrow().clone() else {
        return false;
    };
    let content = state.display_content();
    let Some(g) = content.glyphs.get(l.selected) else {
        return false;
    };
    if g.permission.is_some()
        || g.sessions.is_empty()
        || !state.animation_settled()
        || l.progress < 0.999
    {
        return false;
    }
    let action = if msg == 0x0100 {
        match wparam.0 {
            0x1b => 0,
            0x25 => 1,
            0x27 | 0x0d | 0x20 => 2,
            _ => return false,
        }
    } else {
        let x = (lparam.0 as u16 as i16) as i32;
        let y = ((lparam.0 >> 16) as u16 as i16) as i32;
        let Some(action) = session_footer_hit(&l, x, y, g.selected_session.is_some()) else {
            return false;
        };
        action
    };
    state.last_interaction.set(Instant::now());
    if action == 0 {
        *state.session_selection.borrow_mut() = None;
    } else {
        let index = g.selected_session.map_or(0, |i| {
            if action == 1 {
                (i + g.sessions.len() - 1) % g.sessions.len()
            } else {
                (i + 1) % g.sessions.len()
            }
        });
        *state.session_selection.borrow_mut() =
            Some((g.label.clone(), g.sessions[index].id.clone()));
    }
    present(hwnd, state);
    true
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if matches!(msg, 0x0201 | 0x0202 | 0x0100) {
        if session_input(hwnd, msg, wparam, lparam) {
            return LRESULT(0);
        }
        return permission_input(hwnd, msg, wparam, lparam);
    }
    match msg {
        WM_TIMER => {
            let ptr =
                GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA);
            if ptr == 0 {
                return LRESULT(0);
            }
            let state = &*(ptr as *const WindowState<Box<dyn Fn() -> OverlayContent>>);

            if wparam.0 == tokens::TIMER_REFRESH {
                let fresh = state.fresh_content();
                let displayed = state.display_content();
                if interpolate_usage(&displayed, &fresh, 0.0) != fresh {
                    *state.usage_from.lock().unwrap() = displayed;
                    state.usage_start.set(Instant::now());
                }
                {
                    let mut current = state.current.lock().unwrap();
                    let selected = replacement_selection(&current, &fresh, state.selected.get());
                    if selected != state.selected.get() {
                        state.selected.set(selected);
                        state.pending_selected.set(None);
                    }
                    if current
                        .glyphs
                        .iter()
                        .map(|g| (g.state, g.permission.as_ref().map(|p| p.id)))
                        .collect::<Vec<_>>()
                        != fresh
                            .glyphs
                            .iter()
                            .map(|g| (g.state, g.permission.as_ref().map(|p| p.id)))
                            .collect::<Vec<_>>()
                    {
                        state.content_since.set(Instant::now());
                    }
                    *current = fresh;
                }
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
            } else if wparam.0 == tokens::TIMER_HOVER {
                let mut cursor = POINT::default();
                let _ = GetCursorPos(&mut cursor);
                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                let content = state.current.lock().unwrap();
                let Some(layout) = state.painted_layout.borrow().clone() else {
                    return LRESULT(0);
                };
                let x = (cursor.x - rect.left) as f32;
                let y = (cursor.y - rect.top) as f32;
                let inside = x >= 0.0
                    && x < layout.width as f32
                    && y >= 0.0
                    && y < layout.height as f32
                    && surface_coverage(x, y, &layout) > 0.5;
                let cursor_position = (cursor.x, cursor.y);
                if inside && state.last_cursor.get() != Some(cursor_position) {
                    state.last_interaction.set(Instant::now());
                }
                state.last_cursor.set(Some(cursor_position));
                // The visible material is interactive; all transparent space passes through.
                let style =
                    GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE);
                let next = if inside && !content.glyphs.is_empty() {
                    style & !(WS_EX_TRANSPARENT.0 as isize)
                } else {
                    style | WS_EX_TRANSPARENT.0 as isize
                };
                if style != next {
                    SetWindowLongPtrW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE,
                        next,
                    );
                }
                if inside && x >= layout.text_column_width as f32 {
                    let selected = layout.glyph_tops.iter().position(|top| {
                        y >= *top as f32 && y < (*top + layout.p(tokens::GLYPH_SLOT_HEIGHT)) as f32
                    });
                    if let Some(selected) = selected {
                        if selected != state.selected.get() {
                            state.pending_selected.set(Some(selected));
                            drop(content);
                            if state.anim_to.get() != 0.0 {
                                start_transition(hwnd, state, 0.0);
                            }
                        } else {
                            drop(content);
                        }
                    } else {
                        drop(content);
                    }
                } else {
                    drop(content);
                }
                if inside != state.hovered.get() {
                    if state.hovered.get() && !inside {
                        state.open_requested.set(false);
                    }
                    state.hovered.set(inside);
                    state.hover_since.set(Instant::now());
                }
                let delay = if inside {
                    tokens::HOVER_ENTER_MS
                } else {
                    tokens::HOVER_EXIT_MS
                };
                let reminder = state
                    .current
                    .lock()
                    .unwrap()
                    .glyphs
                    .iter()
                    .any(|g| g.reminder.is_some() || g.permission.is_some());
                if state.animation_settled() && state.progress() < 0.002 {
                    if let Some(selected) = state.pending_selected.take() {
                        state.selected.set(selected);
                    }
                }
                let target = if (inside || reminder || state.open_requested.get())
                    && state.pending_selected.get().is_none()
                {
                    1.0
                } else {
                    0.0
                };
                if state.hover_since.get().elapsed().as_millis() >= delay as u128
                    && state.anim_to.get() != target
                {
                    start_transition(hwnd, state, target);
                }
            } else if wparam.0 == tokens::TIMER_ANIM {
                present(hwnd, state);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // The message-loop owner releases the Arc exactly once after GetMessage ends.
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ---- Layout ----

#[derive(Clone)]
struct Layout {
    width: i32,
    height: i32,
    text_column_width: i32,
    progress: f32,
    scale: f32,
    selected: usize,
    glyph_tops: Vec<i32>,
    overflow_row_top: Option<i32>,
    card_height: f32,
    card_top: f32,
    rail_offset: i32,
    rail_height: i32,
}

impl Layout {
    fn p(&self, dip: impl Into<f64>) -> i32 {
        (dip.into() as f32 * self.scale).round() as i32
    }
}

fn compute_layout(content: &OverlayContent, progress: f32, scale: f32, selected: usize) -> Layout {
    let scale = scale * tokens::SURFACE_SCALE;
    let p = |v: f32| (v * scale).round() as i32;
    let n = content.glyphs.len();
    let selected = selected.min(n.saturating_sub(1));
    let progress = progress.clamp(0.0, 1.0);
    let text_column_width = if n == 0 {
        0
    } else {
        p(tokens::TEXT_COLUMN_MAX_WIDTH * progress)
    };
    let overflow = content.overflow_count.filter(|n| *n > 0);
    let rail_height = if n == 0 {
        p(tokens::IDLE_HEIGHT)
    } else {
        p(2.0 * tokens::EDGE_FLARE
            + tokens::CAPSULE_PAD_V
            + tokens::PAD_BOTTOM
            + n as f32 * tokens::GLYPH_SLOT_HEIGHT
            + (n - 1) as f32 * tokens::GLYPH_GAP_V
            + if overflow.is_some() {
                tokens::OVERFLOW_ROW_HEIGHT + tokens::GLYPH_GAP_V
            } else {
                0.0
            })
    };
    let usage_height = content.glyphs.get(selected).map_or(0.0, |g| {
        if usage_rows(g).is_empty() {
            tokens::DETAIL_HEIGHT
        } else {
            2.0 * tokens::TEXT_COLUMN_PAD
                + tokens::TITLE_LINE
                + tokens::USAGE_ROW * usage_rows(g).len() as f32
                + if g.reminder.is_some() {
                    tokens::BODY_LINE + 8.0
                } else {
                    0.0
                }
        }
    }) + if content
        .glyphs
        .get(selected)
        .is_some_and(|g| !g.sessions.is_empty())
    {
        36.0
    } else {
        0.0
    };
    // Transparent canvas grows independently of rail geometry, keeping the physical anchor fixed.
    let baseline_height = rail_height.max(p(usage_height + 8.0));
    let rail_offset = (baseline_height - rail_height) / 2;
    let center = rail_offset
        + p(tokens::EDGE_FLARE
            + tokens::CAPSULE_PAD_V
            + selected as f32 * (tokens::GLYPH_SLOT_HEIGHT + tokens::GLYPH_GAP_V)
            + tokens::RING_DIAMETER / 2.0);
    let card_top = (center as f32 - usage_height * scale / 2.0).clamp(
        0.0,
        (baseline_height as f32 - usage_height * scale).max(0.0),
    );
    let card_height = content
        .glyphs
        .get(selected)
        .and_then(|g| g.permission.as_ref())
        .map_or(
            if content
                .glyphs
                .get(selected)
                .is_some_and(|g| g.selected_session.is_some())
            {
                usage_height.max(244.0)
            } else {
                usage_height
            },
            |request| {
                tokens::PERMISSION_HEIGHT
                    - 3usize.saturating_sub(permission_lines(request).0.len()) as f32
                        * tokens::BODY_LINE
            },
        );
    let height = baseline_height.max(card_top.ceil() as i32 + p(card_height));
    Layout {
        width: if n == 0 {
            p(tokens::IDLE_WIDTH)
        } else {
            text_column_width + p(tokens::GLYPH_COLUMN_WIDTH)
        },
        height,
        rail_offset,
        rail_height,
        text_column_width,
        progress,
        scale,
        selected,
        card_height,
        card_top,
        glyph_tops: (0..n)
            .map(|i| {
                rail_offset
                    + p(tokens::EDGE_FLARE
                        + tokens::CAPSULE_PAD_V
                        + i as f32 * (tokens::GLYPH_SLOT_HEIGHT + tokens::GLYPH_GAP_V))
            })
            .collect(),
        overflow_row_top: overflow.map(|_| {
            rail_offset
                + p(tokens::EDGE_FLARE
                    + tokens::CAPSULE_PAD_V
                    + n as f32 * (tokens::GLYPH_SLOT_HEIGHT + tokens::GLYPH_GAP_V))
        }),
    }
}

fn card_bounds(l: &Layout) -> (f32, f32) {
    let height = (l.card_height * l.scale).min(l.height as f32);
    (l.card_top, height)
}

fn collapse_for_inactivity(content: &OverlayContent, inactive: std::time::Duration) -> bool {
    inactive.as_secs() >= tokens::INACTIVITY_SECONDS
        && !content
            .glyphs
            .iter()
            .any(|g| g.permission.is_some() || g.state == StateTint::Waiting)
}

fn replacement_selection(
    previous: &OverlayContent,
    next: &OverlayContent,
    selected: usize,
) -> usize {
    next.glyphs
        .iter()
        .position(|g| {
            g.permission.as_ref().is_some_and(|request| {
                !previous.glyphs.iter().any(|old| {
                    old.label == g.label
                        && old.permission.as_ref().is_some_and(|r| r.id == request.id)
                })
            })
        })
        .or_else(|| {
            previous
                .glyphs
                .get(selected)
                .and_then(|old| next.glyphs.iter().position(|g| g.label == old.label))
        })
        .unwrap_or(0)
}

fn fit_to_display(content: &OverlayContent, available_dips: f32) -> OverlayContent {
    let mut visible = content.clone();
    while visible.glyphs.len() > 1
        && compute_layout(&visible, 0.0, 1.0, 0).rail_height as f32 > available_dips
    {
        visible.glyphs.pop();
        visible.overflow_count = Some(visible.overflow_count.unwrap_or(0) + 1);
    }
    visible
}

/// Shared paint/hit geometry: an inverse fillet joins the body to the bezel.
fn surface_coverage(x: f32, y: f32, l: &Layout) -> f32 {
    if x < 0.0 || y < 0.0 || x >= l.width as f32 || y >= l.height as f32 {
        return 0.0;
    }
    if l.glyph_tops.is_empty() {
        return rounded_rect_coverage(
            x,
            y,
            l.width as f32,
            l.height as f32,
            2.0 * l.scale,
            0.0,
            0.0,
            2.0 * l.scale,
        );
    }
    let w = l.width as f32;
    let h = l.rail_height as f32;
    let rail_y = y - l.rail_offset as f32;
    let flare = tokens::EDGE_FLARE as f32 * l.scale;
    let rail_x = l.text_column_width as f32;
    let corner = tokens::CORNER_RADIUS * l.scale;
    let rail = if rail_y < 0.0 || rail_y > h {
        0.0
    } else if rail_y < flare || rail_y > h - flare {
        let cy = if rail_y < flare { 0.0 } else { h };
        if x < w - flare {
            0.0
        } else {
            ((((x - (w - flare)).powi(2) + (rail_y - cy).powi(2)).sqrt() - flare) + 0.5)
                .clamp(0.0, 1.0)
        }
    } else if x >= rail_x {
        rounded_rect_coverage(
            x - rail_x,
            rail_y - flare,
            w - rail_x,
            h - 2.0 * flare,
            corner,
            0.0,
            0.0,
            corner,
        )
    } else {
        0.0
    };
    // The flare/body join is internal material, not an antialiased exposed edge.
    let rail = if x >= w - flare && rail_y >= flare && rail_y <= h - flare {
        1.0
    } else {
        rail
    };
    if l.text_column_width == 0 {
        return rail;
    }
    let (top, height) = card_bounds(l);
    let lobe = if y >= top && y <= top + height {
        rounded_rect_coverage(
            x,
            y - top,
            (rail_x - l.p(tokens::CONNECTOR_WIDTH) as f32).max(0.0),
            height,
            tokens::CARD_CORNER * l.scale,
            tokens::CARD_CORNER * l.scale,
            tokens::CARD_CORNER * l.scale,
            tokens::CARD_CORNER * l.scale,
        )
    } else {
        0.0
    };
    // A soft neck grows from the selected identity. The panel keeps its own shoulders.
    let body_edge = rail_x - l.p(tokens::CONNECTOR_WIDTH) as f32;
    let neck_start = body_edge - l.p(tokens::CARD_CORNER) as f32;
    let neck_end = rail_x + l.p(4) as f32;
    let center = l.glyph_tops[l.selected] as f32 + l.p(tokens::RING_DIAMETER / 2.0) as f32;
    let join = if x >= neck_start && x <= neck_end {
        let t = ((x - body_edge) / (neck_end - body_edge)).clamp(0.0, 1.0);
        let half = l.scale
            * (tokens::CONNECTOR_HALF_HEIGHT
                + tokens::CARD_CORNER * 0.65 * (1.0 - (1.0 - (1.0 - t).powi(2)).sqrt()));
        (half - (y - center).abs() + 0.5).clamp(0.0, 1.0)
    } else {
        0.0
    };
    rail.max(lobe).max(join)
}

// ---- Presentation ----

fn present<F: Fn() -> OverlayContent>(hwnd: HWND, state: &WindowState<F>) {
    unsafe {
        let screen_dc = GetDC(None);
        let scale = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let content = fit_to_display(&state.display_content(), (screen_h - 16) as f32 / scale);
        let layout = compute_layout(&content, state.progress(), scale, state.selected.get());

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
            paint_idle(buf, &layout);
        } else {
            let phase = if state.reduced_motion {
                0.0
            } else {
                state.started.elapsed().as_secs_f32()
            };
            let attention = if state.reduced_motion {
                0.0
            } else {
                let t =
                    state.content_since.get().elapsed().as_secs_f32() / tokens::PERMISSION_SECONDS;
                if t < 1.0 {
                    (t * std::f32::consts::PI).sin().powi(2)
                } else {
                    0.0
                }
            };
            paint_capsule(buf, &layout, &content, mem_dc, phase, attention);
            if state.keyboard_mode.get()
                && content
                    .glyphs
                    .get(layout.selected)
                    .is_some_and(|g| g.permission.is_some())
            {
                let (bx, by, bw, bh) =
                    permission_buttons(&layout)[usize::from(state.keyboard_approve.get())];
                for yy in by - 2..by + bh + 2 {
                    for xx in bx - 2..bx + bw + 2 {
                        if xx >= 0
                            && yy >= 0
                            && xx < layout.width
                            && yy < layout.height
                            && (xx < bx || xx >= bx + bw || yy < by || yy >= by + bh)
                        {
                            buf[(yy * layout.width + xx) as usize] = composite(
                                buf[(yy * layout.width + xx) as usize],
                                tokens::TEXT_PRIMARY_RGB,
                                0.9,
                            );
                        }
                    }
                }
            }
        }

        let animate_state = content.glyphs.iter().any(|g| {
            g.state == StateTint::Working
                || (g.state == StateTint::Waiting
                    && state.content_since.get().elapsed().as_secs_f32()
                        < tokens::PERMISSION_SECONDS)
        });
        let animate_usage = state.usage_start.get().elapsed() < MotionToken::Standard.duration();
        if !state.reduced_motion && (!state.animation_settled() || animate_state || animate_usage) {
            let tick = if state.animation_settled() && !animate_usage {
                if content.glyphs.iter().any(|g| g.state == StateTint::Working) {
                    32
                } else {
                    tokens::STATE_TICK_MS
                }
            } else {
                tokens::ANIM_TICK_MS
            };
            let _ = SetTimer(hwnd, tokens::TIMER_ANIM, tick, None);
        } else {
            let _ = KillTimer(hwnd, tokens::TIMER_ANIM);
        }
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let x = screen_w - layout.width;
        // Vertically centered with a slight upward bias (design §3).
        let rail_top = ((screen_h - layout.rail_height) / 2 - screen_h / 12).max(8);
        let y = (rail_top - layout.rail_offset)
            .max(0)
            .min((screen_h - layout.height).max(0));

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

        let updated = UpdateLayeredWindow(
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

        if updated.is_ok() {
            *state.painted_layout.borrow_mut() = Some(layout.clone());
            let id = content
                .glyphs
                .get(layout.selected)
                .and_then(|g| g.permission.as_ref())
                .map(|r| r.id);
            if id != state.painted_permission.get() {
                state.permission_visible_since.set(Instant::now());
                state.keyboard_approve.set(false);
                state.keyboard_mode.set(false);
            }
            state.painted_permission.set(id);
        }
        let _ = DeleteDC(mem_dc);
        let _ = DeleteObject(bitmap);
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

fn paint_idle(buf: &mut [u32], layout: &Layout) {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let mut light = 1u32;
    let mut bytes = 4u32;
    unsafe {
        let _ = RegGetValueW(
            HKEY_CURRENT_USER,
            windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            windows::core::w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut light as *mut u32).cast()),
            Some(&mut bytes),
        );
    }
    let color = if light == 0 {
        (245, 245, 245)
    } else {
        (15, 15, 15)
    };
    for y in 0..layout.height {
        for x in 0..layout.width {
            let cov = surface_coverage(x as f32 + 0.5, y as f32 + 0.5, layout);
            buf[(y * layout.width + x) as usize] =
                composite(0, color, cov * tokens::IDLE_ALPHA as f32 / 255.0);
        }
    }
}

fn paint_capsule(
    buf: &mut [u32],
    l: &Layout,
    content: &OverlayContent,
    dc: HDC,
    phase: f32,
    attention: f32,
) {
    let (w, h) = (l.width, l.height);
    for y in 0..h {
        for x in 0..w {
            let cov = surface_coverage(x as f32 + 0.5, y as f32 + 0.5, l);
            let density = (tokens::MATERIAL_ALPHA_INNER
                + (tokens::MATERIAL_ALPHA_EDGE - tokens::MATERIAL_ALPHA_INNER) * x as f32
                    / w as f32)
                / 255.0;
            buf[(y * w + x) as usize] = composite(0, tokens::MATERIAL_RGB, cov * density);
        }
    }
    let cx = l.text_column_width + l.p(tokens::GLYPH_COLUMN_WIDTH / 2.0);
    for (i, g) in content.glyphs.iter().enumerate() {
        let cy = l.glyph_tops[i] + l.p(tokens::RING_DIAMETER / 2.0);
        let alpha = if g.dimmed {
            tokens::DIM_MULTIPLIER
        } else {
            1.0
        };
        if let Some(color) = tint_rgb(g.state) {
            // A broad falloff in the material, not a second outline competing with usage.
            let r = l.p(tokens::STATE_LIGHT_RADIUS);
            for y in (cy - r).max(0)..(cy + r).min(h) {
                for x in (cx - r).max(0)..(cx + r).min(w) {
                    let offset = if g.state == StateTint::Working {
                        (phase * TAU / tokens::WORKING_CYCLE_SECONDS).sin()
                            * tokens::WORKING_TRAVEL_DIP
                    } else if g.state == StateTint::Waiting {
                        -attention * tokens::PERMISSION_TRAVEL_DIP
                    } else {
                        0.0
                    };
                    let dx = (x - cx) as f32 / l.scale - offset;
                    let dy = (y - cy) as f32 / l.scale;

                    // Usage can be aged while activity is known right now.
                    let light = (-(dx * dx + (dy + 2.0).powi(2)) / tokens::STATE_LIGHT_SPREAD)
                        .exp()
                        * tokens::STATE_LIGHT_INTENSITY;
                    let k = (y * w + x) as usize;
                    buf[k] = composite(buf[k], color, light);
                }
            }
        }
        paint_ring(
            buf,
            w,
            h,
            cx,
            cy,
            l.p(tokens::RING_RADIUS),
            l.p(tokens::TRACK_THICKNESS),
            tokens::NEUTRAL_RING_RGB,
            tokens::NEUTRAL_RING_ALPHA,
        );
        if g.state == StateTint::Working {
            // A moving arc conveys work; its length never encodes quota.
            let angle = phase * TAU / 1.6;
            let radius = l.p(tokens::RING_RADIUS) as f32;
            let stroke = l.p(tokens::RING_THICKNESS) as f32;
            for y in (cy - radius as i32 - 6).max(0)..(cy + radius as i32 + 6).min(h) {
                for x in (cx - radius as i32 - 6).max(0)..(cx + radius as i32 + 6).min(w) {
                    let dx = x as f32 + 0.5 - cx as f32;
                    let dy = y as f32 + 0.5 - cy as f32;
                    let (sin, cos) = angle.sin_cos();
                    let cov = ring_arc_coverage(
                        cx as f32 + dx * cos + dy * sin,
                        cy as f32 - dx * sin + dy * cos,
                        cx as f32,
                        cy as f32,
                        radius,
                        stroke,
                        0.24,
                    );
                    let k = (y * w + x) as usize;
                    buf[k] = composite(buf[k], tokens::STATE_WORKING, cov);
                }
            }
        } else if let Some(color) = tint_rgb(g.state) {
            paint_ring(
                buf,
                w,
                h,
                cx,
                cy,
                l.p(tokens::RING_RADIUS),
                l.p(tokens::RING_THICKNESS),
                color,
                1.0,
            );
        }
        if matches!(g.label.as_str(), "Claude" | "ChatGPT" | "OpenCode") {
            paint_brand_mark(
                &g.label,
                buf,
                w,
                h,
                cx,
                cy,
                l.p(tokens::GLYPH_DIAMETER),
                g.brand_color,
                1.0,
            );
        } else {
            let mark = g.mark.to_string();
            let size = l.p(tokens::GLYPH_DIAMETER);
            let width = text_width(dc, &mark, size, false);
            draw_text_alpha(
                buf,
                w,
                h,
                dc,
                &mark,
                cx - width / 2,
                cy - size / 2,
                width + l.p(2),
                size + l.p(4),
                size,
                false,
                g.brand_color,
                alpha,
            );
        }
        let caption = match g.metric {
            Metric::Fraction(f) => format!("{:.0}%", f.clamp(0.0, 1.0) * 100.0),
            _ => match g.state {
                StateTint::Working => "Working",
                StateTint::Waiting => "Needs you",
                StateTint::Completed => "Completed",
                StateTint::Stopped => "Stopped",
                StateTint::Neutral => g.label.as_str(),
            }
            .to_string(),
        };
        let caption_size = l.p(if matches!(g.metric, Metric::Fraction(_)) {
            tokens::TYPE_NUMERAL_PX
        } else {
            tokens::TYPE_MICRO_PX
        });
        let text_w = text_width(dc, &caption, caption_size, false);
        draw_text_alpha(
            buf,
            w,
            h,
            dc,
            &caption,
            cx - text_w / 2,
            cy + l.p(tokens::RING_DIAMETER / 2.0 + tokens::RING_LABEL_GAP),
            text_w + l.p(2),
            l.p(tokens::NUMERAL_LINE_HEIGHT),
            caption_size,
            false,
            tokens::TEXT_PRIMARY_RGB,
            if matches!(g.metric, Metric::Fraction(_))
                || (g.activity_label.is_none() && g.state == StateTint::Neutral)
            {
                alpha
            } else {
                1.0
            },
        );
        if let Some(count) = g.session_count.filter(|n| *n > 0) {
            let count = if count > 99 {
                "99+".into()
            } else {
                count.to_string()
            };
            let font = l.p(tokens::TYPE_MICRO_PX);
            let text_w = text_width(dc, &count, font, true);
            let badge_w = (text_w + l.p(12)).max(l.p(24));
            let badge_h = l.p(24);
            let badge_x =
                (cx + l.p(tokens::RING_DIAMETER / 2.0) - badge_w / 2).min(w - badge_w - l.p(4));
            let badge_y = cy - l.p(tokens::RING_DIAMETER / 2.0 + 22.0);
            for by in 0..badge_h {
                for bx in 0..badge_w {
                    let (px, py) = (badge_x + bx, badge_y + by);
                    if px < 0 || px >= w || py < 0 || py >= h {
                        continue;
                    }
                    let radius = badge_h as f32 / 2.0;
                    let outer = rounded_rect_coverage(
                        bx as f32 + 0.5,
                        by as f32 + 0.5,
                        badge_w as f32,
                        badge_h as f32,
                        radius,
                        radius,
                        radius,
                        radius,
                    );
                    let inner = rounded_rect_coverage(
                        bx as f32 - 0.5,
                        by as f32 - 0.5,
                        (badge_w - 2) as f32,
                        (badge_h - 2) as f32,
                        radius - 1.0,
                        radius - 1.0,
                        radius - 1.0,
                        radius - 1.0,
                    );
                    let k = (py * w + px) as usize;
                    buf[k] = composite(buf[k], (94, 94, 98), outer);
                    buf[k] = composite(buf[k], (32, 32, 35), inner);
                }
            }
            draw_text_alpha(
                buf,
                w,
                h,
                dc,
                &count,
                badge_x + (badge_w - text_w) / 2,
                badge_y + (badge_h - font) / 2 - l.p(1),
                text_w + l.p(2),
                badge_h,
                font,
                true,
                tokens::TEXT_PRIMARY_RGB,
                1.0,
            );
        }
        if i == l.selected && l.progress > 0.0 {
            let x = l.p(tokens::TEXT_COLUMN_PAD);
            let top = card_bounds(l).0.round() as i32 + l.p(tokens::TEXT_COLUMN_PAD);
            // Text is stationary relative to the identity, revealed by the opening material.
            let x = x + l.text_column_width - l.p(tokens::TEXT_COLUMN_MAX_WIDTH);
            let reveal = ((l.progress - 0.2) / 0.8).clamp(0.0, 1.0);
            let heading = g.label.clone();
            let heading_x = if matches!(g.label.as_str(), "Claude" | "ChatGPT" | "OpenCode") {
                paint_brand_mark(
                    &g.label,
                    buf,
                    w,
                    h,
                    x + l.p(tokens::HEADER_MARK / 2.0),
                    top + l.p(18),
                    l.p(tokens::HEADER_MARK),
                    g.brand_color,
                    reveal,
                );
                x + l.p(tokens::HEADER_MARK + tokens::HEADER_IDENTITY_GAP)
            } else {
                x
            };
            draw_text_alpha(
                buf,
                w,
                h,
                dc,
                &heading,
                heading_x,
                top + l.p(2),
                l.p(tokens::CONTENT_WIDTH) - (heading_x - x),
                l.p(tokens::TITLE_LINE),
                l.p(tokens::TYPE_TITLE_PX),
                false,
                tokens::TEXT_PRIMARY_RGB,
                reveal,
            );
            if let Some(n) = g.session_count.filter(|n| *n > 0) {
                let count = format!("{n} {}", if n == 1 { "session" } else { "sessions" });
                let count_x = heading_x
                    + text_width(dc, &heading, l.p(tokens::TYPE_TITLE_PX), false)
                    + l.p(12);
                draw_text_alpha(
                    buf,
                    w,
                    h,
                    dc,
                    &count,
                    count_x,
                    top + l.p(10),
                    x + l
                        .p(tokens::CONTENT_WIDTH - if g.permission.is_some() { 26.0 } else { 0.0 })
                        - count_x,
                    l.p(tokens::BODY_LINE),
                    l.p(tokens::TYPE_SESSION_PX),
                    false,
                    tokens::TEXT_SECONDARY_RGB,
                    reveal,
                );
            }
            if let Some(request) = &g.permission {
                paint_permission(buf, l, request, dc, x, top, reveal);
                continue;
            }
            if !g.sessions.is_empty() {
                let (fy, fh) = session_footer(l);
                let labels = if let Some(index) = g.selected_session {
                    vec![
                        "\u{2039} Back".into(),
                        "\u{2039}".into(),
                        format!("{} / {}", index + 1, g.sessions.len()),
                        "\u{203a}".into(),
                    ]
                } else {
                    vec![format!(
                        "{}   ›",
                        g.session_summary.as_deref().unwrap_or("Sessions")
                    )]
                };
                let width = l.p(tokens::CONTENT_WIDTH);
                let bounds = if g.selected_session.is_some() {
                    session_navigation_bounds(width).to_vec()
                } else {
                    vec![0, width]
                };
                for (index, label) in labels.iter().enumerate() {
                    draw_text_alpha(
                        buf,
                        w,
                        h,
                        dc,
                        label,
                        x + bounds[index],
                        fy,
                        bounds[index + 1] - bounds[index],
                        fh,
                        l.p(tokens::TYPE_MICRO_PX),
                        false,
                        tokens::TEXT_PRIMARY_RGB,
                        reveal,
                    );
                }
            }

            if let Some(session) = g.selected_session.and_then(|index| g.sessions.get(index)) {
                for (row, line) in std::iter::once(&session.title)
                    .chain(session.lines.iter())
                    .enumerate()
                {
                    draw_text_alpha(
                        buf,
                        w,
                        h,
                        dc,
                        &safe_display(line),
                        x,
                        top + l.p(tokens::TITLE_LINE + 6.0 + row as f32 * tokens::BODY_LINE),
                        l.p(tokens::CONTENT_WIDTH),
                        l.p(tokens::BODY_LINE),
                        l.p(if row == 0 {
                            tokens::TYPE_BODY_PX
                        } else {
                            tokens::TYPE_MICRO_PX
                        }),
                        false,
                        if row == 0 || row == 2 {
                            tokens::TEXT_PRIMARY_RGB
                        } else {
                            tokens::TEXT_SECONDARY_RGB
                        },
                        reveal,
                    );
                }
                continue;
            }
            if !usage_rows(g).is_empty() {
                paint_usage_details(buf, l, g, dc, x, top, reveal);
                continue;
            }
            let status = g.activity_label.as_deref().or(match g.state {
                StateTint::Working => Some("Working"),
                StateTint::Waiting => Some("Needs you"),
                StateTint::Completed => Some("Completed"),
                StateTint::Stopped => Some("Stopped / error"),
                StateTint::Neutral => None,
            });
            let lines: Vec<&str> = status
                .into_iter()
                .chain(g.detail_lines.iter().map(String::as_str))
                .filter(|line| !line.ends_with("% used"))
                .take(3)
                .collect();
            for (j, line) in lines.iter().enumerate() {
                draw_text_alpha(
                    buf,
                    w,
                    h,
                    dc,
                    line,
                    x,
                    top + l.p(tokens::TITLE_LINE + 10.0 + j as f32 * tokens::BODY_LINE),
                    l.p(tokens::CONTENT_WIDTH),
                    l.p(tokens::BODY_LINE),
                    l.p(tokens::TYPE_MICRO_PX),
                    false,
                    tokens::TEXT_SECONDARY_RGB,
                    if status == Some(*line) {
                        reveal
                    } else {
                        reveal * alpha
                    },
                );
            }
        }
    }
    if let (Some(count), Some(top)) = (content.overflow_count, l.overflow_row_top) {
        let label = format!("+{count}");
        let width = text_width(dc, &label, l.p(tokens::TYPE_OVERFLOW_PX), false);
        draw_text_alpha(
            buf,
            w,
            h,
            dc,
            &label,
            cx - width / 2,
            top,
            width + l.p(2),
            l.p(20),
            l.p(tokens::TYPE_OVERFLOW_PX),
            false,
            tokens::TEXT_SECONDARY_RGB,
            1.0,
        );
    }
    // Clip all layers to the physical object, including during partial reveals.
    for y in 0..h {
        for x in 0..w {
            if surface_coverage(x as f32 + 0.5, y as f32 + 0.5, l) == 0.0 {
                buf[(y * w + x) as usize] = 0;
            }
        }
    }
}

// Brand masks share a geometric center and preserve their original SVG viewBox.
fn paint_brand_mark(
    label: &str,
    buf: &mut [u32],
    w: i32,
    h: i32,
    cx: i32,
    cy: i32,
    size: i32,
    color: (u8, u8, u8),
    alpha: f32,
) {
    let mask: &[u8; 16384] = if label == "OpenCode" {
        include_bytes!("opencode-mark.alpha")
    } else if label == "ChatGPT" {
        include_bytes!("openai-mark.alpha")
    } else {
        include_bytes!("claude-mark.alpha")
    };
    let left = cx as f32 - size as f32 / 2.0;
    let top = cy as f32 - size as f32 / 2.0;
    for py in (top.floor() as i32).max(0)..((top + size as f32).ceil() as i32).min(h) {
        for px in (left.floor() as i32).max(0)..((left + size as f32).ceil() as i32).min(w) {
            let sx = ((px as f32 + 0.5 - left) * 128.0 / size as f32 - 0.5).clamp(0.0, 127.0);
            let sy = ((py as f32 + 0.5 - top) * 128.0 / size as f32 - 0.5).clamp(0.0, 127.0);
            let x = sx.floor() as usize;
            let y = sy.floor() as usize;
            let fx = sx - x as f32;
            let fy = sy - y as f32;
            let sample = |x: usize, y: usize| mask[y.min(127) * 128 + x.min(127)] as f32;
            let coverage = (sample(x, y) * (1.0 - fx) * (1.0 - fy)
                + sample(x + 1, y) * fx * (1.0 - fy)
                + sample(x, y + 1) * (1.0 - fx) * fy
                + sample(x + 1, y + 1) * fx * fy)
                / 255.0;
            let k = (py * w + px) as usize;
            buf[k] = composite(buf[k], color, coverage * alpha);
        }
    }
}

// ---- Rasterization primitives ----

fn paint_usage_details(
    buf: &mut [u32],
    l: &Layout,
    g: &verge_core::ports::ToolGlyph,
    dc: HDC,
    x: i32,
    top: i32,
    reveal: f32,
) {
    let (w, h) = (l.width, l.height);
    let width = l.p(tokens::CONTENT_WIDTH);
    for (row, (label, window)) in usage_rows(g).into_iter().enumerate() {
        let weekly = label.to_ascii_lowercase().contains("week") || label == "All models";
        let font = l.p(if weekly {
            tokens::TYPE_MICRO_PX
        } else {
            tokens::TYPE_BODY_PX
        });
        let y = top + l.p(tokens::TITLE_LINE + 6.0 + row as f32 * tokens::USAGE_ROW);
        let Some(window) = window else {
            draw_text_alpha(
                buf,
                w,
                h,
                dc,
                label,
                x,
                y,
                width,
                l.p(tokens::BODY_LINE),
                font,
                false,
                tokens::TEXT_PRIMARY_RGB,
                reveal,
            );
            draw_text_alpha(
                buf,
                w,
                h,
                dc,
                "Not reported",
                x,
                y + l.p(29),
                width,
                l.p(tokens::BODY_LINE),
                l.p(tokens::TYPE_MICRO_PX),
                false,
                tokens::TEXT_SECONDARY_RGB,
                reveal,
            );
            continue;
        };
        let alpha = reveal * if window.dimmed { 0.75 } else { 1.0 };
        let reset_width = text_width(dc, &window.reset, l.p(tokens::TYPE_MICRO_PX), false);
        draw_text_alpha(
            buf,
            w,
            h,
            dc,
            label,
            x,
            y,
            width - reset_width - l.p(8),
            l.p(tokens::BODY_LINE),
            font,
            false,
            tokens::TEXT_PRIMARY_RGB,
            alpha,
        );
        draw_text_alpha(
            buf,
            w,
            h,
            dc,
            &window.reset,
            x + width - reset_width,
            y + l.p(1),
            reset_width,
            l.p(tokens::BODY_LINE),
            l.p(tokens::TYPE_MICRO_PX),
            false,
            tokens::TEXT_SECONDARY_RGB,
            alpha,
        );
        let bar_y = y + l.p(29);
        let bar_h = l.p(tokens::USAGE_BAR_HEIGHT).max(2);
        let filled = (width as f32 * window.fraction).round() as i32;
        for by in 0..bar_h {
            for bx in 0..width {
                let px = x + bx;
                let py = bar_y + by;
                if px < 0 || px >= w || py < 0 || py >= h {
                    continue;
                }
                let coverage = rounded_rect_coverage(
                    bx as f32 + 0.5,
                    by as f32 + 0.5,
                    width as f32,
                    bar_h as f32,
                    bar_h as f32 / 2.0,
                    bar_h as f32 / 2.0,
                    bar_h as f32 / 2.0,
                    bar_h as f32 / 2.0,
                );
                let k = (py * w + px) as usize;
                buf[k] = composite(buf[k], (43, 43, 45), coverage * alpha);
                if filled > 0 {
                    let radius = (bar_h as f32 / 2.0).min(filled as f32 / 2.0);
                    let fill = rounded_rect_coverage(
                        bx as f32 + 0.5,
                        by as f32 + 0.5,
                        filled as f32,
                        bar_h as f32,
                        radius,
                        radius,
                        radius,
                        radius,
                    );
                    buf[k] = composite(buf[k], tokens::usage_color(window.fraction), fill * alpha);
                }
            }
        }
        let used = format!("{:.0}% used", window.fraction * 100.0);
        draw_text_alpha(
            buf,
            w,
            h,
            dc,
            &used,
            x,
            y + l.p(43),
            width,
            l.p(tokens::BODY_LINE),
            font,
            false,
            tokens::TEXT_PRIMARY_RGB,
            alpha,
        );
    }
    if let Some(status) = &g.activity_label {
        let size = l.p(tokens::TYPE_SESSION_PX);
        let status_width = text_width(dc, status, size, false).min(width / 2);
        draw_text_alpha(
            buf,
            w,
            h,
            dc,
            status,
            x + width - status_width,
            top + l.p(tokens::TITLE_LINE
                + 49.0
                + (usage_rows(g).len().saturating_sub(1)) as f32 * tokens::USAGE_ROW),
            status_width,
            l.p(tokens::BODY_LINE),
            size,
            false,
            tint_rgb(g.state).unwrap_or(tokens::TEXT_SECONDARY_RGB),
            reveal,
        );
    }
    if let Some(message) = &g.reminder {
        draw_text_alpha(
            buf,
            w,
            h,
            dc,
            message,
            x,
            top + l.p(tokens::TITLE_LINE + tokens::USAGE_ROW * usage_rows(g).len() as f32),
            width,
            l.p(tokens::BODY_LINE),
            l.p(tokens::TYPE_MICRO_PX),
            false,
            tokens::TEXT_SECONDARY_RGB,
            reveal,
        );
    }
}

fn safe_display(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() && c != '\n'
                || matches!(c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
            {
                format!("\\u{{{:x}}}", c as u32).chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn permission_lines(request: &verge_core::domain::PermissionRequest) -> (Vec<String>, bool) {
    // Show the action, not its JSON envelope. The full original stays in the native review.
    let parsed = serde_json::from_str::<serde_json::Value>(&request.detail).ok();
    let summary = parsed.as_ref().and_then(|value| {
        ["command", "file_path", "url", "pattern"]
            .iter()
            .find_map(|key| value.get(key).and_then(|v| v.as_str()))
    });
    let text = safe_display(summary.unwrap_or(&request.detail));
    let mut lines = Vec::new();
    for line in text.lines() {
        let chars = line.chars().collect::<Vec<_>>();
        if chars.is_empty() {
            lines.push(String::new());
        } else {
            for chunk in chars.chunks(36) {
                lines.push(chunk.iter().collect());
            }
        }
    }
    let clipped = summary.is_some()
        || lines.len() > 3
        || !text.is_ascii()
        || text.lines().any(|line| line.len() > 22)
        || !request.cwd.is_ascii()
        || request.cwd.len() > 22;
    lines.truncate(3);
    if clipped && lines.len() == 3 {
        lines[2] = "Review full action before approving…".into();
    }
    (lines, clipped)
}
fn permission_buttons(l: &Layout) -> [(i32, i32, i32, i32); 2] {
    let width = l.p((tokens::CONTENT_WIDTH - tokens::BUTTON_GAP) / 2.0);
    let x = l.text_column_width - l.p(tokens::TEXT_COLUMN_MAX_WIDTH) + l.p(tokens::TEXT_COLUMN_PAD);
    let (top, height) = card_bounds(l);
    let y = (top + height).round() as i32 - l.p(tokens::TEXT_COLUMN_PAD + tokens::BUTTON_HEIGHT);
    [
        (x, y, width, l.p(tokens::BUTTON_HEIGHT)),
        (
            x + width + l.p(tokens::BUTTON_GAP),
            y,
            width,
            l.p(tokens::BUTTON_HEIGHT),
        ),
    ]
}

fn paint_permission(
    buf: &mut [u32],
    l: &Layout,
    r: &verge_core::domain::PermissionRequest,
    dc: HDC,
    x: i32,
    top: i32,
    alpha: f32,
) {
    let width = l.p(tokens::CONTENT_WIDTH);
    let mut text = |label: &str, y: f32, size: f32, bold: bool, color| {
        draw_text_alpha(
            buf,
            l.width,
            l.height,
            dc,
            label,
            x,
            top + l.p(y),
            width,
            l.p(tokens::BODY_LINE + 4.0),
            l.p(size),
            bold,
            color,
            alpha,
        )
    };
    text(
        "Needs your approval",
        tokens::TITLE_LINE + 4.0,
        tokens::TYPE_MICRO_PX,
        false,
        tokens::TEXT_PRIMARY_RGB,
    );
    let waiting = format!(
        "Waiting {}s · this action only",
        r.created.elapsed().as_secs()
    );
    text(
        &waiting,
        76.0,
        tokens::TYPE_MICRO_PX,
        false,
        tokens::TEXT_SECONDARY_RGB,
    );
    let action = match r.tool.as_str() {
        "Bash" => "Run a command".into(),
        "Edit" | "Write" | "MultiEdit" => "Change a file".into(),
        "Read" => "Read a file".into(),
        _ => format!("Use {}", safe_display(&r.tool)),
    };
    text(
        &action,
        114.0,
        tokens::TYPE_BODY_PX,
        false,
        tokens::TEXT_PRIMARY_RGB,
    );
    text(
        &safe_display(&r.cwd),
        142.0,
        tokens::TYPE_MICRO_PX,
        false,
        tokens::TEXT_SECONDARY_RGB,
    );
    let (lines, clipped) = permission_lines(r);
    for (i, line) in lines.iter().enumerate() {
        text(
            line,
            176.0 + i as f32 * tokens::BODY_LINE,
            tokens::TYPE_MICRO_PX,
            false,
            tokens::TEXT_PRIMARY_RGB,
        );
    }
    draw_text_alpha(
        buf,
        l.width,
        l.height,
        dc,
        "×",
        x + width - l.p(16),
        top,
        l.p(24),
        l.p(24),
        l.p(20),
        false,
        tokens::TEXT_SECONDARY_RGB,
        alpha,
    );
    for (i, (bx, by, bw, bh)) in permission_buttons(l).into_iter().enumerate() {
        for yy in 0..bh {
            for xx in 0..bw {
                let px = bx + xx;
                let py = by + yy;
                if px < 0 || px >= l.width || py < 0 || py >= l.height {
                    continue;
                }
                let cov = rounded_rect_coverage(
                    xx as f32 + 0.5,
                    yy as f32 + 0.5,
                    bw as f32,
                    bh as f32,
                    l.p(tokens::BUTTON_RADIUS) as f32,
                    l.p(tokens::BUTTON_RADIUS) as f32,
                    l.p(tokens::BUTTON_RADIUS) as f32,
                    l.p(tokens::BUTTON_RADIUS) as f32,
                );
                let k = (py * l.width + px) as usize;
                buf[k] = composite(
                    buf[k],
                    if i == 0 { (45, 25, 26) } else { (23, 42, 30) },
                    cov * alpha,
                );
            }
        }
        let label = if i == 0 {
            "Deny"
        } else if clipped {
            "Approve…"
        } else {
            "Approve"
        };
        let font = l.p(tokens::TYPE_MICRO_PX);
        let tw = text_width(dc, label, font, false);
        draw_text_alpha(
            buf,
            l.width,
            l.height,
            dc,
            label,
            bx + (bw - tw) / 2,
            by + (bh - font) / 2 - l.p(1),
            tw + l.p(2),
            l.p(20),
            font,
            false,
            if i == 0 {
                tokens::STATE_STOPPED
            } else {
                tokens::STATE_COMPLETED
            },
            alpha,
        );
    }
}

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
    if w <= 0.0 || h <= 0.0 {
        return 0.0;
    }
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
    let r = r.min(cx).min(cy);
    let qx = px.abs() - (cx - r);
    let qy = py.abs() - (cy - r);
    let dist = qx.max(qy).min(0.0) + (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() - r;
    (0.5 - dist).clamp(0.0, 1.0)
}

fn ring_coverage(x: f32, y: f32, cx: f32, cy: f32, r: f32, thickness: f32) -> f32 {
    let dist = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    let d = (dist - r).abs();
    (thickness / 2.0 - d + 0.5).clamp(0.0, 1.0)
}

/// Same ring test as `ring_coverage`, but only within `fraction` of the
/// circle's circumference, measured clockwise from twelve o'clock — the
/// usage-ring fill arc (design spec §9's "the ring fills proportionally").
/// Rounded, antialiased caps preserve the reference's instrument-like stroke.
fn ring_arc_coverage(
    x: f32,
    y: f32,
    cx: f32,
    cy: f32,
    r: f32,
    thickness: f32,
    fraction: f32,
) -> f32 {
    if fraction <= 0.0 {
        return 0.0;
    }
    let base = ring_coverage(x, y, cx, cy, r, thickness);
    if fraction >= 1.0 {
        return base;
    }
    // atan2 gives an angle from +X axis, counter-clockwise; rotate so 0 is
    // at twelve o'clock and increases clockwise (screen Y grows downward,
    // which already flips the usual counter-clockwise sense to clockwise
    // once offset from the top).
    let angle = (x - cx).atan2(-(y - cy));
    let normalized = if angle < 0.0 { angle + TAU } else { angle } / TAU;
    let sweep = if normalized <= fraction { base } else { 0.0 };
    let cap = |angle: f32| {
        let dx = x - (cx + r * angle.sin());
        let dy = y - (cy - r * angle.cos());
        (thickness / 2.0 + 0.5 - dx.hypot(dy)).clamp(0.0, 1.0)
    };
    sweep.max(cap(0.0)).max(cap(fraction * TAU))
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
    static LOADED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let loaded = *LOADED.get_or_init(|| {
        [
            include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice(),
            include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice(),
        ]
        .into_iter()
        .all(|bytes| unsafe {
            let mut count = 0u32;
            let handle = windows::Win32::Graphics::Gdi::AddFontMemResourceEx(
                bytes.as_ptr().cast(),
                bytes.len() as u32,
                None,
                &mut count,
            );
            !handle.is_invalid() && count > 0
        })
    });
    unsafe {
        // Private, process-lifetime resources: no system font installation is needed.
        let name = to_wide(if loaded {
            tokens::FONT_FAMILY
        } else {
            "Segoe UI"
        });
        CreateFontW(
            -size_px,
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
            ANTIALIASED_QUALITY.0 as u32,
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
    if text.is_empty() || box_w <= 0 || box_h <= 0 || alpha_mult <= 0.0 {
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
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: box_w,
            bottom: box_h,
        };
        DrawTextW(
            scratch_dc,
            &mut wide,
            &mut rect,
            DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        let _ = GdiFlush();
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

        let _ = DeleteDC(scratch_dc);
        let _ = DeleteObject(scratch_bitmap);
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod visual_tests {
    use super::*;
    use verge_core::ports::ToolGlyph;

    #[test]
    fn mouse_back_includes_leading_padding_at_every_scale() {
        let content = OverlayContent {
            glyphs: vec![fixture(StateTint::Working, Metric::Fraction(0.2))],
            overflow_count: None,
        };
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let l = compute_layout(&content, 1.0, scale, 0);
            let y = session_footer(&l).0 + l.p(8);
            let left = l.p(tokens::TEXT_COLUMN_PAD);
            let width = l.p(tokens::CONTENT_WIDTH);
            for x in [1, left - 1, left, left + width / 6] {
                assert_eq!(session_footer_hit(&l, x, y, true), Some(0));
            }
            assert_eq!(
                session_footer_hit(&l, left + width * 5 / 12, y, true),
                Some(1)
            );
            assert_eq!(session_footer_hit(&l, left + width - 1, y, true), Some(2));
            assert_eq!(
                session_footer_hit(&l, left + width * 2 / 3, y, true),
                None,
                "the counter is not a navigation button"
            );
            assert_eq!(session_footer_hit(&l, 1, y, false), Some(2));
            assert_eq!(session_footer_hit(&l, -1, y, true), None);
            assert_eq!(
                session_footer_hit(&l, 1, session_footer(&l).0 - 1, true),
                None
            );
        }
    }

    #[test]
    fn usage_motion_preserves_identity_and_claude_limit_rows() {
        let from = OverlayContent {
            glyphs: vec![fixture(StateTint::Working, Metric::Fraction(0.2))],
            overflow_count: None,
        };
        let to = OverlayContent {
            glyphs: vec![fixture(StateTint::Working, Metric::Fraction(0.8))],
            overflow_count: None,
        };
        let mid = interpolate_usage(&from, &to, 0.5);
        assert_eq!(mid.glyphs[0].metric, Metric::Fraction(0.5));
        assert_eq!(interpolate_usage(&from, &to, 1.0), to);
        let mut unknown = from.clone();
        unknown.glyphs[0].metric = Metric::None;
        assert_eq!(
            interpolate_usage(&unknown, &to, 0.0).glyphs[0].metric,
            to.glyphs[0].metric
        );
        let mut other = from.clone();
        other.glyphs[0].label = "Other".into();
        assert_eq!(interpolate_usage(&other, &to, 0.0), to);
        let mut single = to.clone();
        single.glyphs[0].usage_windows = vec![relevant_usage(&to.glyphs[0]).unwrap().clone()];
        assert_eq!(relevant_usage(&to.glyphs[0]).unwrap().fraction, 0.8);
        assert_eq!(
            compute_layout(&single, 1.0, 1.25, 0).card_height,
            compute_layout(&to, 1.0, 1.25, 0).card_height
        );
        assert_eq!(usage_rows(&to.glyphs[0]).len(), 2);
        assert!(usage_rows(&single.glyphs[0])[1].1.is_none());
        let l = compute_layout(&single, 1.0, 1.25, 0);
        let neck_x = (l.text_column_width - l.p(tokens::CONNECTOR_WIDTH / 2.0)) as f32;
        let center_y = (l.glyph_tops[0] + l.p(tokens::RING_DIAMETER / 2.0)) as f32;
        assert!(surface_coverage(neck_x, center_y, &l) > 0.99);
        assert_eq!(surface_coverage(neck_x, center_y + l.p(12) as f32, &l), 0.0);
    }

    #[test]
    fn bundled_inter_is_selected_by_windows() {
        unsafe {
            let dc = CreateCompatibleDC(None);
            for bold in [false, true] {
                let font = make_font(17, bold);
                let previous = SelectObject(dc, font);
                let mut face = [0u16; 64];
                let count = windows::Win32::Graphics::Gdi::GetTextFaceW(dc, Some(&mut face));
                assert!(count > 0);
                assert_eq!(
                    String::from_utf16_lossy(&face[..count as usize - 1]),
                    "Inter"
                );
                SelectObject(dc, previous);
                let _ = DeleteObject(font);
            }
            let _ = DeleteDC(dc);
        }
    }

    #[test]
    fn inactivity_collapses_working_but_permissions_wake_and_hold() {
        use std::time::Duration;
        let mut content = OverlayContent {
            glyphs: vec![fixture(StateTint::Working, Metric::Fraction(0.35))],
            overflow_count: None,
        };
        assert!(!collapse_for_inactivity(&content, Duration::from_secs(29)));
        assert!(collapse_for_inactivity(&content, Duration::from_secs(30)));
        content
            .glyphs
            .push(fixture(StateTint::Waiting, Metric::None));
        assert!(!collapse_for_inactivity(&content, Duration::from_secs(90)));
        content.glyphs[1].permission = None;
        assert!(!collapse_for_inactivity(&content, Duration::from_secs(90)));
        content.glyphs[1].state = StateTint::Completed;
        assert!(collapse_for_inactivity(&content, Duration::from_secs(90)));
        assert!(!collapse_for_inactivity(&content, Duration::ZERO));
    }

    #[test]
    fn permission_replaces_its_agents_usage_at_the_same_anchor() {
        let mut usage = OverlayContent {
            glyphs: vec![fixture(StateTint::Neutral, Metric::Fraction(0.5)); 3],
            overflow_count: None,
        };
        usage.glyphs[1].label = "ChatGPT".into();
        usage.glyphs[2].label = "OpenCode".into();
        for selected in 0..3 {
            let mut waiting = usage.clone();
            waiting.glyphs[selected].permission =
                fixture(StateTint::Waiting, Metric::None).permission;
            assert_eq!(replacement_selection(&usage, &waiting, 0), selected);
            for dpi in [1.0, 1.25, 1.5, 2.0] {
                let before = compute_layout(&usage, 1.0, dpi, selected);
                let after = compute_layout(&waiting, 1.0, dpi, selected);
                assert_eq!(card_bounds(&before).0, card_bounds(&after).0);
                assert_eq!(before.glyph_tops, after.glyph_tops);
                assert_eq!(before.text_column_width, after.text_column_width);
                assert!(card_bounds(&after).0 + card_bounds(&after).1 <= after.height as f32 + 0.5);
            }
        }
    }

    #[test]
    fn compact_density_and_permission_summary_preserve_the_real_action() {
        let g = fixture(StateTint::Waiting, Metric::Fraction(0.65));
        let content = OverlayContent {
            glyphs: vec![g.clone(); 4],
            overflow_count: Some(2),
        };
        let fitted = fit_to_display(&content, 524.0);
        assert!(compute_layout(&fitted, 0.0, 1.0, 0).rail_height <= 524);
        assert_eq!(
            fitted.glyphs.len() as u32 + fitted.overflow_count.unwrap(),
            6
        );
        let request = g.permission.unwrap();
        let (lines, review) = permission_lines(&request);
        assert_eq!(lines[0], "cargo test --workspace");
        assert!(
            review,
            "hiding the JSON envelope requires full review before approval"
        );
        assert!(
            String::from_utf16_lossy(&wide_permission_review(&request)).contains(&request.detail)
        );
        assert_eq!(safe_display("safe\u{202e}danger"), "safe\\u{202e}danger");
    }

    /// These are renderer fixtures only. Neither the application nor its state source imports them.
    fn fixture(state: StateTint, metric: Metric) -> ToolGlyph {
        ToolGlyph {
            sessions: vec![],
            session_summary: None,
            selected_session: None,
            label: "Claude".into(),
            mark: '✳',
            brand_color: (217, 119, 87),
            state,
            metric,
            dimmed: false,
            permission: if state == StateTint::Waiting {
                Some(verge_core::domain::PermissionRequest {
                    id: 1,
                    session_id: "fixture-session".into(),
                    tool: "Bash".into(),
                    detail: "{\n  \"command\": \"cargo test --workspace\"\n}".into(),
                    cwd: "D:\\example-project".into(),
                    created: Instant::now() - std::time::Duration::from_secs(20),
                    expires: Instant::now() + std::time::Duration::from_secs(100),
                })
            } else {
                None
            },
            session_count: Some(2),
            usage_windows: if let Metric::Fraction(f) = metric {
                vec![
                    verge_core::ports::UsageDetail {
                        label: "Current session".into(),
                        fraction: f,
                        reset: "Resets in 51 min".into(),
                        dimmed: false,
                    },
                    verge_core::ports::UsageDetail {
                        label: "All models".into(),
                        fraction: 0.07,
                        reset: "Resets in 2d".into(),
                        dimmed: false,
                    },
                ]
            } else {
                vec![]
            },
            reminder: None,
            activity_label: None,
            detail_lines: vec!["Verge".into(), "Reading app-sidebar.tsx".into()],
        }
    }

    #[test]
    fn reference_usage_bands_caps_and_independent_activity() {
        for (used, color) in [
            (0.21, (0, 255, 136)),
            (0.499, (0, 255, 136)),
            (0.50, (242, 255, 0)),
            (0.52, (242, 255, 0)),
            (0.699, (242, 255, 0)),
            (0.70, (255, 63, 0)),
            (0.73, (255, 63, 0)),
            (1.0, (255, 63, 0)),
        ] {
            assert_eq!(tokens::usage_color(used), color);
        }
        assert_eq!(ring_arc_coverage(22.0, 0.0, 0.0, 0.0, 22.0, 2.0, 0.0), 0.0);
        assert_eq!(ring_arc_coverage(22.0, 0.0, 0.0, 0.0, 22.0, 2.0, 1.0), 1.0);
        // A round cap extends beyond the angular end, without becoming a square cut.
        assert!(ring_arc_coverage(22.0, 0.5, 0.0, 0.0, 22.0, 2.0, 0.25) > 0.9);
        assert_eq!(ring_arc_coverage(22.0, 2.0, 0.0, 0.0, 22.0, 2.0, 0.25), 0.0);
        unsafe {
            let dc = GetDC(None);
            for used in [0.21, 0.52, 0.73] {
                let mut content = OverlayContent {
                    glyphs: vec![fixture(StateTint::Working, Metric::Fraction(used))],
                    overflow_count: None,
                };
                let l = compute_layout(&content, 0.0, 1.25, 0);
                let mut fresh = vec![0; (l.width * l.height) as usize];
                let mut aged = fresh.clone();
                paint_capsule(&mut fresh, &l, &content, dc, 0.0, 0.0);
                content.glyphs[0].dimmed = true;
                paint_capsule(&mut aged, &l, &content, dc, 0.0, 0.0);
                let halo = ((l.glyph_tops[0] + l.p(tokens::RING_DIAMETER / 2.0)) * l.width
                    + l.p(tokens::GLYPH_COLUMN_WIDTH / 2.0 + 13.0))
                    as usize;
                assert_eq!(
                    fresh[halo], aged[halo],
                    "fresh activity must not inherit usage age"
                );
                assert_ne!(fresh, aged, "the aged usage itself must still dim");
                if let Some(dir) = std::env::var_os("VERGE_VISUAL_DIR") {
                    let dir = std::path::PathBuf::from(dir);
                    std::fs::create_dir_all(&dir).unwrap();
                    write_bmp(
                        &dir.join(format!(
                            "reference-{}-125.bmp",
                            (used * 100.0).round() as i32
                        )),
                        l.width,
                        l.height,
                        &fresh,
                    );
                }
            }
            let _ = ReleaseDC(None, dc);
        }
    }

    #[test]
    fn native_visual_states_and_geometry() {
        assert!(tokens::GLYPH_COLUMN_WIDTH >= tokens::RING_DIAMETER + 32.0);
        assert!(tokens::RING_DIAMETER > tokens::GLYPH_DIAMETER + 20.0);
        let working = fixture(StateTint::Working, Metric::Neutral);
        let waiting = fixture(StateTint::Waiting, Metric::Neutral);
        let stopped = fixture(StateTint::Completed, Metric::Neutral);
        let usage = fixture(StateTint::Neutral, Metric::Fraction(0.73));
        let states = [
            ("session-detail", {
                let mut g = usage.clone();
                g.sessions.push(verge_core::ports::SessionDetail {
                    id: "fixture-session".into(),
                    title: "Example project".into(),
                    lines: vec![
                        "Model not reported".into(),
                        "Context 82% · High".into(),
                        "Working".into(),
                        "Capacity · 200000 tokens".into(),
                    ],
                    priority: 3,
                });
                g.selected_session = Some(0);
                OverlayContent {
                    glyphs: vec![g],
                    overflow_count: None,
                }
            }),
            ("idle", OverlayContent::default()),
            (
                "working",
                OverlayContent {
                    glyphs: vec![working.clone()],
                    overflow_count: None,
                },
            ),
            (
                "permission",
                OverlayContent {
                    glyphs: vec![waiting.clone()],
                    overflow_count: None,
                },
            ),
            (
                "completed",
                OverlayContent {
                    glyphs: vec![stopped.clone()],
                    overflow_count: None,
                },
            ),
            (
                "usage",
                OverlayContent {
                    glyphs: vec![usage.clone()],
                    overflow_count: None,
                },
            ),
            (
                "two-agents",
                OverlayContent {
                    glyphs: vec![working.clone(), waiting.clone()],
                    overflow_count: None,
                },
            ),
            (
                "four-agents",
                OverlayContent {
                    glyphs: vec![
                        working.clone(),
                        waiting.clone(),
                        stopped.clone(),
                        usage.clone(),
                    ],
                    overflow_count: None,
                },
            ),
            (
                "overflow",
                OverlayContent {
                    glyphs: vec![working, waiting, stopped, usage],
                    overflow_count: Some(2),
                },
            ),
        ];
        let output = std::env::var_os("VERGE_VISUAL_DIR").map(std::path::PathBuf::from);
        if let Some(dir) = &output {
            std::fs::create_dir_all(dir).unwrap();
        }
        unsafe {
            let dc = GetDC(None);
            for scale in [1.0, 1.25, 1.5, 2.0] {
                for (name, content) in &states {
                    for (phase, progress) in [("compact", 0.0), ("mid", 0.5), ("expanded", 1.0)] {
                        let l = compute_layout(content, progress, scale, 1);
                        let mut buf = vec![0; (l.width * l.height) as usize];
                        if content.glyphs.is_empty() {
                            paint_idle(&mut buf, &l);
                        } else {
                            paint_capsule(&mut buf, &l, content, dc, 0.0, 0.0);
                            // The logo's physical screen position must be invariant through expansion.
                            assert_eq!(
                                l.width - l.text_column_width,
                                l.p(tokens::GLYPH_COLUMN_WIDTH)
                            );
                            assert!(surface_coverage(0.0, 0.0, &l) < 0.01);
                            assert!(
                                surface_coverage(l.width as f32 - 0.5, l.height as f32 / 2.0, &l)
                                    > 0.99
                            );
                            let compact = compute_layout(content, 0.0, scale, 1);
                            assert_eq!(l.glyph_tops, compact.glyph_tops);
                        }
                        assert!(buf.iter().any(|p| *p != 0));
                        assert!(
                            buf.iter().all(|p| {
                                let a = p >> 24;
                                ((p >> 16) & 255) <= a && ((p >> 8) & 255) <= a && (p & 255) <= a
                            }),
                            "all pixels must remain premultiplied"
                        );
                        if let Some(dir) = &output {
                            write_bmp(
                                &dir.join(format!("{name}-{phase}-{}.bmp", (scale * 100.0) as i32)),
                                l.width,
                                l.height,
                                &buf,
                            );
                        }
                    }
                }
            }
            let mut mark = vec![0u32; 64 * 64];
            paint_brand_mark(
                "OpenCode",
                &mut mark,
                64,
                64,
                32,
                32,
                32,
                (255, 255, 255),
                1.0,
            );
            let total: f64 = mark.iter().map(|p| (p >> 24) as f64).sum();
            let center_x: f64 = mark
                .iter()
                .enumerate()
                .map(|(i, p)| ((i % 64) as f64 + 0.5) * (p >> 24) as f64)
                .sum::<f64>()
                / total;
            let center_y: f64 = mark
                .iter()
                .enumerate()
                .map(|(i, p)| ((i / 64) as f64 + 0.5) * (p >> 24) as f64)
                .sum::<f64>()
                / total;
            assert!(
                (center_x - 32.0).abs() < 0.1 && (center_y - 32.0).abs() < 0.1,
                "OpenCode must be centered"
            );
            // Usage age must not alter activity rings.
            let content = OverlayContent {
                glyphs: vec![fixture(StateTint::Neutral, Metric::Fraction(0.73))],
                overflow_count: None,
            };
            let l = compute_layout(&content, 0.0, 1.25, 0);
            let mut bright = vec![0; (l.width * l.height) as usize];
            let mut dim = bright.clone();
            paint_capsule(&mut bright, &l, &content, dc, 0.0, 0.0);
            let mut aged = content.clone();
            aged.glyphs[0].dimmed = true;
            paint_capsule(&mut dim, &l, &aged, dc, 0.0, 0.0);
            let arc = ((l.glyph_tops[0] + l.p(tokens::RING_DIAMETER / 2.0)) * l.width
                + l.p(tokens::GLYPH_COLUMN_WIDTH / 2.0 + tokens::RING_RADIUS))
                as usize;
            assert_eq!(dim[arc], bright[arc]);
            let mut completed = content.clone();
            completed.glyphs[0].state = StateTint::Completed;
            paint_capsule(&mut bright, &l, &completed, dc, 0.0, 0.0);
            completed.glyphs[0].state = StateTint::Stopped;
            paint_capsule(&mut dim, &l, &completed, dc, 0.0, 0.0);
            assert!(
                ((bright[arc] >> 8) & 255) > ((bright[arc] >> 16) & 255),
                "completed is green"
            );
            assert!(
                ((dim[arc] >> 16) & 255) > ((dim[arc] >> 8) & 255),
                "stopped is red"
            );
            for (state, phase, attention) in [
                (StateTint::Working, 3.0, 0.0),
                (StateTint::Waiting, 0.0, 1.0),
            ] {
                let animated = OverlayContent {
                    glyphs: vec![fixture(state, Metric::Neutral)],
                    overflow_count: None,
                };
                paint_capsule(&mut bright, &l, &animated, dc, 0.0, 0.0);
                paint_capsule(&mut dim, &l, &animated, dc, phase, attention);
                assert_ne!(bright, dim, "activity light must respond to animation time");
            }
            let _ = ReleaseDC(None, dc);
        }
        // Cubic movement settles monotonically, without overshoot or a bounce.
        let values: Vec<_> = (0..=100)
            .map(|i| tokens::ease_out_cubic(i as f32 / 100.0))
            .collect();
        assert_eq!(values[0], 0.0);
        assert_eq!(values[100], 1.0);
        assert!(values.windows(2).all(|pair| pair[1] >= pair[0]));
    }

    fn write_bmp(path: &std::path::Path, w: i32, h: i32, pixels: &[u32]) {
        // A native renderer capture on a neutral desktop; not an HTML recreation.
        let bytes = (w * h * 4) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(54 + bytes).to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&(-h).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&[0; 24]);
        for px in pixels {
            let a = (px >> 24) as f32 / 255.0;
            let channel = |shift: u32| {
                (((px >> shift) & 255u32) as f32 + 220.0 * (1.0 - a))
                    .round()
                    .min(255.0) as u8
            };
            out.extend_from_slice(&[channel(0), channel(8), channel(16), 255]);
        }
        std::fs::write(path, out).unwrap();
    }
}
