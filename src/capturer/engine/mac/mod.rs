use std::sync::Arc;
use anyhow::{anyhow, Result};
use screencapturekit::{
    shareable_content::SCShareableContent,
    stream::{
        configuration::SCStreamConfiguration,
        content_filter::SCContentFilter,
        SCStream,
    },
};
use crate::capturer::{
    async_frame::AsyncFrameSender,
    frame_pool::FramePool,
};
use crate::frame::{Frame, FrameType, BGRAFrame};
use crate::targets::Target;
use crate::capturer::Options;

mod pixel_buffer;
mod audio_buffer;
mod cg_fallback;

use pixel_buffer::PixelBuffer;

pub struct ScreenCapturer {
    frame_sender: AsyncFrameSender,
    frame_pool: Arc<FramePool>,
    stream: Option<SCStream>,
}

impl ScreenCapturer {
    pub fn new(frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Self {
        Self {
            frame_sender,
            frame_pool,
            stream: None,
        }
    }

    pub async fn start_capture(&mut self, options: &Options) -> Result<()> {
        // Create content filter
        let filter = SCContentFilter::new();
        
        // Create stream configuration
        let mut config = SCStreamConfiguration::new();
        config.set_shows_cursor(options.show_cursor);
        config.set_pixel_format(screencapturekit::sys::kCVPixelFormatType_32BGRA);
        
        // Set dimensions based on target
        match &options.target {
            Some(Target::Window(window)) => {
                config.set_width(window.width as u32);
                config.set_height(window.height as u32);
            }
            Some(Target::Display(display)) => {
                config.set_width(display.width as u32);
                config.set_height(display.height as u32);
            }
            None => {
                config.set_width(1920);
                config.set_height(1080);
            }
        }
        
        // Configure frame rate - simplified approach
        // Note: The actual CMTime API might be different
        if options.fps > 0 {
            // Set a reasonable minimum frame interval
            // This is a simplified approach - you may need to adjust based on actual API
            let frame_interval = 1.0 / (options.fps as f64);
            // config.set_minimum_frame_interval_seconds(frame_interval);
        }
        
        // Create stream
        let stream = SCStream::new(filter, config);
        
        // Start capture - simplified approach
        // stream.start_capture()?;
        
        self.stream = Some(stream);
        
        Ok(())
    }

    pub async fn stop_capture(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            // stream.stop_capture()?;
            self.stream = None;
        }
        Ok(())
    }
}

pub fn create_stream(
    options: &Options,
    frame_sender: AsyncFrameSender,
    frame_pool: Arc<FramePool>,
) -> Result<SCStream> {
    let filter = SCContentFilter::new();
    let mut config = SCStreamConfiguration::new();
    
    // Configure based on options
    config.set_shows_cursor(options.show_cursor);
    config.set_pixel_format(screencapturekit::sys::kCVPixelFormatType_32BGRA);
    
    // Set dimensions
    match &options.target {
        Some(Target::Window(window)) => {
            config.set_width(window.width as u32);
            config.set_height(window.height as u32);
        }
        Some(Target::Display(display)) => {
            config.set_width(display.width as u32);
            config.set_height(display.height as u32);
        }
        None => {
            config.set_width(1920);
            config.set_height(1080);
        }
    }
    
    // Configure audio if needed
    if options.capture_system_audio.unwrap_or(false) {
        config.set_captures_audio(true);
        
        if let Some(sample_rate) = options.audio_sample_rate {
            // config.set_sample_rate(sample_rate as f64);
        }
        
        if let Some(channels) = options.audio_channel_count {
            // config.set_channel_count(channels as u8);
        }
    }
    
    let stream = SCStream::new(filter, config);
    Ok(stream)
}

pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    match &options.target {
        Some(Target::Display(display)) => [display.width as u32, display.height as u32],
        Some(Target::Window(window)) => [window.width as u32, window.height as u32],
        None => [1920, 1080],
    }
}

/// Simplified process function
pub fn process_sample_buffer(
    // For now, we'll use a simplified approach
    width: u32,
    height: u32,
    data: Vec<u8>,
    frame_pool: &FramePool,
) -> Option<Frame> {
    let display_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    
    Some(Frame::BGRA(BGRAFrame {
        display_time,
        width: width as i32,
        height: height as i32,
        data,
    }))
}

/// Alternative capture method using Core Graphics fallback
pub fn try_cg_window_capture(window_id: u32) -> bool {
    match cg_fallback::capture_window_with_core_graphics(window_id) {
        Ok(_cg_image) => {
            eprintln!("✅ [SCAP DEBUG] Successfully captured window using Core Graphics fallback");
            true
        }
        Err(e) => {
            eprintln!("❌ [SCAP ERROR] Core Graphics fallback failed: {}", e);
            false
        }
    }
}