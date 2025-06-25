pub mod engine;

use std::{error::Error, sync::mpsc};
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::Mutex;

use anyhow::anyhow;

use engine::ChannelItem;

use crate::{
    frame::{Frame, FrameType},
    has_permission, is_supported,
    targets::Target,
};

pub use engine::get_output_frame_size;

mod async_frame;
mod engine;
mod frame_pool;
mod error_recovery;

pub use engine::Engine;
use async_frame::{AsyncFrameReceiver, AsyncFrameSender, CaptureState};
pub use frame_pool::FramePool;
pub use error_recovery::{ErrorRecovery, ErrorRecoveryConfig};

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
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Default, Clone)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Default, Clone)]
pub struct Area {
    pub origin: Point,
    pub size: Size,
}

#[derive(Debug, Clone)]
pub struct WindowAudioOptions {
    pub capture_window_audio_only: bool,
    pub include_system_notifications: bool,
    pub audio_ducking: bool,
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

    /// Window-specific options
    pub window_audio: Option<WindowAudioOptions>,

    /// Whether to exclude overlapping windows
    pub exclude_overlapping_windows: Option<bool>,

    /// Window frame padding
    pub window_frame_padding: Option<f64>,

    /// Whether to match window resolution
    pub match_window_resolution: Option<bool>,

    /// Whether to include window shadow
    pub include_window_shadow: Option<bool>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            target: None,
            output_type: FrameType::BGRAFrame,
            output_resolution: Resolution::Captured,
            show_cursor: true,
            show_highlight: false,
            crop_area: None,
            fps: 30,
            excluded_targets: None,
            capture_system_audio: None,
            exclude_current_process_audio: None,
            audio_sample_rate: None,
            audio_channel_count: None,
            capture_microphone: None,
            microphone_device_id: None,
            window_audio: None,
            exclude_overlapping_windows: Some(false),
            window_frame_padding: None,
            match_window_resolution: Some(true),
            include_window_shadow: Some(true),
        }
    }
}

/// Screen capturer class
pub struct Capturer {
    engine: Engine,
    frame_receiver: AsyncFrameReceiver,
    state: Arc<Mutex<CaptureState>>,
    frame_pool: Arc<FramePool>,
    error_recovery: ErrorRecovery,
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
        let (frame_receiver, frame_sender) = AsyncFrameReceiver::new(32); // Buffer size of 32 frames
        let state = Arc::new(Mutex::new(CaptureState::Idle));
        
        let engine = Engine::new(options, frame_sender)?;
        
        Ok(Self {
            engine,
            frame_receiver,
            state: state.clone(),
        })
    }

    /// Build a new [Capturer] instance with the provided options
    pub fn build(options: Options) -> Result<Self> {
        let (frame_receiver, frame_sender) = AsyncFrameReceiver::new(32); // Buffer size of 32 frames
        let state = Arc::new(Mutex::new(CaptureState::Idle));
        let frame_pool = Arc::new(FramePool::new(10)); // Pool size of 10 buffers
        let error_recovery = ErrorRecovery::new(Arc::clone(&state), None);
        
        let engine = Engine::new(options, frame_sender, Arc::clone(&frame_pool))?;
        
        Ok(Self {
            engine,
            frame_receiver,
            state: Arc::clone(&state),
            frame_pool,
            error_recovery,
        })
    }

    /// Start capturing the frames
    pub async fn start_capture(&mut self) {
        {
            let mut state = self.state.lock().await;
            *state = CaptureState::Starting;
        }
        
        if let Err(e) = self.engine.start_capture().await {
            log::error!("Failed to start capture: {}", e);
            if !self.error_recovery.handle_error(&e.to_string()).await {
                return;
            }
        }
        
        {
            let mut state = self.state.lock().await;
            *state = CaptureState::Running;
        }
    }

    /// Stop the capturer
    pub async fn stop_capture(&mut self) {
        {
            let mut state = self.state.lock().await;
            *state = CaptureState::Stopping;
        }
        
        if let Err(e) = self.engine.stop_capture().await {
            log::error!("Failed to stop capture: {}", e);
        }
        
        {
            let mut state = self.state.lock().await;
            *state = CaptureState::Idle;
        }
    }

    /// Get the next captured frame
    pub async fn get_next_frame(&mut self) -> Result<Frame> {
        match self.frame_receiver.next_frame().await {
            Some(frame) => {
                match &frame {
                    Ok(frame) => {
                        // Return buffers to pool after frame is processed
                        match frame {
                            Frame::RGB(f) | Frame::BGR0(f) | Frame::BGRA(f) => {
                                self.frame_pool.return_video_buffer(f.data.clone());
                            }
                            Frame::SystemAudio(f) | Frame::MicrophoneAudio(f) => {
                                self.frame_pool.return_audio_buffer(f.data.clone());
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        if !self.error_recovery.handle_error(&e.to_string()).await {
                            return Err(anyhow::anyhow!("Max retry attempts reached"));
                        }
                    }
                }
                frame
            }
            None => Err(anyhow::anyhow!("Frame channel closed")),
        }
    }

    /// Get the dimensions the frames will be captured in
    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        self.engine.get_output_frame_size()
    }

    pub fn raw(&self) -> RawCapturer {
        RawCapturer { capturer: self }
    }

    pub async fn get_state(&self) -> CaptureState {
        self.state.lock().await.clone()
    }
}

pub struct RawCapturer<'a> {
    capturer: &'a Capturer,
}

impl RawCapturer<'_> {
    #[cfg(target_os = "macos")]
    pub fn get_next_pixel_buffer(&self) -> Result<PixelBuffer, Box<dyn std::error::Error>> {
        use std::time::Duration;

        let capturer = &self.capturer;

        loop {
            let error_flag = capturer
                .engine
                .error_flag
                .load(std::sync::atomic::Ordering::Relaxed);
            if error_flag {
                return Err("Capture error occurred".into());
            }

            let res = match capturer.rx.recv_timeout(Duration::from_millis(10)) {
                Ok(v) => v,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err("Channel disconnected".into()),
            };

            if let Some(frame) = PixelBuffer::from_channel_item(res?) {
                return Ok(frame);
            }
        }
    }
}
