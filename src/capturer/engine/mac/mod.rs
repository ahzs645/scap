use std::sync::{Arc, Mutex};
use anyhow::Result;
use screencapturekit::{
    stream::{
        configuration::SCStreamConfiguration,
        content_filter::SCContentFilter,
        SCStream,
    },
    shareable_content::SCShareableContent,
};
use crate::{
    capturer::{
        frame_pool::FramePool,
        AsyncFrameSender,
    },
    frame::{Frame, BGRAFrame},
    targets::Target,
    Options,
};

mod pixel_buffer;
mod audio_buffer;

pub struct ScreenCapturer {
    frame_pool: Arc<FramePool>,
    frame_sender: AsyncFrameSender,
    stream: Arc<Mutex<Option<SCStream>>>,
}

impl ScreenCapturer {
    pub fn new(frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Result<Self> {
        Ok(Self {
            frame_pool,
            frame_sender,
            stream: Arc::new(Mutex::new(None)),
        })
    }

    pub fn start_capture(&self, target: &Target) -> Result<()> {
        // Fix: Create complete Options struct with all required fields
        let options = Options {
            fps: 30,
            show_cursor: true,
            show_highlight: false,
            target: Some(target.clone()),
            crop_area: None,
            output_type: crate::frame::FrameType::BGRAFrame,
            output_resolution: crate::capturer::Resolution::Captured,
            excluded_targets: None,
            capture_system_audio: Some(false),
            exclude_current_process_audio: Some(true),
            audio_sample_rate: Some(48000),
            audio_channel_count: Some(2),
            capture_microphone: Some(false),
            microphone_device_id: None,
            window_audio: None,
            exclude_overlapping_windows: None,
            window_frame_padding: None,
            match_window_resolution: None,
            include_window_shadow: None,
        };

        let [width, height] = Self::get_output_frame_size(&options);
        
        let config = SCStreamConfiguration::new()
            .set_width(width)
            .map_err(|e| anyhow::anyhow!("Failed to set width: {:?}", e))?
            .set_height(height)
            .map_err(|e| anyhow::anyhow!("Failed to set height: {:?}", e))?
            .set_shows_cursor(true)
            .map_err(|e| anyhow::anyhow!("Failed to set cursor: {:?}", e))?;

        let content = SCShareableContent::get()
            .map_err(|e| anyhow::anyhow!("Failed to get shareable content: {:?}", e))?;
        let mut filter = SCContentFilter::new();
        
        match target {
            Target::Window(window) => {
                let windows = content.windows();
                let target = windows.iter()
                    .find(|w| w.window_id() == window.id as u32)
                    .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
                // Fix: Use correct method names for the new API
                filter = filter.with_desktop_independent_window(target);
            }
            Target::Display(display) => {
                let displays = content.displays();
                let target = displays.iter()
                    .find(|d| d.display_id() == display.id as u32)
                    .ok_or_else(|| anyhow::anyhow!("Display not found"))?;
                filter = filter.with_display_excluding_windows(target, &[]);
            }
        }

        let stream = SCStream::new(&filter, &config);

        // For now, we'll use a simpler approach without the output handler
        // TODO: Implement proper frame handling when the ScreenCaptureKit API is clarified
        
        stream.start_capture()
            .map_err(|e| anyhow::anyhow!("Failed to start capture: {:?}", e))?;

        *self.stream.lock().unwrap() = Some(stream);

        Ok(())
    }

    pub fn stop_capture(&self) -> Result<()> {
        if let Some(stream) = self.stream.lock().unwrap().take() {
            stream.stop_capture()
                .map_err(|e| anyhow::anyhow!("Failed to stop capture: {:?}", e))?;
        }
        Ok(())
    }

    pub fn get_frame(&self) -> Result<Option<Frame>> {
        // Fix: Use correct method name
        Ok(self.frame_pool.get_next_frame())
    }

    pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
        match &options.target {
            Some(Target::Display(display)) => [display.width as u32, display.height as u32],
            Some(Target::Window(window)) => [window.width as u32, window.height as u32],
            None => [1920, 1080],
        }
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