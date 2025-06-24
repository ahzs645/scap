use core::slice;
use core_video_sys::{
    CVPixelBufferGetBaseAddress, CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRow,
    CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferGetHeight, CVPixelBufferGetHeightOfPlane,
    CVPixelBufferGetPlaneCount, CVPixelBufferGetWidth, CVPixelBufferGetWidthOfPlane,
    CVPixelBufferLockBaseAddress, CVPixelBufferRef, CVPixelBufferUnlockBaseAddress,
};
use core_media_rs::cm_sample_buffer::CMSampleBuffer;
use core_foundation::base::TCFType;
use std::{ops::Deref, sync::mpsc};

use crate::capturer::{engine::ChannelItem, RawCapturer};

pub struct PixelBuffer {
    pub display_time: u64,
    pub width: usize,
    pub height: usize,
    pub bytes_per_row: usize,
    pub buffer: CMSampleBuffer,
}

impl PixelBuffer {
    pub fn display_time(&self) -> u64 {
        self.display_time
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn buffer(&self) -> &CMSampleBuffer {
        &self.buffer
    }

    pub fn bytes_per_row(&self) -> usize {
        self.bytes_per_row
    }

    pub fn data(&self) -> PixelBufferData {
        unsafe {
            let pixel_buffer = sample_buffer_to_pixel_buffer(&self.buffer)
                .expect("PixelBuffer should always have valid image buffer");

            CVPixelBufferLockBaseAddress(pixel_buffer, 0);

            let base_address = CVPixelBufferGetBaseAddress(pixel_buffer);
            
            // Validate the pointer and size before creating slice
            if base_address.is_null() {
                panic!("CVPixelBufferGetBaseAddress returned null pointer");
            }
            
            let total_size = self.bytes_per_row * self.height;
            if total_size == 0 {
                panic!("Invalid buffer size: bytes_per_row={}, height={}", self.bytes_per_row, self.height);
            }

            PixelBufferData {
                buffer: pixel_buffer,
                data: slice::from_raw_parts(
                    base_address as *mut _,
                    total_size,
                ),
            }
        }
    }

    pub fn planes(&self) -> Vec<Plane> {
        unsafe {
            let pixel_buffer = sample_buffer_to_pixel_buffer(&self.buffer)
                .expect("PixelBuffer should always have valid image buffer");
            let count = CVPixelBufferGetPlaneCount(pixel_buffer);

            CVPixelBufferLockBaseAddress(pixel_buffer, 0);

            (0..count)
                .map(|i| Plane {
                    buffer: pixel_buffer,
                    width: CVPixelBufferGetWidthOfPlane(pixel_buffer, i),
                    height: CVPixelBufferGetHeightOfPlane(pixel_buffer, i),
                    bytes_per_row: CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, i),
                    index: i,
                })
                .collect()
        }
    }

    pub fn from_channel_item(item: ChannelItem) -> Option<Self> {
        let display_time = 0; // Simplified for now
        
        // Check if this sample buffer has an image buffer (video) vs audio buffer
        let image_buffer = match item.0.get_image_buffer() {
            Ok(buffer) => buffer,
            Err(_) => return None, // This is likely an audio buffer, skip it
        };
        
        let pixel_buffer = image_buffer.as_CFTypeRef() as CVPixelBufferRef;

        let (width, height) = unsafe { pixel_buffer_bounds(pixel_buffer) };

        if width == 0 || height == 0 {
            return None;
        }

        // With core-media-rs, we don't need to check frame status
        Some(Self {
            display_time,
            width,
            height,
            bytes_per_row: unsafe { pixel_buffer_bytes_per_row(pixel_buffer) },
            buffer: item.0,
        })
    }
}

impl Into<CMSampleBuffer> for PixelBuffer {
    fn into(self) -> CMSampleBuffer {
        self.buffer
    }
}

#[derive(Debug)]
pub struct Plane {
    buffer: CVPixelBufferRef,
    index: usize,
    width: usize,
    height: usize,
    bytes_per_row: usize,
}

impl Plane {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn bytes_per_row(&self) -> usize {
        self.bytes_per_row
    }

    pub fn data(&self) -> PixelBufferData {
        unsafe {
            CVPixelBufferLockBaseAddress(self.buffer, 0);

            let base_address = CVPixelBufferGetBaseAddressOfPlane(self.buffer, self.index);
            
            // Validate the pointer and size before creating slice
            if base_address.is_null() {
                panic!("CVPixelBufferGetBaseAddressOfPlane returned null pointer");
            }
            
            let total_size = self.bytes_per_row * self.height;
            if total_size == 0 {
                panic!("Invalid plane buffer size: bytes_per_row={}, height={}", self.bytes_per_row, self.height);
            }

            PixelBufferData {
                buffer: self.buffer,
                data: slice::from_raw_parts(
                    base_address as *mut _,
                    total_size,
                ),
            }
        }
    }
}

pub struct PixelBufferData<'a> {
    buffer: CVPixelBufferRef,
    data: &'a [u8],
}

impl<'a> Deref for PixelBufferData<'a> {
    type Target = [u8];

    fn deref(&self) -> &'a Self::Target {
        self.data
    }
}

impl<'a> Drop for PixelBufferData<'a> {
    fn drop(&mut self) {
        unsafe { CVPixelBufferUnlockBaseAddress(self.buffer, 0) };
    }
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

pub unsafe fn sample_buffer_to_pixel_buffer(sample_buffer: &CMSampleBuffer) -> Result<CVPixelBufferRef, Box<dyn std::error::Error>> {
    // Use core-media-rs API to get the image buffer
    let image_buffer = sample_buffer.get_image_buffer().map_err(|e| format!("Failed to get image buffer: {:?}", e))?;
    // Get the raw pointer from the CVImageBuffer
    Ok(image_buffer.as_CFTypeRef() as CVPixelBufferRef)
}

pub unsafe fn pixel_buffer_bounds(pixel_buffer: CVPixelBufferRef) -> (usize, usize) {
    let width = CVPixelBufferGetWidth(pixel_buffer);
    let height = CVPixelBufferGetHeight(pixel_buffer);
    (width, height)
}

pub unsafe fn pixel_buffer_bytes_per_row(pixel_buffer: CVPixelBufferRef) -> usize {
    CVPixelBufferGetBytesPerRow(pixel_buffer)
}

pub unsafe fn pixel_buffer_display_time(_sample_buffer: &CMSampleBuffer) -> u64 {
    // Simplified for now - return current time or 0
    0
}
