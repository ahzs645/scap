use anyhow::Result;
use cocoa::appkit::{NSApp, NSScreen};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSRect, NSString, NSUInteger};
use core_graphics_helmer_fork::display::{CGDirectDisplayID, CGDisplay, CGMainDisplayID};
use core_graphics_helmer_fork::window::CGWindowID;
use objc::{msg_send, sel, sel_impl};
use screencapturekit::shareable_content::SCShareableContent;
use screencapturekit::display::SCDisplay;
use screencapturekit::window::SCWindow;

use super::{Display, Target, Window};

fn get_display_name(display_id: CGDirectDisplayID) -> String {
    unsafe {
        // Get all screens
        let screens: id = NSScreen::screens(nil);
        let count: u64 = msg_send![screens, count];

        for i in 0..count {
            let screen: id = msg_send![screens, objectAtIndex: i];
            let device_description: id = msg_send![screen, deviceDescription];
            let display_id_number: id = msg_send![device_description, objectForKey: NSString::alloc(nil).init_str("NSScreenNumber")];
            let display_id_number: u32 = msg_send![display_id_number, unsignedIntValue];

            if display_id_number == display_id {
                let localized_name: id = msg_send![screen, localizedName];
                let name: *const i8 = msg_send![localized_name, UTF8String];
                return std::ffi::CStr::from_ptr(name)
                    .to_string_lossy()
                    .into_owned();
            }
        }

        format!("Unknown Display {}", display_id)
    }
}

pub fn get_all_targets() -> Result<Vec<Target>> {
    let sc_shareable_content = screencapturekit::ShareableContent::get()
        .map_err(|e| anyhow::anyhow!("Failed to get shareable content: {}", e))?;

    let mut targets = Vec::new();

    // Add displays
    for display in sc_shareable_content.displays() {
        targets.push(Target::Display(Display {
            id: display.display_id() as u32,
            title: format!("Display {}", display.display_id()),
            width: display.width() as u64,
            height: display.height() as u64,
            raw_handle: display,
        }));
    }

    // Add windows with better filtering
    for window in sc_shareable_content.windows() {
        if !is_system_window(&window) && is_window_capturable(&window) {
            let app = window.owning_application();
            targets.push(Target::Window(Window {
                id: window.window_id() as u32,
                title: window.title().unwrap_or_default(),
                width: window.get_frame().size.width as u64,
                height: window.get_frame().size.height as u64,
                app_name: app.as_ref().and_then(|a| a.application_name()).unwrap_or_default(),
                app_bundle_id: app.as_ref().and_then(|a| a.bundle_identifier()).unwrap_or_default(),
                is_on_screen: window.is_on_screen(),
                process_id: app.as_ref().map(|a| a.process_id()).unwrap_or(0),
                window_level: window.window_level(),
                has_shadow: true, // Default value
                is_transparent: false, // Default value
                raw_handle: window,
            }));
        }
    }

    Ok(targets)
}

fn is_system_window(window: &SCWindow) -> bool {
    let title = window.title().unwrap_or_default();
    let owner = window.owning_application().map(|app| app.application_name()).flatten().unwrap_or_default();
    
    // Check for common system window patterns
    let system_patterns = [
        "Menubar",
        "Dock",
        "Desktop",
        "Notification Center",
        "Control Center",
        "Status Bar",
        "Menu Extra",
        "Spotlight",
        "Mission Control",
        "Dashboard",
    ];

    // Check for system applications
    let system_apps = [
        "Finder",
        "SystemUIServer",
        "Dock",
        "WindowServer",
        "loginwindow",
        "ControlCenter",
        "NotificationCenter",
    ];

    // Check title patterns
    if system_patterns.iter().any(|pattern| title.contains(pattern)) {
        return true;
    }

    // Check owner application
    if system_apps.iter().any(|app| owner == *app) {
        return true;
    }

    // Check window level
    let window_level = window.window_level();
    if window_level > 1000 { // System window levels are typically very high
        return true;
    }

    false
}

fn is_window_capturable(window: &SCWindow) -> bool {
    // Basic checks
    if !window.is_on_screen() {
        return false;
    }

    let title = window.title().unwrap_or_default();
    if title.is_empty() || title.len() < 3 {
        return false;
    }

    // Skip untitled windows
    if title.starts_with("Untitled") || title.starts_with("Item-") {
        return false;
    }

    // Skip windows with no content
    let frame = window.get_frame();
    if frame.size.width < 50.0 || frame.size.height < 50.0 {
        return false;
    }

    // Skip windows from the current process
    if let Some(app) = window.owning_application() {
        if app.process_id() == std::process::id() {
            return false;
        }
    }

    true
}

pub fn get_main_display() -> Result<Display> {
    let id = unsafe { CGMainDisplayID() };
    let title = get_display_name(id);
    
    // Get the display from ScreenCaptureKit
    let content = screencapturekit::ShareableContent::get()
        .map_err(|e| anyhow::anyhow!("Failed to get shareable content: {}", e))?;
    
    let display = content.displays()
        .into_iter()
        .find(|d| d.display_id() as u32 == id)
        .ok_or_else(|| anyhow::anyhow!("Main display not found"))?;

    Ok(Display {
        id,
        title,
        width: display.width() as u64,
        height: display.height() as u64,
        raw_handle: display,
    })
}

pub fn get_scale_factor(target: &Target) -> f64 {
    match target {
        Target::Window(_window) => {
            // For windows, return a default scale factor to avoid null pointer issues
            // ScreenCaptureKit will handle the actual scaling internally
            2.0 // Default for Retina displays, fallback value
        },
        Target::Display(display) => {
            let mode = display.raw_handle.display_mode().unwrap();
            (mode.pixel_width() / mode.width()) as f64
        }
    }
}

pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    match target {
        Target::Window(window) => {
            // For windows, get actual dimensions from the window
            if let Some(bounds) = super::super::capturer::engine::mac::window::get_window_bounds_with_shadow(window.id) {
                (bounds.size.width as u64, bounds.size.height as u64)
            } else {
                (window.width, window.height)
            }
        },
        Target::Display(display) => {
            let mode = display.raw_handle.display_mode().unwrap();
            (mode.width(), mode.height())
        }
    }
}
