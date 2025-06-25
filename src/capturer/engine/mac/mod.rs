use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::{cmp, sync::Arc};

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
use core_media_rs::cm_sample_buffer::CMSampleBuffer;


use crate::frame::{Frame, FrameType};
use crate::targets::Target;
use crate::{
    capturer::{Area, Options, Point, Resolution, Size},
    targets,
    async_frame::AsyncFrameSender,
    frame_pool::FramePool,
};

use super::ChannelItem;

mod apple_sys;
mod pixel_buffer;
mod pixelformat;
mod audio_buffer;
mod cg_fallback;

pub use pixel_buffer::PixelBuffer;

pub struct ScreenCapturer {
    frame_sender: AsyncFrameSender,
    frame_pool: Arc<FramePool>,
}

impl ScreenCapturer {
    pub fn new(frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Self {
        ScreenCapturer { frame_sender, frame_pool }
    }
}

impl SCStreamOutputTrait for ScreenCapturer {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, of_type: SCStreamOutputType) {
        let frame = process_sample_buffer(sample.clone(), of_type, FrameType::BGR0, &self.frame_pool);
        if let Some(frame) = frame {
            self.frame_sender.send_frame(Ok(frame)).unwrap_or(());
        }
    }
}

pub struct AudioCapturer {
    frame_sender: AsyncFrameSender,
    frame_pool: Arc<FramePool>,
}

impl AudioCapturer {
    pub fn new(frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Self {
        AudioCapturer { frame_sender, frame_pool }
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
) -> anyhow::Result<SCStream> {
    let filter = match &options.target {
        Some(target) => match target {
            Target::Display(display) => {
                SCContentFilter::display(display.clone())
                    .map_err(|e| anyhow::anyhow!("Failed to create display filter: {}", e))?
            }
            Target::Window(window) => {
                SCContentFilter::window(window.clone())
                    .map_err(|e| anyhow::anyhow!("Failed to create window filter: {}", e))?
            }
        },
        None => SCContentFilter::display_default()
            .map_err(|e| anyhow::anyhow!("Failed to create default display filter: {}", e))?,
    };

    let mut config = SCStreamConfiguration::new();
    config.set_shows_cursor(options.show_cursor);
    config.set_width(1920); // TODO: Get from display
    config.set_height(1080);
    config.set_captures_audio(true);

    let stream = SCStream::new(filter, config)
        .map_err(|e| anyhow::anyhow!("Failed to create stream: {}", e))?;

    stream.add_output(ScreenCapturer::new(frame_sender.clone(), Arc::clone(&frame_pool)));
    stream.add_output(AudioCapturer::new(frame_sender, Arc::clone(&frame_pool)));

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

pub fn process_audio_buffer(
    sample_buffer: CMSampleBuffer,
    frame_pool: &FramePool,
) -> anyhow::Result<crate::frame::AudioFrame> {
    audio_buffer::process_audio_sample_buffer_enhanced(sample_buffer, frame_pool)
}

pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    match &options.target {
        Some(target) => match target {
            Target::Display(display) => [display.width as u32, display.height as u32],
            Target::Window(window) => [window.width as u32, window.height as u32],
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
