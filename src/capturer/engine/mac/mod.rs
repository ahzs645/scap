use std::sync::Arc;
use anyhow::Result;
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
use crate::frame::{Frame, BGRAFrame};
use crate::targets::Target;
use crate::capturer::Options;

mod pixel_buffer;
mod audio_buffer;

// Core Video pixel format constants
const K_CV_PIXEL_FORMAT_TYPE_32_BGRA: u32 = 1111970369; // 'BGRA'

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
        let filter = SCContentFilter::new();
        let mut config = SCStreamConfiguration::new();
        
        config.set_shows_cursor(options.show_cursor);
        config.set_pixel_format(K_CV_PIXEL_FORMAT_TYPE_32_BGRA);
        
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
        
        let stream = SCStream::new(&filter, &config);
        self.stream = Some(stream);
        
        Ok(())
    }

    pub async fn stop_capture(&mut self) -> Result<()> {
        if let Some(_stream) = &self.stream {
            self.stream = None;
        }
        Ok(())
    }
}

pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    match &options.target {
        Some(Target::Display(display)) => [display.width as u32, display.height as u32],
        Some(Target::Window(window)) => [window.width as u32, window.height as u32],
        None => [1920, 1080],
    }
}

pub fn process_sample_buffer(
    width: u32,
    height: u32,
    data: Vec<u8>,
    _frame_pool: &FramePool,
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