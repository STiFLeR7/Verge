//! Windows implementations of `verge_core::ports` capability contracts.

mod credential_store;

#[cfg(windows)]
mod overlay;

pub use credential_store::WindowsCredentialStore;

#[cfg(windows)]
pub use overlay::WindowsOverlaySurface;
