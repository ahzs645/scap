use anyhow::Result;
use crate::{
    capturer::{Options, async_frame::AsyncFrameSender},
};
use super::{error::LinCapError, LinuxCapturerImpl};

pub struct X11Capturer {
    _placeholder: bool,
}

impl X11Capturer {
    pub fn new(_options: &Options, _tx: AsyncFrameSender) -> Result<Self, LinCapError> {
        log::warn!("X11 screen capture is not fully implemented in this version");
        Ok(Self {
            _placeholder: false,
        })
    }
}

impl LinuxCapturerImpl for X11Capturer {
    fn start_capture(&mut self) {
        log::warn!("X11 start_capture called but not implemented");
    }

    fn stop_capture(&mut self) {
        log::warn!("X11 stop_capture called but not implemented");
    }
}
