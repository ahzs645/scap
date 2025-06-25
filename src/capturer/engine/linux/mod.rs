use std::{env, sync::mpsc};
use anyhow::{anyhow, Result};
use crate::{capturer::{Options, async_frame::AsyncFrameSender}, frame::Frame};

mod error;

#[cfg(feature = "wayland")]
mod wayland;
mod x11;

#[cfg(feature = "wayland")]
use wayland::WaylandCapturer;
use x11::X11Capturer;

pub trait LinuxCapturerImpl {
    fn start_capture(&mut self);
    fn stop_capture(&mut self);
}

pub struct LinuxCapturer {
    pub imp: Box<dyn LinuxCapturerImpl + Send>,
}

impl LinuxCapturer {
    pub fn new(options: &Options, tx: AsyncFrameSender) -> Result<Self> {
        #[cfg(feature = "wayland")]
        if env::var("WAYLAND_DISPLAY").is_ok() {
            log::debug!("Creating new Wayland screen capturer.");
            return Ok(Self {
                imp: Box::new(WaylandCapturer::new(options, tx)?),
            });
        }

        if env::var("DISPLAY").is_ok() {
            log::debug!("Creating new X11 screen capturer.");
            Ok(Self {
                imp: Box::new(X11Capturer::new(options, tx)?),
            })
        } else {
            #[cfg(feature = "wayland")]
            let error_msg = "Unsupported platform. Could not detect Wayland or X11 displays";
            #[cfg(not(feature = "wayland"))]
            let error_msg = "Unsupported platform. Could not detect X11 display. Enable the 'wayland' feature for Wayland support.";
            
            Err(anyhow!(error_msg))
        }
    }

    pub fn start_capture(&mut self) {
        self.imp.start_capture();
    }

    pub fn stop_capture(&mut self) {
        self.imp.stop_capture();
    }
}

pub fn create_capturer(
    options: &Options,
    tx: AsyncFrameSender,
) -> Result<LinuxCapturer> {
    LinuxCapturer::new(options, tx)
}