pub mod engine;

use std::{error::Error, sync::mpsc};

use anyhow::anyhow;

use engine::ChannelItem;

use crate::{
    frame::{Frame, FrameType},
    has_permission, is_supported,
    targets::Target,
};

pub use engine::get_output_frame_size;

#[derive(Debug, Clone, Copy, Default)]
pub enum Resolution {
    _480p,
    _720p,
    _1080p,
    _1440p,
    _2160p,
    _4320p,

    #[default]
    Captured,
}

impl Resolution {
    fn value(&self, aspect_ratio: f32) -> [u32; 2] {
        match *self {
            Resolution::_480p => [640, (640_f32 / aspect_ratio).floor() as u32],
            Resolution::_720p => [1280, (1280_f32 / aspect_ratio).floor() as u32],
            Resolution::_1080p => [1920, (1920_f32 / aspect_ratio).floor() as u32],
            Resolution::_1440p => [2560, (2560_f32 / aspect_ratio).floor() as u32],
            Resolution::_2160p => [3840, (3840_f32 / aspect_ratio).floor() as u32],
            Resolution::_4320p => [7680, (7680_f32 / aspect_ratio).floor() as u32],
            Resolution::Captured => {
                panic!(".value should not be called when Resolution type is Captured")
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Default, Clone)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}
#[derive(Debug, Default, Clone)]
pub struct Area {
    pub origin: Point,
    pub size: Size,
}

/// Options passed to the screen capturer
#[derive(Debug, Clone)]
pub struct Options {
    /// The target to capture
    pub target: Option<Target>,

    /// The output type for the captured frames
    pub output_type: FrameType,

    /// The output resolution for the captured frames
    pub output_resolution: Resolution,

    /// Whether to show the cursor in the captured frames
    pub show_cursor: bool,

    /// Whether to show highlight clicks in the captured frames
    pub show_highlight: bool,

    /// The area to crop from the captured frames
    pub crop_area: Option<Area>,

    /// The FPS for the captured frames
    pub fps: u32,

    /// Targets to exclude from the capture
    pub excluded_targets: Option<Vec<Target>>,

    /// Whether to capture system audio (macOS only)
    pub capture_system_audio: Option<bool>,

    /// Whether to exclude current process audio from system audio capture (macOS only)
    pub exclude_current_process_audio: Option<bool>,

    /// Audio sample rate for system audio capture (macOS only)
    pub audio_sample_rate: Option<u32>,

    /// Audio channel count for system audio capture (macOS only)
    pub audio_channel_count: Option<u32>,

    /// Whether to capture microphone audio
    pub capture_microphone: Option<bool>,

    /// Microphone device ID (optional, uses default if None)
    pub microphone_device_id: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            target: None,
            output_type: FrameType::BGRAFrame,
            output_resolution: Resolution::_720p,
            show_cursor: true,
            show_highlight: false,
            crop_area: None,
            fps: 30,
            excluded_targets: None,
            capture_system_audio: Some(false),
            exclude_current_process_audio: Some(true),
            audio_sample_rate: Some(48000),
            audio_channel_count: Some(2),
            capture_microphone: Some(false),
            microphone_device_id: None,
        }
    }
}

/// Screen capturer class
pub struct Capturer {
    engine: engine::Engine,
    rx: mpsc::Receiver<anyhow::Result<ChannelItem>>,
}

#[derive(Debug)]
pub enum CapturerBuildError {
    NotSupported,
    PermissionNotGranted,
}

impl std::fmt::Display for CapturerBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapturerBuildError::NotSupported => write!(f, "Screen capturing is not supported"),
            CapturerBuildError::PermissionNotGranted => {
                write!(f, "Permission to capture the screen is not granted")
            }
        }
    }
}

impl Error for CapturerBuildError {}

impl Capturer {
    /// Create a new capturer instance with the provided options
    #[deprecated(
        since = "0.0.6",
        note = "Use `build` instead of `new` to create a new capturer instance."
    )]
    pub fn new(options: Options) -> anyhow::Result<Capturer> {
        let (tx, rx) = mpsc::channel();
        let engine = engine::Engine::new(&options, tx)?;

        Ok(Capturer { engine, rx })
    }

    /// Build a new [Capturer] instance with the provided options
    pub fn build(options: Options) -> anyhow::Result<Capturer> {
        if !is_supported() {
            return Err(anyhow!(CapturerBuildError::NotSupported));
        }

        if !has_permission() {
            return Err(anyhow!(CapturerBuildError::PermissionNotGranted));
        }

        let (tx, rx) = mpsc::channel();
        let engine = engine::Engine::new(&options, tx)?;

        Ok(Capturer { engine, rx })
    }

    // TODO
    // Prevent starting capture if already started
    /// Start capturing the frames
    pub fn start_capture(&mut self) {
        self.engine.start();
    }

    /// Stop the capturer
    pub fn stop_capture(&mut self) {
        self.engine.stop();
    }

    /// Get the next captured frame
    pub fn get_next_frame(&self) -> anyhow::Result<Frame> {
        loop {
            let res = self.rx.recv()??;

            if let Some(frame) = self.engine.process_channel_item(res) {
                return Ok(frame);
            }
        }
    }

    /// Get the dimensions the frames will be captured in
    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        self.engine.get_output_frame_size()
    }

    pub fn raw(&self) -> RawCapturer {
        RawCapturer { capturer: self }
    }
}

pub struct RawCapturer<'a> {
    capturer: &'a Capturer,
}
