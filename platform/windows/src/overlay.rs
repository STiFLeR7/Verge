//! Windows `OverlaySurface` implementation.
//!
//! Mechanism validated empirically by the disposable overlay-capability
//! spike (`D:\overlay-capability-spike\windows`, results summarized in
//! `SPIKE_RESULTS.md` §2 and §10) before being reimplemented here as
//! production code: `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW |
//! WS_EX_NOACTIVATE`, color-key transparency via
//! `SetLayeredWindowAttributes`, Per-Monitor-V2 DPI awareness, and a
//! periodic `HWND_TOPMOST` re-assertion (observed necessary — see the
//! spike's Test 2/3 notes).
//!
//! **Deliberate simplification for this first vertical slice:** this
//! surface has no interactive region, so `WS_EX_TRANSPARENT` is set once,
//! permanently, right after window creation (see the note below on why it
//! cannot be part of `CreateWindowExW`'s own style bits). The spike's other
//! key finding — that
//! `WM_NCHITTEST`/`HTTRANSPARENT` alone does *not* deliver clicks to a
//! different-process window, and that click-through-with-one-interactive-
//! region instead requires dynamically toggling `WS_EX_TRANSPARENT` off a
//! cursor-position poll — is preserved as a design note here, not
//! implemented, because nothing in this surface is clickable yet. Adding
//! the first interactive control is what makes that toggle necessary again.

use std::sync::{Arc, Mutex};

use verge_core::ports::{OverlayContent, OverlaySurface};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DrawTextW, EndPaint, FillRect, SetBkMode, SetTextColor, DT_LEFT,
    DT_NOCLIP, DT_WORDBREAK, PAINTSTRUCT, TRANSPARENT as GDI_TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW,
    GetSystemMetrics, GetWindowLongPtrW, LoadCursorW, PostQuitMessage, RegisterClassExW,
    SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    TranslateMessage, HWND_TOPMOST, IDC_ARROW, LWA_COLORKEY, MSG, SM_CXSCREEN, SM_CYSCREEN,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE, WM_DESTROY, WM_PAINT, WM_TIMER,
    WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

const WINDOW_WIDTH: i32 = 320;
const WINDOW_HEIGHT: i32 = 72;
const SCREEN_MARGIN: i32 = 24;
/// An RGB value chosen to be vanishingly unlikely to appear in rendered
/// text; every pixel this color is treated as transparent by the OS.
const TRANSPARENT_KEY: COLORREF = COLORREF(0x00FF00FE);
const TEXT_COLOR: COLORREF = COLORREF(0x00E6E6E6);
/// Re-assert top-most and refresh content on this cadence. The spike found
/// top-most needs periodic re-assertion; there is no reason to poll faster
/// than a human-perceptible ambient indicator needs to update.
const REFRESH_MS: u32 = 3000;

const TIMER_REFRESH: usize = 1;

struct WindowState<F: Fn() -> OverlayContent> {
    content_source: F,
    current: Mutex<OverlayContent>,
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
    // virtualizes coordinates for this process (the same bug the spike's
    // own PowerShell test harness hit before declaring this context).
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

    let screen_w = GetSystemMetrics(SM_CXSCREEN);
    let screen_h = GetSystemMetrics(SM_CYSCREEN);
    let x = screen_w - WINDOW_WIDTH - SCREEN_MARGIN;
    let y = screen_h - WINDOW_HEIGHT - SCREEN_MARGIN;

    let boxed_source: Box<dyn Fn() -> OverlayContent> = Box::new(content_source);
    let state = Arc::new(WindowState {
        content_source: boxed_source,
        current: Mutex::new(OverlayContent {
            lines: vec!["Verge starting…".to_string()],
        }),
    });
    let state_ptr = Arc::into_raw(state.clone());

    // WS_EX_TRANSPARENT is applied after creation (below), matching the
    // spike's validated mechanism, which only ever toggled this bit
    // post-creation rather than baking it into CreateWindowExW's style.
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        PCWSTR(class_name.as_ptr()),
        PCWSTR(to_wide("Verge").as_ptr()),
        WS_POPUP,
        x,
        y,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
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

    // Now safe to add click-through: the window already has its real,
    // final position and size.
    let current_ex_style =
        GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE);
    SetWindowLongPtrW(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE,
        current_ex_style | (WS_EX_TRANSPARENT.0 as isize),
    );

    SetLayeredWindowAttributes(hwnd, TRANSPARENT_KEY, 0, LWA_COLORKEY)
        .map_err(|e| std::io::Error::other(format!("SetLayeredWindowAttributes: {e}")))?;

    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    // SWP_NOMOVE | SWP_NOSIZE are not optional here: without them, the
    // (0, 0, 0, 0) position/size arguments are taken literally and Windows
    // collapses the window to a zero-size rect at the screen origin — a
    // real bug hit and fixed during this slice's own bring-up, not a
    // finding from the spike.
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

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    // Drop our extra Arc strong count; the window proc holds the other one
    // for the life of the window and its own WM_DESTROY drops it.
    drop(Arc::from_raw(state_ptr));

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
            if wparam.0 == TIMER_REFRESH {
                let ptr =
                    GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA);
                if ptr != 0 {
                    let state = &*(ptr as *const WindowState<Box<dyn Fn() -> OverlayContent>>);
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
                    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(hwnd, None, true);
                }
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let brush = CreateSolidBrush(TRANSPARENT_KEY);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            FillRect(hdc, &rect, brush);

            let ptr =
                GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA);
            if ptr != 0 {
                let state = &*(ptr as *const WindowState<Box<dyn Fn() -> OverlayContent>>);
                let content = state.current.lock().unwrap();
                let text = content.lines.join("\n");
                let mut wide = to_wide(&text);
                SetBkMode(hdc, GDI_TRANSPARENT);
                SetTextColor(hdc, TEXT_COLOR);
                let mut text_rect = rect;
                text_rect.left += 8;
                text_rect.top += 8;
                DrawTextW(
                    hdc,
                    &mut wide,
                    &mut text_rect,
                    DT_LEFT | DT_WORDBREAK | DT_NOCLIP,
                );
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr =
                GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA);
            if ptr != 0 {
                drop(Arc::from_raw(
                    ptr as *const WindowState<Box<dyn Fn() -> OverlayContent>>,
                ));
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
