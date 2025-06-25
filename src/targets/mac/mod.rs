use anyhow::Result;
use cocoa::appkit::NSScreen;
use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use core_graphics_helmer_fork::display::{CGDirectDisplayID, CGMainDisplayID};
use objc::{msg_send, sel, sel_impl};

use super::{Display, Target, Window};

fn get_display_name(display_id: CGDirectDisplayID) -> String {
    unsafe {
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
        format!("Display {}", display_id)
    }
}

pub fn get_all_targets() -> Result<Vec<Target>> {
    let mut targets = Vec::new();

    // Get all displays
    unsafe {
        let screens: id = NSScreen::screens(nil);
        let count: u64 = msg_send![screens, count];

        for i in 0..count {
            let screen: id = msg_send![screens, objectAtIndex: i];
            let device_description: id = msg_send![screen, deviceDescription];
            let display_id_number: id = msg_send![device_description, objectForKey: NSString::alloc(nil).init_str("NSScreenNumber")];
            let display_id_number: u32 = msg_send![display_id_number, unsignedIntValue];

            let frame = unsafe { NSScreen::frame(screen) };
            targets.push(Target::Display(Display {
                id: display_id_number,
                title: get_display_name(display_id_number),
                width: frame.size.width as u64,
                height: frame.size.height as u64,
                raw_handle: display_id_number,
            }));
        }
    }

    // Get all windows
    let content = screencapturekit::shareable_content::SCShareableContent::get().map_err(|e| anyhow::anyhow!("Failed to get shareable content: {}", e))?;
    for window in content.windows() {
        if !is_window_capturable(&window) {
            continue;
        }

        let frame = window.get_frame();
        let app = window.owning_application();
        targets.push(Target::Window(Window {
            id: window.window_id(),
            title: window.title(),
            width: frame.size.width as u64,
            height: frame.size.height as u64,
            app_name: app.application_name(),
            app_bundle_id: app.bundle_identifier(),
            is_on_screen: window.is_on_screen(),
            process_id: app.process_id() as u32,
            window_level: window.window_layer() as i32,
            has_shadow: true,
            is_transparent: false,
            raw_handle: window.window_id(),
        }));
    }

    Ok(targets)
}

pub fn get_primary_target() -> Result<Target> {
    let display_id = unsafe { CGMainDisplayID() };
    let title = get_display_name(display_id);

    let screens: id = unsafe { NSScreen::screens(nil) };
    let screen: id = unsafe { msg_send![screens, objectAtIndex: 0] };
    let frame = unsafe { NSScreen::frame(screen) };

    Ok(Target::Display(Display {
        id: display_id,
        title,
        width: frame.size.width as u64,
        height: frame.size.height as u64,
        raw_handle: display_id,
    }))
}

fn is_window_capturable(window: &screencapturekit::shareable_content::window::SCWindow) -> bool {
    if !window.is_on_screen() {
        return false;
    }

    let title = window.title();
    if title.is_empty() || title.len() < 3 {
        return false;
    }

    if is_system_window(window) {
        return false;
    }

    let app = window.owning_application();
    if app.process_id() == std::process::id().try_into().unwrap_or(-1) {
        return false;
    }

    let frame = window.get_frame();
    if frame.size.width < 50.0 || frame.size.height < 50.0 {
        return false;
    }

    true
}

fn is_system_window(window: &screencapturekit::shareable_content::window::SCWindow) -> bool {
    let title = window.title();
    let app = window.owning_application();
    let owner = app.application_name();
    
    let system_patterns = [
        "Menubar", "Dock", "Desktop", "Notification Center",
        "Control Center", "Status Bar", "Menu Extra", "Spotlight",
        "Mission Control", "Dashboard",
    ];

    let system_apps = [
        "Finder", "SystemUIServer", "Dock", "WindowServer",
        "loginwindow", "ControlCenter", "NotificationCenter",
    ];

    if system_patterns.iter().any(|pattern| title.contains(pattern)) {
        return true;
    }

    if system_apps.iter().any(|app| owner == *app) {
        return true;
    }

    let window_level = window.window_layer();
    if window_level > 1000 {
        return true;
    }

    false
}

pub fn get_main_display() -> Result<Display> {
    let id = unsafe { CGMainDisplayID() };
    let title = get_display_name(id);
    let (width, height) = get_display_dimensions();
    
    Ok(Display {
        id,
        title,
        width,
        height,
        raw_handle: id,
    })
}

fn get_display_dimensions() -> (u64, u64) {
    if let Ok(content) = screencapturekit::shareable_content::SCShareableContent::get() {
        if let Some(display) = content.displays().first() {
            let frame = display.frame();
            return (frame.size.width as u64, frame.size.height as u64);
        }
    }
    (1920, 1080)
}

pub fn get_scale_factor(_target: &Target) -> f64 {
    2.0
}

pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    match target {
        Target::Window(window) => (window.width, window.height),
        Target::Display(display) => (display.width, display.height),
    }
}