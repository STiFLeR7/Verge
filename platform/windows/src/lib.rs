//! Windows implementations of `verge_core::ports` capability contracts.

mod credential_store;
#[cfg(windows)]
mod process_probe;
#[cfg(windows)]
pub use process_probe::WindowsProcessProbe;

#[cfg(windows)]
mod overlay;

#[cfg(windows)]
mod tokens;

pub use credential_store::WindowsCredentialStore;

#[cfg(windows)]
pub use overlay::WindowsOverlaySurface;

#[cfg(windows)]
mod permission_pipe;
#[cfg(windows)]
pub use permission_pipe::{claude_ancestor, request_permission, WindowsPermissions};

#[cfg(windows)]
mod permission_review;

#[cfg(windows)]
mod discovery;
#[cfg(windows)]
pub use discovery::{discover_tools, install_observers, observed_tools};
