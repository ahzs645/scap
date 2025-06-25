use super::{Display, Target, Window};
use anyhow::{Context as _, Result};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::HMONITOR,
};
use windows_capture::{monitor::Monitor, window::Window as WCWindow};

pub fn get_all_targets() -> Result<Vec<Target>> {
    let mut targets: Vec<Target> = Vec::new();

    // Add displays to targets
    let displays = Monitor::enumerate().context("Failed to enumerate monitors")?;
    for display in displays {
        let id = display.as_raw_hmonitor() as u32;
        let title = display
            .device_name()
            .context("Failed to get monitor name")?;

        let target = Target::Display(Display {
            id,
            title,
            raw_handle: HMONITOR(display.as_raw_hmonitor()),
            width: display.width()? as u64,
            height: display.height()? as u64,
        });
        targets.push(target);
    }

    // Add windows to targets
    let windows = WCWindow::enumerate().context("Failed to enumerate windows")?;
    for window in windows {
        let id = window.as_raw_hwnd() as u32;
        let title = window
            .title()
            .context("Window title not found")?
            .to_string();

        // Get window dimensions
        let (width, height) = get_window_dimensions(HWND(window.as_raw_hwnd()));

        let target = Target::Window(Window {
            id,
            title,
            width,
            height,
            app_name: String::new(), // TODO: Get actual app name
            app_bundle_id: String::new(), // TODO: Get actual bundle ID  
            is_on_screen: true, // TODO: Check if window is visible
            process_id: 0, // TODO: Get actual process ID
            window_level: 0,
            has_shadow: true,
            is_transparent: false,
            raw_handle: HWND(window.as_raw_hwnd()),
        });
        targets.push(target);
    }

    Ok(targets)
}

fn get_window_dimensions(hwnd: HWND) -> (u64, u64) {
    unsafe {
        let mut rect = RECT::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
        let width = (rect.right - rect.left) as u64;
        let height = (rect.bottom - rect.top) as u64;
        (width, height)
    }
}

pub fn get_main_display() -> Result<Display> {
    let display = Monitor::primary().context("Failed to get primary monitor")?;
    let id = display.as_raw_hmonitor() as u32;

    Ok(Display {
        id,
        title: display
            .device_name()
            .context("Failed to get monitor name")?,
        raw_handle: HMONITOR(display.as_raw_hmonitor()),
        width: display.width()? as u64,
        height: display.height()? as u64,
    })
}

// Referred to: https://github.com/tauri-apps/tao/blob/ab792dbd6c5f0a708c818b20eaff1d9a7534c7c1/src/platform_impl/windows/dpi.rs#L50
pub fn get_scale_factor(target: &Target) -> f64 {
    const BASE_DPI: u32 = 96;

    let mut dpi_x = 0;
    let mut dpi_y = 0;

    let dpi = match target {
        Target::Window(window) => unsafe { GetDpiForWindow(window.raw_handle) },
        Target::Display(display) => unsafe {
            if GetDpiForMonitor(
                display.raw_handle,
                MDT_EFFECTIVE_DPI,
                &mut dpi_x,
                &mut dpi_y,
            )
            .is_ok()
            {
                dpi_x
            } else {
                BASE_DPI
            }
        },
    };

    dpi as f64 / BASE_DPI as f64
}

pub fn get_target_dimensions(target: &Target) -> (u64, u64) {
    match target {
        Target::Window(window) => (window.width, window.height),
        Target::Display(display) => {
            let monitor = Monitor::from_raw_hmonitor(display.raw_handle.0);
            (
                monitor.width().unwrap_or(1920) as u64,
                monitor.height().unwrap_or(1080) as u64,
            )
        }
    }
}