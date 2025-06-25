#[cfg(target_os = "macos")]
mod mac;

#[cfg(target_os = "windows")]
mod win;

use anyhow::Result;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(crate) mod linux;

#[derive(Debug, Clone)]
pub struct Window {
    pub id: u32,
    pub title: String,
    pub width: u64,
    pub height: u64,
    pub app_name: String,
    pub app_bundle_id: String,
    pub is_on_screen: bool,
    pub process_id: u32,
    pub window_level: i32,
    pub has_shadow: bool,
    pub is_transparent: bool,
    pub raw_handle: screencapturekit::window::SCWindow,
}

#[derive(Debug, Clone)]
pub struct Display {
    pub id: u32,
    pub title: String,
    pub width: u64,
    pub height: u64,
    pub raw_handle: screencapturekit::display::SCDisplay,
}

#[derive(Debug, Clone)]
pub enum Target {
    Display(Display),
    Window(Window),
}

// Both `HWND` and `HMONITOR` are `Send` and `Sync`, so we can safely implement these traits for `Target`
#[cfg(target_os = "windows")]
unsafe impl Send for Target {}
#[cfg(target_os = "windows")]
unsafe impl Sync for Target {}

/// Returns a list of targets that can be captured
pub fn get_all_targets() -> Result<Vec<Target>> {
    #[cfg(target_os = "macos")]
    return mac::get_all_targets();

    #[cfg(target_os = "windows")]
    return win::get_all_targets();

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return linux::get_all_targets();
}

pub fn get_scale_factor(target: &Target) -> f64 {
    #[cfg(target_os = "macos")]
    return mac::get_scale_factor(target);

    #[cfg(target_os = "windows")]
    return win::get_scale_factor(target);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return 1.0;
}

pub fn get_main_display() -> Result<Display> {
    #[cfg(target_os = "macos")]
    return mac::get_main_display();

    #[cfg(target_os = "windows")]
    return win::get_main_display();

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return linux::get_main_display();
}

pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    #[cfg(target_os = "macos")]
    return mac::get_target_dimensions(target);

    #[cfg(target_os = "windows")]
    return win::get_target_dimensions(target);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    return linux::get_target_dimensions(target);
}
