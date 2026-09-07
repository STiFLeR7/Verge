//! Verge core: pure domain model + application orchestration.
//!
//! This crate must never depend on an OS API, a vendor SDK, or a UI
//! framework. If a type here needs Win32, AppKit, X11, Wayland, or Tauri to
//! make sense, it belongs in `platform/`, `tools/`, or `ui/` instead.

pub mod application;
pub mod domain;
pub mod ports;
