// src/napi.rs - Node.js bindings for scap
#![cfg(feature = "napi")]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::{Arc, Mutex};
use crate::{
    capturer::{Capturer, Options, Point, Size, Area, Resolution},
    frame::{Frame, FrameType},
    targets::Target,
    get_all_targets, has_permission, request_permission, is_supported,
};

#[napi(object)]
pub struct ScreenSource {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_display: bool,
}

#[napi(object)]
pub struct RecordingOptions {
    pub fps: Option<u32>,
    pub show_cursor: Option<bool>,
    pub show_highlight: Option<bool>,
    pub output_type: Option<String>, // "BGRA", "RGB", "YUV", "BGR0"
    pub output_resolution: Option<String>, // "720p", "1080p", "captured"
    pub crop_area: Option<CropArea>,
    pub excluded_targets: Option<Vec<String>>,
    pub capture_system_audio: Option<bool>,
    pub exclude_current_process_audio: Option<bool>,
    pub audio_sample_rate: Option<u32>,
    pub audio_channel_count: Option<u32>,
    pub capture_microphone: Option<bool>,
    pub microphone_device_id: Option<String>,
}

#[napi(object)]
pub struct CropArea {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[napi(object)]
pub struct FrameData {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
    pub display_time: f64,
    pub frame_type: String,
}

#[napi(object)]
pub struct AudioData {
    pub sample_rate: u32,
    pub channel_count: u32,
    pub sample_count: u32,
    pub data: Vec<u8>,
    pub display_time: f64,
    pub bits_per_sample: u32,
    pub source: String, // "system" or "microphone"
}

/// Main screen capture class for Node.js
#[napi]
pub struct ScreenCapture {
    capturer: Option<Arc<Mutex<Capturer>>>,
}

#[napi]
impl ScreenCapture {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        Ok(Self {
            capturer: None,
        })
    }

    /// Check if screen capture is supported on this platform
    #[napi]
    pub fn is_supported() -> bool {
        is_supported()
    }

    /// Check if we have screen recording permission
    #[napi]
    pub fn has_permission() -> bool {
        has_permission()
    }

    /// Request screen recording permission
    #[napi]
    pub fn request_permission() -> bool {
        request_permission()
    }

    /// Get all available screen capture targets
    #[napi]
    pub fn get_all_targets() -> Result<Vec<ScreenSource>> {
        let targets = get_all_targets().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Failed to get targets: {}", e))
        })?;

        Ok(targets.into_iter().map(|target| {
            match target {
                Target::Display(display) => ScreenSource {
                    id: format!("display:{}", display.id),
                    name: display.title,
                    width: display.width as u32,
                    height: display.height as u32,
                    is_display: true,
                },
                Target::Window(window) => ScreenSource {
                    id: format!("window:{}", window.id),
                    name: window.title,
                    width: window.width as u32,
                    height: window.height as u32,
                    is_display: false,
                },
            }
        }).collect())
    }

    /// Create a capturer with the given options
    #[napi]
    pub fn create_capturer(&mut self, options: Option<RecordingOptions>) -> Result<()> {
        let opts = self.convert_options(options)?;
        
        let capturer = Capturer::build(opts).map_err(|e| {
            Error::new(Status::GenericFailure, format!("Failed to create capturer: {}", e))
        })?;

        self.capturer = Some(Arc::new(Mutex::new(capturer)));
        Ok(())
    }

    /// Start screen capture
    #[napi]
    pub fn start_capture(&self) -> Result<()> {
        if let Some(ref capturer) = self.capturer {
            let mut capturer = capturer.lock().map_err(|_| {
                Error::new(Status::GenericFailure, "Failed to lock capturer")
            })?;
            
            // Use sync version to avoid async complications in NAPI
            capturer.start_capture_sync().map_err(|e| {
                Error::new(Status::GenericFailure, format!("Failed to start capture: {}", e))
            })?;
            Ok(())
        } else {
            Err(Error::new(Status::GenericFailure, "Capturer not created"))
        }
    }

    /// Stop screen capture
    #[napi]
    pub fn stop_capture(&self) -> Result<()> {
        if let Some(ref capturer) = self.capturer {
            let mut capturer = capturer.lock().map_err(|_| {
                Error::new(Status::GenericFailure, "Failed to lock capturer")
            })?;
            
            // Use sync version to avoid async complications in NAPI
            capturer.stop_capture_sync().map_err(|e| {
                Error::new(Status::GenericFailure, format!("Failed to stop capture: {}", e))
            })?;
            Ok(())
        } else {
            Err(Error::new(Status::GenericFailure, "Capturer not created"))
        }
    }

    /// Get the next frame (blocking)
    #[napi]
    pub fn get_next_frame(&self) -> Result<FrameData> {
        if let Some(ref capturer) = self.capturer {
            let mut capturer = capturer.lock().map_err(|_| {
                Error::new(Status::GenericFailure, "Failed to lock capturer")
            })?;
            
            // Use sync version to avoid async complications in NAPI
            let frame = capturer.get_next_frame_sync().map_err(|e| {
                Error::new(Status::GenericFailure, format!("Failed to get frame: {}", e))
            })?;

            Ok(self.convert_frame(frame))
        } else {
            Err(Error::new(Status::GenericFailure, "Capturer not created"))
        }
    }

    /// Get output frame size
    #[napi]
    pub fn get_output_frame_size(&self) -> Result<Vec<u32>> {
        if let Some(ref capturer) = self.capturer {
            let mut capturer = capturer.lock().map_err(|_| {
                Error::new(Status::GenericFailure, "Failed to lock capturer")
            })?;
            
            let [width, height] = capturer.get_output_frame_size();
            Ok(vec![width, height])
        } else {
            Err(Error::new(Status::GenericFailure, "Capturer not created"))
        }
    }

    // Helper methods
    fn convert_options(&self, options: Option<RecordingOptions>) -> Result<Options> {
        let opts = options.unwrap_or(RecordingOptions {
            fps: Some(30),
            show_cursor: Some(true),
            show_highlight: Some(false),
            output_type: Some("BGRA".to_string()),
            output_resolution: Some("captured".to_string()),
            crop_area: None,
            excluded_targets: None,
            capture_system_audio: Some(false),
            exclude_current_process_audio: Some(true),
            audio_sample_rate: Some(48000),
            audio_channel_count: Some(2),
            capture_microphone: Some(false),
            microphone_device_id: None,
        });

        let output_type = match opts.output_type.as_deref() {
            Some("BGRA") => FrameType::BGRAFrame,
            Some("RGB") => FrameType::RGB,
            Some("BGR0") => FrameType::BGR0,
            Some("YUV") => FrameType::YUVFrame,
            _ => FrameType::BGRAFrame,
        };

        let output_resolution = match opts.output_resolution.as_deref() {
            Some("480p") => Resolution::_480p,
            Some("720p") => Resolution::_720p,
            Some("1080p") => Resolution::_1080p,
            Some("1440p") => Resolution::_1440p,
            Some("2160p") => Resolution::_2160p,
            Some("4320p") => Resolution::_4320p,
            _ => Resolution::Captured,
        };

        let crop_area = opts.crop_area.map(|area| Area {
            origin: Point { x: area.x, y: area.y },
            size: Size { width: area.width, height: area.height },
        });

        Ok(Options {
            fps: opts.fps.unwrap_or(30),
            show_cursor: opts.show_cursor.unwrap_or(true),
            show_highlight: opts.show_highlight.unwrap_or(false),
            target: None, // Would need to be set based on target selection
            crop_area,
            output_type,
            output_resolution,
            excluded_targets: None, // Would need conversion from Vec<String>
            capture_system_audio: opts.capture_system_audio,
            exclude_current_process_audio: opts.exclude_current_process_audio,
            audio_sample_rate: opts.audio_sample_rate,
            audio_channel_count: opts.audio_channel_count,
            capture_microphone: opts.capture_microphone,
            microphone_device_id: opts.microphone_device_id,
            window_audio: None,
            exclude_overlapping_windows: None,
            window_frame_padding: None,
            match_window_resolution: None,
            include_window_shadow: None,
        })
    }

    fn convert_frame(&self, frame: Frame) -> FrameData {
        match frame {
            Frame::BGRA(f) => FrameData {
                width: f.width,
                height: f.height,
                data: f.data,
                display_time: f.display_time as f64 / 1_000_000_000.0, // Convert to seconds
                frame_type: "BGRA".to_string(),
            },
            Frame::RGB(f) => FrameData {
                width: f.width,
                height: f.height,
                data: f.data,
                display_time: f.display_time as f64 / 1_000_000_000.0,
                frame_type: "RGB".to_string(),
            },
            Frame::BGR0(f) => FrameData {
                width: f.width,
                height: f.height,
                data: f.data,
                display_time: f.display_time as f64 / 1_000_000_000.0,
                frame_type: "BGR0".to_string(),
            },
            Frame::YUVFrame(f) => FrameData {
                width: f.width,
                height: f.height,
                data: f.luminance_bytes, // Just return luminance for now
                display_time: f.display_time as f64 / 1_000_000_000.0,
                frame_type: "YUV".to_string(),
            },
            Frame::SystemAudio(_) | Frame::MicrophoneAudio(_) => FrameData {
                width: 0,
                height: 0,
                data: vec![],
                display_time: 0.0,
                frame_type: "Audio".to_string(),
            },
        }
    }

    fn convert_audio_frame(&self, frame: crate::frame::AudioFrame) -> AudioData {
        AudioData {
            sample_rate: frame.sample_rate,
            channel_count: frame.channel_count,
            sample_count: frame.sample_count,
            data: frame.data,
            display_time: frame.display_time as f64 / 1_000_000_000.0,
            bits_per_sample: frame.bits_per_sample,
            source: match frame.source {
                crate::frame::AudioSource::System => "system".to_string(),
                crate::frame::AudioSource::Microphone => "microphone".to_string(),
            },
        }
    }
}

// Utility functions
#[napi]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[napi]
pub fn get_supported_platforms() -> Vec<String> {
    vec![
        "windows".to_string(),
        "macos".to_string(), 
        "linux".to_string(),
    ]
}