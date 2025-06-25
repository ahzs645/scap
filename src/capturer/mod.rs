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

pub mod async_frame;
pub mod engine;
pub mod frame_pool;
pub mod error_recovery;

pub use engine::Engine;
use async_frame::{AsyncFrameReceiver, AsyncFrameSender, CaptureState};
pub use frame_pool::FramePool;
pub use error_recovery::{ErrorRecovery, ErrorRecoveryConfig};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolution {
    Native,
    Custom(Size),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub window_frame_padding: Option<f32>,

    /// Whether to match window resolution
    pub match_window_resolution: Option<bool>,

    /// Whether to include window shadow
    pub include_window_shadow: Option<bool>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            target: None,
            fps: 30,
            show_cursor: true,
            show_highlight: false,
            excluded_targets: None,
            output_type: FrameType::BGRAFrame,
            output_resolution: Resolution::Native,
            crop_area: None,
            capture_system_audio: None,
            exclude_current_process_audio: None,
            audio_sample_rate: None,
            audio_channel_count: None,
            capture_microphone: None,
            microphone_device_id: None,
            window_audio: None,
            exclude_overlapping_windows: None,
            window_frame_padding: None,
            match_window_resolution: None,
            include_window_shadow: None,
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
        Self::build(options)
    }

    /// Build a new [Capturer] instance with the provided options
    pub fn build(options: Options) -> Result<Self> {
        let frame_pool = Arc::new(FramePool::new(10));
        let (frame_sender, frame_receiver) = AsyncFrameReceiver::new(32);
        let state = Arc::new(Mutex::new(CaptureState::Stopped));
        
        let error_recovery = ErrorRecovery::new(Arc::clone(&state), Some(ErrorRecoveryConfig::default()));
        
        let engine = Engine::new(options, frame_sender, Arc::clone(&frame_pool))?;
        
        Ok(Self {
            engine,
            frame_receiver,
            state,
            frame_pool,
            error_recovery,
        })
    }

    /// Start capturing the frames
    pub async fn start_capture(&mut self) -> Result<()> {
        let mut state = self.state.lock().await;
        if *state == CaptureState::Running {
            return Ok(());
        }
        
        self.engine.start_capture().await?;
        *state = CaptureState::Running;
        Ok(())
    }

    /// Stop the capturer
    pub async fn stop_capture(&mut self) -> Result<()> {
        let mut state = self.state.lock().await;
        if *state == CaptureState::Stopped {
            return Ok(());
        }
        
        self.engine.stop_capture().await?;
        *state = CaptureState::Stopped;
        Ok(())
    }

    /// Get the next captured frame
    pub async fn get_next_frame(&mut self) -> Result<Frame> {
        match self.state {
            CaptureState::Running => {
                match self.frame_receiver.try_recv() {
                    Ok(frame) => Ok(frame),
                    Err(_) => {
                        // Use Box::pin for recursive async call
                        Box::pin(self.get_next_frame()).await
                    }
                }
            }
            CaptureState::Stopped => Err(anyhow!("Capture is stopped")),
            CaptureState::Error(ref e) => Err(anyhow!("Capture error: {}", e)),
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
