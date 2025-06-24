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
use core_foundation::error::CFError;

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

pub use pixel_buffer::PixelBuffer;

pub struct ScreenCapturer {
    pub tx: mpsc::Sender<ChannelItem>,
}

impl ScreenCapturer {
    pub fn new(tx: mpsc::Sender<ChannelItem>) -> Self {
        ScreenCapturer { tx }
    }
}

impl SCStreamOutputTrait for ScreenCapturer {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, of_type: SCStreamOutputType) {
        self.tx.send((sample, of_type)).unwrap_or(());
    }
}

pub struct AudioCapturer {
    pub tx: mpsc::Sender<ChannelItem>,
}

impl AudioCapturer {
    pub fn new(tx: mpsc::Sender<ChannelItem>) -> Self {
        AudioCapturer { tx }
    }
}

impl SCStreamOutputTrait for AudioCapturer {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, of_type: SCStreamOutputType) {
        self.tx.send((sample, of_type)).unwrap_or(());
    }
}

pub fn create_capturer(
    options: &Options,
    tx: mpsc::Sender<ChannelItem>,
    _error_flag: Arc<AtomicBool>,
) -> Result<SCStream, CFError> {
    // If no target is specified, capture the main display
    let target = options
        .target
        .clone()
        .unwrap_or_else(|| Target::Display(targets::get_main_display().unwrap()));

    let sc_shareable_content = SCShareableContent::get()?;

    let filter = match target {
        Target::Window(window) => {
            // Get SCWindow from window id
            let sc_window = sc_shareable_content
                .windows()
                .into_iter()
                .find(|sc_win| sc_win.window_id() == window.id)
                .unwrap();

            SCContentFilter::new().with_desktop_independent_window(&sc_window)
        }
        Target::Display(display) => {
            // Get SCDisplay from display id
            let sc_display = sc_shareable_content
                .displays()
                .into_iter()
                .find(|sc_dis| sc_dis.display_id() == display.id)
                .unwrap();

            match &options.excluded_targets {
                None => SCContentFilter::new().with_display_excluding_windows(&sc_display, &[]),
                Some(excluded_targets) => {
                    let excluded_windows: Vec<_> = sc_shareable_content
                        .windows()
                        .into_iter()
                        .filter(|window| {
                            excluded_targets
                                .iter()
                                .any(|excluded_target| match excluded_target {
                                    Target::Window(excluded_window) => {
                                        excluded_window.id == window.window_id()
                                    }
                                    _ => false,
                                })
                        })
                        .collect();

                    let excluded_refs: Vec<&_> = excluded_windows.iter().collect();
                    SCContentFilter::new().with_display_excluding_windows(&sc_display, &excluded_refs)
                }
            }
        }
    };

    let [width, height] = get_output_frame_size(options);

    let mut stream_config = SCStreamConfiguration::new()
        .set_width(width)?
        .set_height(height)?
        .set_shows_cursor(options.show_cursor)?
        .set_pixel_format(screencapturekit::stream::configuration::pixel_format::PixelFormat::BGRA)?;

    // Enable audio capture if requested (use minimal configuration like reference implementation)
    if options.capture_system_audio.unwrap_or(false) {
        stream_config = stream_config
            .set_captures_audio(true)?
            .set_excludes_current_process_audio(options.exclude_current_process_audio.unwrap_or(true))?;
        // Note: Not setting channel_count - let ScreenCaptureKit use optimal defaults
    }

    let mut stream = SCStream::new(&filter, &stream_config);
    stream.add_output_handler(ScreenCapturer::new(tx.clone()), SCStreamOutputType::Screen);
    
    // Add audio output if audio capture is enabled
    if options.capture_system_audio.unwrap_or(false) {
        stream.add_output_handler(AudioCapturer::new(tx), SCStreamOutputType::Audio);
    }

    Ok(stream)
}

pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    let target = options
        .target
        .clone()
        .unwrap_or_else(|| Target::Display(targets::get_main_display().unwrap()));

    let scale_factor = targets::get_scale_factor(&target);
    let source_rect = get_crop_area(options);

    // Calculate the output height & width based on the required resolution
    // Output width and height need to be multiplied by scale (or dpi)
    let mut output_width = (source_rect.size.width as u32) * (scale_factor as u32);
    let mut output_height = (source_rect.size.height as u32) * (scale_factor as u32);
    // 1200x800
    match options.output_resolution {
        Resolution::Captured => {}
        _ => {
            let [resolved_width, resolved_height] = options
                .output_resolution
                .value((source_rect.size.width as f32) / (source_rect.size.height as f32));
            // 1280 x 853
            output_width = cmp::min(output_width, resolved_width);
            output_height = cmp::min(output_height, resolved_height);
        }
    }

    output_width -= output_width % 2;
    output_height -= output_height % 2;

    [output_width, output_height]
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

pub fn process_sample_buffer(
    sample: CMSampleBuffer,
    of_type: SCStreamOutputType,
    output_type: FrameType,
) -> Option<Frame> {
    match of_type {
        SCStreamOutputType::Screen => {
            // With the new screencapturekit-rs, we don't need to check frame status
            // as the library handles this internally
            unsafe {
                return match output_type {
                    FrameType::YUVFrame => {
                        if let Some(yuvframe) = pixelformat::create_yuv_frame(sample) {
                            Some(Frame::YUVFrame(yuvframe))
                        } else {
                            None
                        }
                    }
                    FrameType::RGB => {
                        if let Some(rgbframe) = pixelformat::create_rgb_frame(sample) {
                            Some(Frame::RGB(rgbframe))
                        } else {
                            None
                        }
                    }
                    FrameType::BGR0 => {
                        if let Some(bgrframe) = pixelformat::create_bgr_frame(sample) {
                            Some(Frame::BGR0(bgrframe))
                        } else {
                            None
                        }
                    }
                    FrameType::BGRAFrame => {
                        if let Some(bgraframe) = pixelformat::create_bgra_frame(sample) {
                            Some(Frame::BGRA(bgraframe))
                        } else {
                            None
                        }
                    }
                };
            }
        }
        SCStreamOutputType::Audio => {
            // Process audio sample buffer using simplified approach
            if let Ok(audio_frame) = audio_buffer::process_audio_sample_buffer_simple(sample) {
                return Some(Frame::SystemAudio(audio_frame));
            }
        }
    }

    None
}
