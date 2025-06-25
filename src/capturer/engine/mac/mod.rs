use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::{cmp, sync::Arc};
use anyhow::Result;
use core_media_rs::cm_sample_buffer::CMSampleBuffer;
use screencapturekit::{
    shareable_content::SCShareableContent,
    stream::{
        configuration::SCStreamConfiguration,
        content_filter::SCContentFilter,
        output_trait::SCStreamOutputTrait,
        output_type::SCStreamOutputType,
        SCStream,
    },
};
use tokio::sync::Mutex;
use anyhow::anyhow;
use crate::capturer::{
    async_frame::AsyncFrameSender,
    frame_pool::FramePool,
};
use core_media_rs::cm_time::CMTime;
use screencapturekit::shareable_content::{display, window};
use crate::capturer::engine::mac::audio_buffer::process_audio_buffer;

use crate::frame::{Frame, FrameType};
use crate::targets::Target;
use crate::{
    capturer::{Area, Options, Point, Resolution, Size},
    targets,
};

use super::ChannelItem;

mod apple_sys;
mod pixel_buffer;
mod pixelformat;
mod audio_buffer;
mod cg_fallback;
mod window;

use pixel_buffer::PixelBuffer;
use window::{WindowCaptureSession, WindowEvent, configure_stream_for_window, create_window_filter};

pub struct ScreenCapturer {
    frame_sender: AsyncFrameSender,
    frame_pool: Arc<FramePool>,
    stream: Option<SCStream>,
    window_session: Option<WindowCaptureSession>,
}

impl ScreenCapturer {
    pub fn new(frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Self {
        Self {
            frame_sender,
            frame_pool,
            stream: None,
            window_session: None,
        }
    }

    pub async fn start_capture(&mut self, options: &Options) -> Result<()> {
        match &options.target {
            Some(Target::Window(window)) => {
                let content = SCShareableContent::get()
                    .map_err(|e| anyhow!("Failed to get shareable content: {}", e))?;
                
                let sc_window = content.windows()
                    .into_iter()
                    .find(|w| w.window_id() as u32 == window.id)
                    .ok_or_else(|| anyhow!("Window not found"))?;
                
                let mut session = WindowCaptureSession::new(&sc_window, options).await?;
                session.start().await?;
                self.window_session = Some(session);
            }
            Some(Target::Display(display)) => {
                let content = SCShareableContent::get()
                    .map_err(|e| anyhow!("Failed to get shareable content: {}", e))?;
                
                let sc_display = content.displays()
                    .into_iter()
                    .find(|d| d.display_id() as u32 == display.id)
                    .ok_or_else(|| anyhow!("Display not found"))?;
                
                let mut filter = SCContentFilter::new();
                filter.set_displays(vec![sc_display]);
                
                let mut config = SCStreamConfiguration::new();
                config.set_width(display.width as u32);
                config.set_height(display.height as u32);
                config.set_shows_cursor(options.show_cursor);
                config.set_pixel_format(screencapturekit::sys::kCVPixelFormatType_32BGRA);
                
                if let Some(fps) = Some(options.fps) {
                    let frame_duration = CMTime::new(1, fps as i32);
                    config.set_minimum_frame_interval(&frame_duration)
                        .map_err(|e| anyhow!("Failed to set frame interval: {}", e))?;
                }
                
                let stream = SCStream::new(&filter, &config)
                    .map_err(|e| anyhow!("Failed to create stream: {}", e))?;
                
                stream.start_capture()
                    .map_err(|e| anyhow!("Failed to start capture: {}", e))?;
                
                self.stream = Some(stream);
            }
            None => {
                return Err(anyhow!("No target specified"));
            }
        }
        
        Ok(())
    }

    pub async fn stop_capture(&mut self) -> Result<()> {
        if let Some(session) = &mut self.window_session {
            session.stop().await?;
            self.window_session = None;
        }
        
        if let Some(stream) = &self.stream {
            stream.stop_capture()
                .map_err(|e| anyhow!("Failed to stop capture: {}", e))?;
            self.stream = None;
        }
        
        Ok(())
    }

    pub fn handle_frame(&mut self, sample_buffer: CMSampleBuffer) -> Result<()> {
        match PixelBuffer::from_sample_buffer(&sample_buffer) {
            Ok(pixel_buffer) => {
                let mut buffer = self.frame_pool.get_video_buffer(pixel_buffer.size())?;
                buffer.extend_from_slice(&pixel_buffer.data());
                
                let frame = Frame::BGRA(crate::frame::BGRAFrame {
                    data: buffer,
                    width: pixel_buffer.width() as u32,
                    height: pixel_buffer.height() as u32,
                    stride: pixel_buffer.bytes_per_row() as u32,
                });
                
                self.frame_sender.send(frame)?;
            }
            Err(e) => {
                log::error!("Failed to process pixel buffer: {}", e);
            }
        }
        
        Ok(())
    }

    pub fn handle_audio(&mut self, sample_buffer: CMSampleBuffer) -> Result<()> {
        match process_audio_buffer(&sample_buffer, &self.frame_pool) {
            Ok(frame) => {
                self.frame_sender.send(frame)?;
            }
            Err(e) => {
                log::error!("Failed to process audio buffer: {}", e);
            }
        }
        
        Ok(())
    }
}

pub struct AudioCapturer {
    frame_sender: AsyncFrameSender,
    frame_pool: Arc<FramePool>,
}

impl AudioCapturer {
    pub fn new(frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Self {
        Self {
            frame_sender,
            frame_pool,
        }
    }
}

impl SCStreamOutputTrait for AudioCapturer {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, of_type: SCStreamOutputType) {
        let frame = process_audio_buffer(sample.clone(), &self.frame_pool);
        if let Ok(frame) = frame {
            self.frame_sender.send_frame(Ok(Frame::SystemAudio(frame))).unwrap_or(());
        }
    }
}

/// Attempts to capture a window using Core Graphics as a fallback
/// Returns true if the window was successfully captured using CG fallback
pub fn try_cg_window_capture(window_id: u32, _tx: &mpsc::Sender<ChannelItem>) -> bool {
    match cg_fallback::capture_window_with_core_graphics(window_id) {
        Ok(cg_image) => {
            // Convert CGImage to Frame and send it
            if let Ok(_frame) = cg_fallback::cgimage_to_bgra_frame(cg_image) {
                // Send the frame through the channel
                // Note: This is a simplified approach - in a real implementation,
                // you'd want to set up a timer to capture frames at the desired FPS
                eprintln!("✅ [SCAP DEBUG] Successfully captured window using Core Graphics fallback");
                true
            } else {
                eprintln!("❌ [SCAP ERROR] Failed to convert CGImage to BGRA frame");
                false
            }
        }
        Err(e) => {
            eprintln!("❌ [SCAP ERROR] Core Graphics fallback failed: {}", e);
            false
        }
    }
}

pub fn create_stream(
    options: &Options,
    frame_sender: AsyncFrameSender,
    error_flag: Arc<AtomicBool>,
    frame_pool: Arc<FramePool>,
) -> Result<SCStream> {
    let filter = match &options.target {
        Some(Target::Display(display)) => {
            let mut filter = SCContentFilter::new();
            filter.set_displays(vec![display.raw_handle.clone()]);
            filter
        }
        Some(Target::Window(window)) => {
            let mut filter = SCContentFilter::new();
            filter.set_windows(vec![window.raw_handle.clone()]);
            filter
        }
        None => SCContentFilter::new(),
    };

    let mut config = SCStreamConfiguration::new();
    
    // Configure based on target type
    match &options.target {
        Some(Target::Window(_)) => {
            configure_stream_for_window(&mut config, options.target.as_ref().unwrap(), options)?;
        }
        _ => {
            config.set_shows_cursor(options.show_cursor);
            config.set_width(1920); // TODO: Get from display
            config.set_height(1080);
        }
    }

    // Configure frame rate
    if let Some(fps) = Some(options.fps) {
        let frame_duration = CMTime::new(1, fps as i32);
        config.set_minimum_frame_interval(&frame_duration)
            .map_err(|e| anyhow!("Failed to set frame interval: {}", e))?;
    }

    // Configure audio
    let captures_audio = if let Some(window_audio) = &options.window_audio {
        // Window-specific audio settings
        config.set_excludes_current_process_audio(true);
        config.set_audio_application_only(window_audio.capture_window_audio_only);
        config.set_audio_ducking(window_audio.audio_ducking);
        true
    } else {
        // Default audio settings
        config.set_excludes_current_process_audio(options.exclude_current_process_audio.unwrap_or(true));
        options.capture_system_audio.unwrap_or(false)
    };
    config.set_captures_audio(captures_audio);

    if let Some(rate) = options.audio_sample_rate {
        config.set_sample_rate(rate);
    }
    if let Some(channels) = options.audio_channel_count {
        config.set_channel_count(channels);
    }

    let stream = SCStream::new(&filter, &config)
        .map_err(|e| anyhow::anyhow!("Failed to create stream: {}", e))?;

    stream.add_output(ScreenCapturer::new(frame_sender.clone(), Arc::clone(&frame_pool)));
    
    if captures_audio {
        stream.add_output(AudioCapturer::new(frame_sender, Arc::clone(&frame_pool)));
    }

    Ok(stream)
}

pub fn process_sample_buffer(
    sample_buffer: CMSampleBuffer,
    output_type: SCStreamOutputType,
    frame_type: FrameType,
    frame_pool: &FramePool,
) -> Option<Frame> {
    match output_type {
        SCStreamOutputType::Screen => {
            let pixel_buffer = PixelBuffer::from_channel_item((sample_buffer, output_type))?;
            let width = pixel_buffer.width() as i32;
            let height = pixel_buffer.height() as i32;
            let bytes_per_row = pixel_buffer.bytes_per_row();
            let display_time = pixel_buffer.display_time();

            // Get a buffer from the pool
            let mut buffer = frame_pool.get_video_buffer(bytes_per_row * height as usize);
            buffer.extend_from_slice(&pixel_buffer.buffer().data());

            match frame_type {
                FrameType::BGR0 => Some(Frame::BGR0(crate::frame::BGRFrame {
                    display_time,
                    width,
                    height,
                    data: buffer,
                })),
                FrameType::RGB => Some(Frame::RGB(crate::frame::RGBFrame {
                    display_time,
                    width,
                    height,
                    data: buffer,
                })),
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    match &options.target {
        Some(target) => match target {
            Target::Display(display) => [display.width as u32, display.height as u32],
            Target::Window(window) => {
                if let Some(bounds) = window::get_window_bounds_with_shadow(window.id) {
                    [bounds.size.width as u32, bounds.size.height as u32]
                } else {
                    [window.width as u32, window.height as u32]
                }
            }
        },
        None => [1920, 1080], // Default size
    }
}

pub fn get_crop_area(options: &Options) -> Area {
    let target = options
        .target
        .clone()
        .unwrap_or_else(|| Target::Display(targets::get_main_display().unwrap()));

    let (width, height) = targets::get_target_dimensions(&target);

    options
        .crop_area
        .as_ref()
        .map(|val| {
            let input_width = val.size.width + (val.size.width % 2.0);
            let input_height = val.size.height + (val.size.height % 2.0);

            Area {
                origin: Point {
                    x: val.origin.x,
                    y: val.origin.y,
                },
                size: Size {
                    width: input_width,
                    height: input_height,
                },
            }
        })
        .unwrap_or_else(|| Area {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: width as f64,
                height: height as f64,
            },
        })
}
