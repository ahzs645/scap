use anyhow::{anyhow, Result};
use core_graphics::window::CGWindowID;
use screencapturekit::{
    stream::{SCStream, SCStreamConfiguration},
    SCContentFilter,
    SCWindow,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::Target;

#[derive(Debug, Clone)]
pub struct WindowState {
    pub is_visible: bool,
    pub is_minimized: bool,
    pub is_occluded: bool,
    pub bounds: core_graphics::geometry::CGRect,
    pub z_order: i32,
    pub has_shadow: bool,
    pub is_transparent: bool,
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub app_name: String,
    pub app_bundle_id: String,
    pub is_on_screen: bool,
    pub bounds: core_graphics::geometry::CGRect,
    pub process_id: u32,
    pub window_level: i32,
    pub has_shadow: bool,
    pub is_transparent: bool,
}

#[derive(Debug)]
pub enum WindowEvent {
    Closed,
    Minimized,
    Restored,
    Moved,
    Resized,
    OcclusionChanged,
}

pub struct WindowCaptureSession {
    window_id: u32,
    state: Arc<Mutex<WindowState>>,
    stream: SCStream,
}

impl WindowCaptureSession {
    pub fn new(window_id: u32, stream: SCStream) -> Self {
        let state = Arc::new(Mutex::new(WindowState {
            is_visible: true,
            is_minimized: false,
            is_occluded: false,
            bounds: core_graphics::geometry::CGRect::new(
                &core_graphics::geometry::CGPoint::new(0.0, 0.0),
                &core_graphics::geometry::CGSize::new(0.0, 0.0),
            ),
            z_order: 0,
            has_shadow: true,
            is_transparent: false,
        }));

        Self {
            window_id,
            state,
            stream,
        }
    }

    pub async fn handle_window_event(&mut self, event: WindowEvent) -> Result<()> {
        let mut state = self.state.lock().await;
        match event {
            WindowEvent::Closed => {
                self.stream.stop_capture()?;
                Ok(())
            }
            WindowEvent::Minimized => {
                state.is_minimized = true;
                state.is_visible = false;
                self.stream.stop_capture()?;
                Ok(())
            }
            WindowEvent::Restored => {
                state.is_minimized = false;
                state.is_visible = true;
                self.stream.start_capture()?;
                Ok(())
            }
            WindowEvent::Moved | WindowEvent::Resized => {
                // Update window bounds
                if let Some(new_bounds) = get_window_bounds_with_shadow(self.window_id) {
                    state.bounds = new_bounds;
                }
                Ok(())
            }
            WindowEvent::OcclusionChanged => {
                state.is_occluded = is_window_occluded(self.window_id);
                Ok(())
            }
        }
    }

    pub async fn is_window_capturable(&self) -> bool {
        let state = self.state.lock().await;
        state.is_visible && !state.is_minimized && !state.is_occluded
    }
}

pub fn configure_stream_for_window(
    config: &mut SCStreamConfiguration,
    window: &Target,
    options: &crate::capturer::Options,
) -> Result<()> {
    if let Target::Window(window_info) = window {
        // Get actual window bounds including shadow
        let bounds = get_window_bounds_with_shadow(window_info.id)
            .ok_or_else(|| anyhow!("Failed to get window bounds"))?;

        // Configure stream for window capture
        config.set_width(bounds.size.width as u32);
        config.set_height(bounds.size.height as u32);
        config.set_shows_cursor(options.show_cursor);

        // Set window-specific options
        config.set_captures_shadow(true);
        config.set_pixel_format(screencapturekit::sys::kCVPixelFormatType_32BGRA);

        // Apply padding if specified
        if let Some(padding) = options.window_frame_padding {
            let padded_width = bounds.size.width + (padding * 2.0);
            let padded_height = bounds.size.height + (padding * 2.0);
            config.set_width(padded_width as u32);
            config.set_height(padded_height as u32);
        }
    }
    Ok(())
}

pub fn create_window_filter(window: &Target, options: &crate::capturer::Options) -> Result<SCContentFilter> {
    if let Target::Window(window_info) = window {
        let sc_window = get_sc_window(window_info.id)
            .ok_or_else(|| anyhow!("Failed to get SCWindow"))?;

        let mut filter = SCContentFilter::new()
            .with_desktop_independent_window(&sc_window);

        // Optionally exclude overlapping windows
        if options.exclude_overlapping_windows.unwrap_or(false) {
            let overlapping = get_overlapping_windows(&sc_window);
            filter = filter.excluding_windows(&overlapping);
        }

        Ok(filter)
    } else {
        Err(anyhow!("Not a window target"))
    }
}

fn get_window_bounds_with_shadow(window_id: CGWindowID) -> Option<core_graphics::geometry::CGRect> {
    use core_graphics::window::{
        kCGWindowListOptionIncludingWindow,
        CGWindowListCopyWindowInfo,
    };
    use core_foundation::{
        array::CFArray,
        dictionary::CFDictionary,
        base::TCFType,
    };

    unsafe {
        let window_list = CGWindowListCopyWindowInfo(
            kCGWindowListOptionIncludingWindow,
            window_id as u32,
        );
        
        if let Some(window_list) = window_list {
            let array: CFArray = window_list.into_CFArray();
            if array.len() > 0 {
                if let Some(dict) = array.get(0) {
                    let dict: CFDictionary = dict.into_CFDictionary();
                    // Extract bounds including shadow
                    // Implementation would use CGWindowBounds key
                    // For now return a placeholder
                    return Some(core_graphics::geometry::CGRect::new(
                        &core_graphics::geometry::CGPoint::new(0.0, 0.0),
                        &core_graphics::geometry::CGSize::new(800.0, 600.0),
                    ));
                }
            }
        }
    }
    None
}

fn is_window_occluded(window_id: CGWindowID) -> bool {
    // Implementation would check window occlusion state
    // For now return a placeholder
    false
}

fn get_sc_window(window_id: CGWindowID) -> Option<SCWindow> {
    // Implementation would get SCWindow from window ID
    // For now return None as placeholder
    None
}

fn get_overlapping_windows(window: &SCWindow) -> Vec<SCWindow> {
    // Implementation would find overlapping windows
    // For now return empty vec as placeholder
    vec![]
}

pub fn get_detailed_window_list() -> Result<Vec<WindowInfo>> {
    // Implementation would return detailed window info
    // For now return empty vec as placeholder
    Ok(vec![])
} 