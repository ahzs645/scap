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

/// Main capturer struct - simplified for sync operation
pub struct Capturer {
    options: Options,
    frame_pool: Arc<FramePool>,
    frame_receiver: Option<std::sync::mpsc::Receiver<Result<Frame>>>,
    is_capturing: bool,
    
    #[cfg(target_os = "macos")]
    mac_engine: Option<engine::mac::ScreenCapturer>,
    
    #[cfg(target_os = "windows")]
    win_engine: Option<engine::win::WCStream>,
    
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    linux_engine: Option<engine::linux::LinuxCapturer>,
}

impl Capturer {
    /// Build a new capturer with the given options
    pub fn build(options: Options) -> Result<Self> {
        let frame_pool = Arc::new(FramePool::new());
        
        Ok(Self {
            options,
            frame_pool,
            frame_receiver: None,
            is_capturing: false,
            
            #[cfg(target_os = "macos")]
            mac_engine: None,
            
            #[cfg(target_os = "windows")]
            win_engine: None,
            
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            linux_engine: None,
        })
    }

    /// Start capture (sync version)
    pub fn start_capture(&mut self) -> Result<()> {
        // Create a sync channel for frame communication
        let (frame_tx, frame_rx) = std::sync::mpsc::channel();
        self.frame_receiver = Some(frame_rx);
        
        // Platform-specific engine creation and startup
        #[cfg(target_os = "macos")]
        {
            // Create async sender that bridges to sync channel
            let (async_sender, mut async_receiver) = async_frame::create_channel();
            
            // Create the Mac engine
            let mut mac_capturer = engine::mac::ScreenCapturer::new(
                async_sender,
                Arc::clone(&self.frame_pool)
            );
            
            // Spawn a thread to bridge async to sync
            let sync_sender = frame_tx.clone();
            std::thread::spawn(move || {
                // Simple runtime for handling async frames
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create runtime");
                
                rt.block_on(async {
                    while let Ok(frame) = async_receiver.recv().await {
                        if sync_sender.send(Ok(frame)).is_err() {
                            break;
                        }
                    }
                });
            });
            
            // Create another runtime for the capture operation
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            
            // Start capture in the runtime
            rt.block_on(async {
                mac_capturer.start_capture(&self.options).await?;
                Ok::<_, anyhow::Error>(())
            })?;
            
            self.mac_engine = Some(mac_capturer);
        }
        
        #[cfg(target_os = "windows")]
        {
            // Convert sync channel to async sender for Windows engine
            let sync_sender = frame_tx.clone();
            let (async_sender, _) = async_frame::create_channel();
            
            // Bridge sync channel to async sender  
            std::thread::spawn(move || {
                // Windows implementation would send frames here
            });
            
            let mut win_capturer = engine::win::create_capturer(&self.options, async_sender)?;
            win_capturer.start_capture();
            self.win_engine = Some(win_capturer);
        }
        
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            let (async_sender, _) = async_frame::create_channel();
            let mut linux_capturer = engine::linux::create_capturer(&self.options, async_sender)?;
            linux_capturer.start_capture();
            self.linux_engine = Some(linux_capturer);
        }
        
        self.is_capturing = true;
        Ok(())
    }

    /// Stop capture (sync version)
    pub fn stop_capture(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        if let Some(mut engine) = self.mac_engine.take() {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            
            rt.block_on(async {
                engine.stop_capture().await?;
                Ok::<_, anyhow::Error>(())
            })?;
        }
        
        #[cfg(target_os = "windows")]
        if let Some(engine) = self.win_engine.take() {
            // ... existing Windows code ...
        }
        
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if let Some(engine) = self.linux_engine.take() {
            // ... existing Linux code ...
        }
        
        self.is_capturing = false;
        self.frame_receiver = None;
        Ok(())
    }

    /// Get next frame (sync version)
    pub fn get_next_frame(&mut self) -> Result<Frame> {
        if let Some(ref receiver) = self.frame_receiver {
            match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(frame_result) => frame_result,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(anyhow::anyhow!("Frame receive timeout"))
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    Err(anyhow::anyhow!("Frame channel disconnected"))
                }
            }
        } else {
            Err(anyhow::anyhow!("Capturer not started"))
        }
    }

    /// Get output frame size
    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        engine::get_output_frame_size(&self.options)
    }

    // Legacy method names for compatibility
    pub fn start_capture_sync(&mut self) -> Result<()> {
        self.start_capture()
    }

    pub fn stop_capture_sync(&mut self) -> Result<()> {
        self.stop_capture()
    }

    pub fn get_next_frame_sync(&mut self) -> Result<Frame> {
        self.get_next_frame()
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