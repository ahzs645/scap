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
        let [width, height] = Self::get_output_frame_size(&Options { target: Some(target.clone()) });
        
        let mut config = SCStreamConfiguration::new();
        config.set_width(width);
        config.set_height(height);
        config.set_shows_cursor(true);

        let content = SCShareableContent::get()
            .map_err(|e| anyhow::anyhow!("Failed to get shareable content: {:?}", e))?;
        let mut filter = SCContentFilter::new();
        
        match target {
            Target::Window(window) => {
                let target = content.windows().iter()
                    .find(|w| w.window_id() == window.id as u32)
                    .ok_or_else(|| anyhow::anyhow!("Window not found"))?;
                filter.include_window(target.clone());
            }
            Target::Display(display) => {
                let target = content.displays().iter()
                    .find(|d| d.display_id() == display.id as u32)
                    .ok_or_else(|| anyhow::anyhow!("Display not found"))?;
                filter.include_display(target.clone());
            }
            _ => {
                // Capture all displays
                if let Some(display) = content.displays().first() {
                    filter.include_display(display.clone());
                } else {
                    return Err(anyhow::anyhow!("No displays found"));
                }
            }
        }

        let stream = SCStream::new(&filter, &config)
            .map_err(|e| anyhow::anyhow!("Failed to create stream: {:?}", e))?;

        let frame_pool = Arc::clone(&self.frame_pool);
        let frame_sender = self.frame_sender.clone();
        stream.add_frame_handler(Box::new(move |frame| {
            if let Some(frame) = frame_pool.as_ref().push_frame(frame) {
                if let Err(e) = frame_sender.send(Ok(frame)) {
                    eprintln!("Failed to send frame: {:?}", e);
                }
            }
        }));

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
        Ok(self.frame_pool.as_ref().pop_frame())
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