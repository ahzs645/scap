use std::sync::Arc;
use anyhow::Result;

use crate::{
    frame::{Frame, FrameType},
    targets::Target,
};

// Submodules
pub mod async_frame;
pub mod frame_pool;
pub mod error_recovery;
pub mod engine;

// Re-exports
pub use async_frame::{AsyncFrameReceiver, AsyncFrameSender, CaptureState};
pub use frame_pool::FramePool;
pub use error_recovery::{ErrorRecovery, ErrorRecoveryConfig};

#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct Area {
    pub origin: Point,
    pub size: Size,
}

#[derive(Debug, Clone, Copy)]
pub enum Resolution {
    _480p,
    _720p,
    _1080p,
    _1440p,
    _2160p,
    _4320p,
    Captured,
}

impl Resolution {
    pub fn value(&self, _aspect_ratio: f32) -> [u32; 2] {
        match self {
            Resolution::_480p => [640, 480],
            Resolution::_720p => [1280, 720],
            Resolution::_1080p => [1920, 1080],
            Resolution::_1440p => [2560, 1440],
            Resolution::_2160p => [3840, 2160],
            Resolution::_4320p => [7680, 4320],
            Resolution::Captured => [1920, 1080], // Default fallback
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowAudioOptions {
    pub capture_window_audio_only: bool,
    pub include_system_notifications: bool,
    pub audio_ducking: bool,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub fps: u32,
    pub show_cursor: bool,
    pub show_highlight: bool,
    pub target: Option<Target>,
    pub crop_area: Option<Area>,
    pub output_type: FrameType,
    pub output_resolution: Resolution,
    pub excluded_targets: Option<Vec<Target>>,
    
    // Audio options
    pub capture_system_audio: Option<bool>,
    pub exclude_current_process_audio: Option<bool>,
    pub audio_sample_rate: Option<u32>,
    pub audio_channel_count: Option<u32>,
    pub capture_microphone: Option<bool>,
    pub microphone_device_id: Option<String>,
    pub window_audio: Option<WindowAudioOptions>,
    
    // Window-specific options
    pub exclude_overlapping_windows: Option<bool>,
    pub window_frame_padding: Option<f64>,
    pub match_window_resolution: Option<bool>,
    pub include_window_shadow: Option<bool>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            fps: 30,
            show_cursor: true,
            show_highlight: false,
            target: None,
            crop_area: None,
            output_type: FrameType::BGRAFrame,
            output_resolution: Resolution::Captured,
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
        }
    }
}

/// Main capturer struct
pub struct Capturer {
    options: Options,
    engine: Option<engine::Engine>,
    frame_pool: Arc<FramePool>,
    frame_receiver: Option<AsyncFrameReceiver>,
    is_capturing: bool,
}

impl Capturer {
    /// Build a new capturer with the given options
    pub fn build(options: Options) -> Result<Self> {
        let frame_pool = Arc::new(FramePool::new());
        
        Ok(Self {
            options,
            engine: None,
            frame_pool,
            frame_receiver: None,
            is_capturing: false,
        })
    }

    /// Start capture (sync version)
    pub fn start_capture_sync(&mut self) -> Result<()> {
        let (sender, receiver) = async_frame::create_channel();
        
        let engine = engine::Engine::new(
            self.options.clone(),
            sender,
            Arc::clone(&self.frame_pool),
        )?;
        
        self.engine = Some(engine);
        self.frame_receiver = Some(receiver);
        self.is_capturing = true;
        
        // Start the engine in sync mode
        if let Some(ref mut engine) = self.engine {
            // For sync operation, we need to handle async calls differently
            // This is a simplified approach
            tokio::runtime::Handle::try_current()
                .map(|handle| {
                    handle.block_on(async {
                        engine.start_capture().await
                    })
                })
                .unwrap_or_else(|_| {
                    // If no tokio runtime, create a simple runtime
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    rt.block_on(async {
                        engine.start_capture().await
                    })
                })?;
        }
        
        Ok(())
    }

    /// Stop capture (sync version)
    pub fn stop_capture_sync(&mut self) -> Result<()> {
        self.is_capturing = false;
        
        if let Some(ref mut engine) = self.engine {
            tokio::runtime::Handle::try_current()
                .map(|handle| {
                    handle.block_on(async {
                        engine.stop_capture().await
                    })
                })
                .unwrap_or_else(|_| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    rt.block_on(async {
                        engine.stop_capture().await
                    })
                })?;
        }
        
        Ok(())
    }

    /// Get next frame (sync version)
    pub fn get_next_frame_sync(&mut self) -> Result<Frame> {
        if let Some(ref mut receiver) = self.frame_receiver {
            match receiver.try_recv() {
                Ok(frame) => Ok(frame),
                Err(e) => Err(e),
            }
        } else {
            Err(anyhow::anyhow!("Capturer not started"))
        }
    }

    /// Get output frame size
    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        if let Some(ref mut engine) = self.engine {
            engine.get_output_frame_size()
        } else {
            engine::get_output_frame_size(&self.options)
        }
    }

    /// Async versions for compatibility
    pub async fn start_capture(&mut self) -> Result<()> {
        let (sender, receiver) = async_frame::create_channel();
        
        let mut engine = engine::Engine::new(
            self.options.clone(),
            sender,
            Arc::clone(&self.frame_pool),
        )?;
        
        engine.start_capture().await?;
        
        self.engine = Some(engine);
        self.frame_receiver = Some(receiver);
        self.is_capturing = true;
        
        Ok(())
    }

    pub async fn stop_capture(&mut self) -> Result<()> {
        self.is_capturing = false;
        
        if let Some(ref mut engine) = self.engine {
            engine.stop_capture().await?;
        }
        
        Ok(())
    }

    pub async fn get_next_frame(&mut self) -> Result<Frame> {
        if let Some(ref mut receiver) = self.frame_receiver {
            receiver.recv().await
        } else {
            Err(anyhow::anyhow!("Capturer not started"))
        }
    }
}

/// Raw capturer for accessing platform-specific features
pub struct RawCapturer<'a> {
    _capturer: &'a Capturer,
}

impl<'a> RawCapturer<'a> {
    pub fn new(capturer: &'a Capturer) -> Self {
        Self { _capturer: capturer }
    }
}