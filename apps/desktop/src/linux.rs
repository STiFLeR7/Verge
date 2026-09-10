pub fn run() -> std::io::Result<()> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(std::io::Error::other("Verge currently requires X11 or XWayland (DISPLAY). Native Wayland layer-shell is not implemented."));
    }
    verge_ui_ambient_linux_x11::run_states(verge_desktop::local_states)
}
