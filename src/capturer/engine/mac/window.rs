use anyhow::{anyhow, Result};
use core_graphics::window::CGWindowID;
use screencapturekit::{
    stream::{SCStream, configuration::SCStreamConfiguration},
    stream::content_filter::SCContentFilter,
    shareable_content::window::SCWindow,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    capturer::{Area, Options, Point, Resolution, Size},
    Target,
};

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
    stream: SCStream,
    window_id: CGWindowID,
    state: Arc<Mutex<WindowState>>,
}

impl WindowCaptureSession {
    pub async fn new(window: &SCWindow, options: &Options) -> Result<Self> {
        let window_id = window.window_id() as u32;
        
        let state = Arc::new(Mutex::new(WindowState {
            is_visible: true,
            is_minimized: false,
            is_occluded: false,
            bounds: window.frame(),
            z_order: 0,
            has_shadow: options.include_window_shadow.unwrap_or(true),
            is_transparent: false,
        }));

        let filter = create_window_filter(window, options)?;
        let config = configure_stream_for_window(options)?;
        
        let stream = SCStream::new(&filter, &config)
            .map_err(|e| anyhow!("Failed to create stream: {}", e))?;

        Ok(Self {
            stream,
            window_id,
            state,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        self.stream.start_capture()
            .map_err(|e| anyhow!("Failed to start capture: {}", e))
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.stream.stop_capture()
            .map_err(|e| anyhow!("Failed to stop capture: {}", e))
    }

    pub async fn update_state(&mut self) -> Result<()> {
        let mut state = self.state.lock().await;
        
        // Update window state
        if let Some(window) = get_window_info(self.window_id) {
            state.is_visible = window.is_on_screen();
            state.is_minimized = window.is_minimized();
            state.is_occluded = is_window_occluded(self.window_id);
            state.bounds = window.frame();
        } else {
            return Err(anyhow!("Window no longer exists"));
        }
        
        Ok(())
    }
}

pub fn create_window_filter(window: &SCWindow, options: &Options) -> Result<SCContentFilter> {
    let mut filter = SCContentFilter::new();
    
    // Set window as content source
    filter = filter.with_window(window);
    
    // Configure window-specific options
    if let Some(exclude_overlapping) = options.exclude_overlapping_windows {
        if exclude_overlapping {
            filter = filter.exclude_overlapping_windows();
        }
    }
    
    if let Some(include_shadow) = options.include_window_shadow {
        if include_shadow {
            filter = filter.include_window_shadow();
        }
    }
    
    Ok(filter)
}

pub fn configure_stream_for_window(options: &Options) -> Result<SCStreamConfiguration> {
    let mut config = SCStreamConfiguration::new();
    
    // Set basic stream properties
    config.set_shows_cursor(options.show_cursor);
    config.set_pixel_format(screencapturekit::sys::kCVPixelFormatType_32BGRA);
    
    if let Some(fps) = Some(options.fps) {
        config.set_minimum_frame_interval(1.0 / fps as f64);
    }
    
    // Configure window-specific options
    if let Some(padding) = options.window_frame_padding {
        config.set_window_frame_padding(padding);
    }
    
    if let Some(match_res) = options.match_window_resolution {
        config.set_matches_window_resolution(match_res);
    }
    
    // Configure audio if enabled
    if let Some(window_audio) = &options.window_audio {
        config.set_captures_audio(true);
        config.set_audio_application_only(window_audio.capture_window_audio_only);
        config.set_audio_ducking(window_audio.audio_ducking);
        
        if let Some(rate) = options.audio_sample_rate {
            config.set_sample_rate(rate);
        }
        
        if let Some(channels) = options.audio_channel_count {
            config.set_channel_count(channels as u8);
        }
    }
    
    Ok(config)
}

fn get_window_info(window_id: CGWindowID) -> Option<SCWindow> {
    if let Ok(content) = screencapturekit::ShareableContent::current() {
        content.windows().into_iter()
            .find(|w| w.window_id() as u32 == window_id)
    } else {
        None
    }
}

fn is_window_occluded(window_id: CGWindowID) -> bool {
    // TODO: Implement window occlusion detection
    false
}

pub fn get_detailed_window_list() -> Result<Vec<WindowInfo>> {
    // Implementation would return detailed window info
    // For now return empty vec as placeholder
    Ok(vec![])
} 