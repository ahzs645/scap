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

// Simplified channel item types to avoid compilation issues
#[cfg(target_os = "macos")]
pub type ChannelItem = (Vec<u8>, u32, u32); // (data, width, height) - simplified
#[cfg(not(target_os = "macos"))]
pub type ChannelItem = Frame;

pub fn get_output_frame_size(options: &Options) -> [u32; 2] {
    #[cfg(target_os = "macos")]
    {
        // Use a simple default for now to avoid compilation issues
        match &options.target {
            Some(crate::targets::Target::Display(display)) => [display.width as u32, display.height as u32],
            Some(crate::targets::Target::Window(window)) => [window.width as u32, window.height as u32],
            None => [1920, 1080],
        }
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
    mac_capturer: Option<mac::ScreenCapturer>,

    #[cfg(target_os = "windows")]
    win: win::WCStream,

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    linux: linux::LinuxCapturer,
}

impl Engine {
    pub fn new(options: Options, frame_sender: AsyncFrameSender, frame_pool: Arc<FramePool>) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            let mac_capturer = Some(mac::ScreenCapturer::new(frame_sender.clone(), Arc::clone(&frame_pool))?);
            
            Ok(Self {
                options,
                frame_sender,
                frame_pool,
                mac_capturer,
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
            if let Some(ref mut capturer) = self.mac_capturer {
                // Extract target from options
                let default_target = crate::targets::Target::Display(
                    crate::targets::get_main_display().unwrap()
                );
                let target = self.options.target.as_ref().unwrap_or(&default_target);
                // Remove .await since start_capture is not async
                capturer.start_capture(target)?;
            }
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
            if let Some(ref mut capturer) = self.mac_capturer {
                // Remove .await since stop_capture is not async
                capturer.stop_capture()?;
            }
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
            // Convert simplified channel item to frame
            let (data, width, height) = data;
            
            // Create a BGRA frame from the data
            if !data.is_empty() && width > 0 && height > 0 {
                let display_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                
                Some(Frame::BGRA(crate::frame::BGRAFrame {
                    display_time,
                    width: width as i32,
                    height: height as i32,
                    data,
                }))
            } else {
                None
            }
        }
        #[cfg(not(target_os = "macos"))]
        Some(data)
    }
}