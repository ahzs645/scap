use anyhow::Result;
use cocoa::appkit::NSScreen;
use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use core_graphics_helmer_fork::display::{CGDirectDisplayID, CGMainDisplayID};
use objc::{msg_send, sel, sel_impl};

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
    let mut targets = Vec::new();

    // Try to get shareable content, but if it fails, fall back to basic methods
    match screencapturekit::shareable_content::SCShareableContent::get() {
        Ok(content) => {
            // Add displays using ScreenCaptureKit
            for (index, display) in content.displays().iter().enumerate() {
                targets.push(Target::Display(Display {
                    id: index as u32, // Use index as ID since actual display methods might not work
                    title: format!("Display {}", index + 1),
                    width: 1920, // Default width - getting actual dimensions might fail
                    height: 1080, // Default height
                    raw_handle: index as u32,
                }));
            }

            // Add windows using ScreenCaptureKit
            for (index, window) in content.windows().iter().enumerate() {
                // Only add windows that are likely to be capturable
                if is_window_capturable(window, index) {
                    targets.push(Target::Window(Window {
                        id: index as u32,
                        title: window.title().unwrap_or_else(|| format!("Window {}", index + 1)),
                        width: 800, // Default dimensions
                        height: 600,
                        app_name: window.owning_application()
                            .and_then(|app| app.application_name())
                            .unwrap_or_default(),
                        app_bundle_id: window.owning_application()
                            .and_then(|app| app.bundle_identifier())
                            .unwrap_or_default(),
                        is_on_screen: window.is_on_screen(),
                        process_id: window.owning_application()
                            .map(|app| app.process_id())
                            .unwrap_or(0),
                        window_level: window.window_level(),
                        has_shadow: true,
                        is_transparent: false,
                        raw_handle: index as u32,
                    }));
                }
            }
        }
        Err(_) => {
            // Fallback: create a default display target
            targets.push(Target::Display(Display {
                id: 0,
                title: "Main Display".to_string(),
                width: 1920,
                height: 1080,
                raw_handle: 0,
            }));
        }
    }

    Ok(targets)
}

fn is_window_capturable(window: &screencapturekit::window::SCWindow, index: usize) -> bool {
    // Basic checks
    if !window.is_on_screen() {
        return false;
    }

    let title = window.title().unwrap_or_default();
    if title.is_empty() || title.len() < 3 {
        return false;
    }

    // Skip system windows
    if is_system_window(window) {
        return false;
    }

    // Skip windows from the current process
    if let Some(app) = window.owning_application() {
        if app.process_id() == std::process::id() {
            return false;
        }
    }

    // Skip windows that are too small
    let frame = window.get_frame();
    if frame.size.width < 50.0 || frame.size.height < 50.0 {
        return false;
    }

    true
}

fn is_system_window(window: &screencapturekit::window::SCWindow) -> bool {
    let title = window.title().unwrap_or_default();
    let owner = window.owning_application()
        .and_then(|app| app.application_name())
        .unwrap_or_default();
    
    // Check for common system window patterns
    let system_patterns = [
        "Menubar", "Dock", "Desktop", "Notification Center",
        "Control Center", "Status Bar", "Menu Extra", "Spotlight",
        "Mission Control", "Dashboard",
    ];

    // Check for system applications
    let system_apps = [
        "Finder", "SystemUIServer", "Dock", "WindowServer",
        "loginwindow", "ControlCenter", "NotificationCenter",
    ];

    // Check title patterns
    if system_patterns.iter().any(|pattern| title.contains(pattern)) {
        return true;
    }

    // Check owner application
    if system_apps.iter().any(|app| owner == *app) {
        return true;
    }

    // Check window level (system windows typically have high levels)
    let window_level = window.window_level();
    if window_level > 1000 {
        return true;
    }

    false
}

pub fn get_main_display() -> Result<Display> {
    let id = unsafe { CGMainDisplayID() };
    let title = get_display_name(id);
    
    Ok(Display {
        id,
        title,
        width: 1920, // Default width - getting actual dimensions might require additional setup
        height: 1080, // Default height
        raw_handle: id,
    })
}

pub fn get_scale_factor(_target: &Target) -> f64 {
    // Return a reasonable default scale factor for macOS
    // In a full implementation, you'd get this from the actual display
    2.0 // Common for Retina displays
}

pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    match target {
        Target::Window(window) => (window.width, window.height),
        Target::Display(display) => (display.width, display.height),
    }
}