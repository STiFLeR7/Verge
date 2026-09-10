//! Read-only native review for actions that do not fit the ambient sheet.
use std::time::Instant;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Input::KeyboardAndMouse::SetFocus, WindowsAndMessaging::*},
    },
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND if [1, 2].contains(&(w.0 & 0xffff)) => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (w.0 & 0xffff) as isize);
            LRESULT(0)
        }
        WM_CLOSE => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 2);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

/// Escape/close/expiry/errors deny. Native edit supports selection, scrolling and assistive technology.
pub unsafe fn review(owner: HWND, text: &[u16], expires: Instant) -> bool {
    let run = || -> windows::core::Result<bool> {
        let instance = GetModuleHandleW(None)?;
        let class = wide("VergePermissionReview");
        let _ = RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            lpszClassName: PCWSTR(class.as_ptr()),
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut _),
            ..Default::default()
        });
        let scale = windows::Win32::UI::HiDpi::GetDpiForWindow(owner).max(96) as f32 / 96.0;
        let px = |v: f32| (v * scale).round() as i32;
        let width = px(580.0).min(GetSystemMetrics(SM_CXSCREEN) - 40);
        let height = px(440.0).min(GetSystemMetrics(SM_CYSCREEN) - 80);
        let title = wide("Claude — review this action");
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_DLGMODALFRAME,
            PCWSTR(class.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_CAPTION | WS_SYSMENU,
            (GetSystemMetrics(SM_CXSCREEN) - width) / 2,
            (GetSystemMetrics(SM_CYSCREEN) - height) / 2,
            width,
            height,
            owner,
            None,
            instance,
            None,
        )?;
        let result = (|| -> windows::core::Result<bool> {
            let mut bounds = RECT::default();
            GetClientRect(hwnd, &mut bounds)?;
            let margin = px(16.0);
            let button_h = px(36.0);
            let edit = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                PCWSTR(wide("EDIT").as_ptr()),
                PCWSTR(text.as_ptr()),
                WS_CHILD
                    | WS_VISIBLE
                    | WS_TABSTOP
                    | WS_VSCROLL
                    | WINDOW_STYLE((ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32),
                margin,
                margin,
                bounds.right - 2 * margin,
                bounds.bottom - 3 * margin - button_h,
                hwnd,
                None,
                instance,
                None,
            )?;
            let font = GetStockObject(DEFAULT_GUI_FONT);
            SendMessageW(edit, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
            let mut deny = HWND::default();
            for (id, label) in [(2, "Deny"), (1, "Approve once")] {
                let x = if id == 2 {
                    margin
                } else {
                    bounds.right - margin - px(160.0)
                };
                let button = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    PCWSTR(wide("BUTTON").as_ptr()),
                    PCWSTR(wide(label).as_ptr()),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                    x,
                    bounds.bottom - margin - button_h,
                    px(160.0),
                    button_h,
                    hwnd,
                    HMENU(id as *mut _),
                    instance,
                    None,
                )?;
                SendMessageW(button, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
                if id == 2 {
                    deny = button;
                }
            }
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(owner, false);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
            let _ = SetFocus(deny);
            let mut message = MSG::default();
            while GetWindowLongPtrW(hwnd, GWLP_USERDATA) == 0 && Instant::now() < expires {
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    if message.message == WM_QUIT {
                        PostQuitMessage(message.wParam.0 as i32);
                        return Ok(false);
                    }
                    if message.message == WM_KEYDOWN && message.wParam.0 == 27 {
                        return Ok(false);
                    }
                    if !IsDialogMessageW(hwnd, &message).as_bool() {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
            Ok(Instant::now() < expires && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == 1)
        })();
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(owner, true);
        let _ = DestroyWindow(hwnd);
        result
    };
    run().unwrap_or(false)
}
