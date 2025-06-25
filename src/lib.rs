//! Cross Platform, Performant and High Quality screen recordings

pub mod capturer;
pub mod frame;
mod targets;
mod utils;

// NAPI bindings for Node.js
#[cfg(feature = "napi")]
pub mod napi;

// Helper Methods
pub use targets::get_all_targets;
pub use targets::Target;
pub use targets::{Display, Window};
pub use utils::has_permission;
pub use utils::is_supported;
pub use utils::request_permission;

// Re-export main capturer types
pub use capturer::{Capturer, Options, Point, Size, Area, Resolution};

// Re-export NAPI bindings when feature is enabled
#[cfg(feature = "napi")]
pub use napi::*;