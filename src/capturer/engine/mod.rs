use std::sync::Arc;
use anyhow::Result;
use tokio::sync::Mutex;

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
        mac::get_output_frame_size(options)
    }

    #[cfg(target_os = "windows")]
    {
        win::get_output_frame_size(options)
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        // TODO: How to calculate this on Linux?
        return [0, 0];
    }
}

pub struct Engine {
    options: Options,
    frame_sender: AsyncFrameSender,
    frame_pool: Arc<FramePool>,

    #[cfg(target_os = "macos")]
    mac: screencapturekit::stream::SCStream,
    #[cfg(target_os = "macos")]
    error_flag: Arc<std::sync::atomic::AtomicBool>,

    #[cfg(target_os = "windows")]
    win: win::WCStream,

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    linux: linux::LinuxCapturer,
}

impl Engine {
    pub fn new(options: Options, frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            let error_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let mac = mac::create_stream(&options, frame_sender.clone(), Arc::clone(&error_flag), Arc::clone(&frame_pool))?;
            
            Ok(Self {
                options,
                frame_sender,
                frame_pool,
                mac,
                error_flag,
            })
        }

        #[cfg(target_os = "windows")]
        {
            let win = win::WCStream::new(&options, frame_sender.clone(), Arc::clone(&frame_pool))?;
            
            Ok(Self {
                options,
                frame_sender,
                frame_pool,
                win,
            })
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            let linux = linux::LinuxCapturer::new(&options, frame_sender.clone(), Arc::clone(&frame_pool))?;
            
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
            self.mac.start_capture().map_err(|e| anyhow::anyhow!("Failed to start capture: {}", e))?;
        }

        #[cfg(target_os = "windows")]
        {
            self.win.start_capture();
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            self.linux.start_capture();
        }

        Ok(())
    }

    pub async fn stop_capture(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            self.mac.stop_capture().map_err(|e| anyhow::anyhow!("Failed to stop capture: {}", e))?;
        }

        #[cfg(target_os = "windows")]
        {
            self.win.stop_capture();
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            self.linux.stop_capture();
        }

        Ok(())
    }

    pub fn get_output_frame_size(&mut self) -> [u32; 2] {
        get_output_frame_size(&self.options)
    }

    pub async fn process_channel_item(&self, data: ChannelItem) -> Option<Frame> {
        #[cfg(target_os = "macos")]
        {
            mac::process_sample_buffer(data.0, data.1, self.options.output_type, &self.frame_pool)
        }
        #[cfg(not(target_os = "macos"))]
        Some(data)
    }
}
