use std::sync::Arc;
use anyhow::Result;

use super::Options;
use crate::frame::Frame;
use crate::capturer::{async_frame::AsyncFrameSender, frame_pool::FramePool};

#[cfg(target_os = "macos")]
pub mod mac;

#[cfg(target_os = "windows")]
mod win;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux;

#[cfg(target_os = "macos")]
pub type ChannelItem = (
    core_media_rs::cm_sample_buffer::CMSampleBuffer,
    screencapturekit::stream::output_type::SCStreamOutputType,
);
#[cfg(not(target_os = "macos"))]
pub type ChannelItem = Frame;

pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    #[cfg(target_os = "macos")]
    {
        // Use a simple default for now to avoid compilation issues
        [1920, 1080]
    }

    #[cfg(target_os = "windows")]
    {
        win::get_output_frame_size(options)
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        // TODO: How to calculate this on Linux?
        [1920, 1080]
    }
}

pub struct Engine {
    options: Options,
    frame_sender: AsyncFrameSender,
    frame_pool: Arc<FramePool>,

    #[cfg(target_os = "macos")]
    _mac_placeholder: bool,

    #[cfg(target_os = "windows")]
    win: win::WCStream,

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    linux: linux::LinuxCapturer,
}

impl Engine {
    pub fn new(options: Options, frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                options,
                frame_sender,
                frame_pool,
                _mac_placeholder: false,
            })
        }

        #[cfg(target_os = "windows")]
        {
            let win = win::create_capturer(&options, frame_sender.clone())?;
            
            Ok(Self {
                options,
                frame_sender,
                frame_pool,
                win,
            })
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            let linux = linux::create_capturer(&options, frame_sender.clone())?;
            
            Ok(Self {
                options,
                frame_sender,
                frame_pool,
                linux,
            })
        }
    }

    pub async fn start_capture(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Placeholder implementation for macOS
            log::warn!("macOS screen capture not fully implemented in this version");
            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            self.win.start_capture();
            Ok(())
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            self.linux.start_capture();
            Ok(())
        }
    }

    pub async fn stop_capture(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Placeholder implementation for macOS
            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            self.win.stop_capture();
            Ok(())
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            self.linux.stop_capture();
            Ok(())
        }
    }

    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        get_output_frame_size(&self.options)
    }

    pub async fn process_channel_item(&self, data: ChannelItem) -> Option<Frame> {
        #[cfg(target_os = "macos")]
        {
            // Placeholder - return None for now
            None
        }
        #[cfg(not(target_os = "macos"))]
        Some(data)
    }
}