use std::slice;

use core_media_rs::cm_sample_buffer::CMSampleBuffer;

use super::{
    pixel_buffer::{pixel_buffer_bounds, sample_buffer_to_pixel_buffer},
};
use crate::frame::{
    convert_bgra_to_rgb, get_cropped_data, remove_alpha_channel, BGRAFrame, BGRFrame, RGBFrame,
    YUVFrame,
};

use core_video_sys::{
    CVPixelBufferGetBaseAddress, CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRow,
    CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferLockBaseAddress,
    CVPixelBufferUnlockBaseAddress,
};

// Returns a frame's presentation timestamp in nanoseconds since an arbitrary start time.
// This is typically yielded from a monotonic clock started on system boot.
pub fn get_pts_in_nanoseconds(_sample_buffer: &CMSampleBuffer) -> u64 {
    // Simplified for now - return 0 or current time
    0
}

pub unsafe fn create_yuv_frame(sample_buffer: CMSampleBuffer) -> Option<YUVFrame> {
    let display_time = get_pts_in_nanoseconds(&sample_buffer);
    let pixel_buffer = match sample_buffer_to_pixel_buffer(&sample_buffer) {
        Ok(buffer) => buffer,
        Err(_) => return None,
    };

    CVPixelBufferLockBaseAddress(pixel_buffer, 0);

    let (width, height) = pixel_buffer_bounds(pixel_buffer);
    if width == 0 || height == 0 {
        return None;
    }

    let luminance_bytes_address = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 0);
    let luminance_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 0);
    let luminance_bytes = slice::from_raw_parts(
        luminance_bytes_address as *mut u8,
        height * luminance_stride,
    )
    .to_vec();

    let chrominance_bytes_address = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 1);
    let chrominance_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 1);
    let chrominance_bytes = slice::from_raw_parts(
        chrominance_bytes_address as *mut u8,
        height * chrominance_stride / 2,
    )
    .to_vec();

    CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);

    YUVFrame {
        display_time,
        width: width as i32,
        height: height as i32,
        luminance_bytes,
        luminance_stride: luminance_stride as i32,
        chrominance_bytes,
        chrominance_stride: chrominance_stride as i32,
    }
    .into()
}

pub unsafe fn create_bgr_frame(sample_buffer: CMSampleBuffer) -> Option<BGRFrame> {
    let pixel_buffer = match sample_buffer_to_pixel_buffer(&sample_buffer) {
        Ok(buffer) => buffer,
        Err(_) => return None,
    };
    let display_time = get_pts_in_nanoseconds(&sample_buffer);

    CVPixelBufferLockBaseAddress(pixel_buffer, 0);

    let (width, height) = pixel_buffer_bounds(pixel_buffer);
    if width == 0 || height == 0 {
        return None;
    }

    let base_address = CVPixelBufferGetBaseAddress(pixel_buffer);
    let bytes_per_row = CVPixelBufferGetBytesPerRow(pixel_buffer);

    let data = slice::from_raw_parts(base_address as *mut u8, bytes_per_row * height).to_vec();

    let cropped_data = get_cropped_data(
        data,
        (bytes_per_row / 4) as i32,
        height as i32,
        width as i32,
    );

    CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);

    Some(BGRFrame {
        display_time,
        width: width as i32, // width does not give accurate results - https://stackoverflow.com/questions/19587185/cvpixelbuffergetbytesperrow-for-cvimagebufferref-returns-unexpected-wrong-valu
        height: height as i32,
        data: remove_alpha_channel(cropped_data),
    })
}

pub unsafe fn create_bgra_frame(sample_buffer: CMSampleBuffer) -> Option<BGRAFrame> {
    let pixel_buffer = match sample_buffer_to_pixel_buffer(&sample_buffer) {
        Ok(buffer) => buffer,
        Err(_) => return None,
    };
    let display_time = get_pts_in_nanoseconds(&sample_buffer);

    CVPixelBufferLockBaseAddress(pixel_buffer, 0);

    let (width, height) = pixel_buffer_bounds(pixel_buffer);
    if width == 0 || height == 0 {
        return None;
    }

    let base_address = CVPixelBufferGetBaseAddress(pixel_buffer);
    let bytes_per_row = CVPixelBufferGetBytesPerRow(pixel_buffer);

    // Get the full buffer data including padding
    let data = slice::from_raw_parts(base_address as *mut u8, bytes_per_row * height).to_vec();

    // Use get_cropped_data to handle the stride/padding correctly
    let cropped_data = get_cropped_data(
        data,
        (bytes_per_row / 4) as i32,  // Current width including padding
        height as i32,
        width as i32,                // Actual width without padding
    );

    CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);

    Some(BGRAFrame {
        display_time,
        width: width as i32,
        height: height as i32,
        data: cropped_data,
    })
}

pub unsafe fn create_rgb_frame(sample_buffer: CMSampleBuffer) -> Option<RGBFrame> {
    let pixel_buffer = match sample_buffer_to_pixel_buffer(&sample_buffer) {
        Ok(buffer) => buffer,
        Err(_) => return None,
    };
    let display_time = get_pts_in_nanoseconds(&sample_buffer);

    CVPixelBufferLockBaseAddress(pixel_buffer, 0);

    let (width, height) = pixel_buffer_bounds(pixel_buffer);
    if width == 0 || height == 0 {
        return None;
    }

    let base_address = CVPixelBufferGetBaseAddress(pixel_buffer);
    let bytes_per_row = CVPixelBufferGetBytesPerRow(pixel_buffer);

    let data = slice::from_raw_parts(base_address as *mut u8, bytes_per_row * height).to_vec();

    let cropped_data = get_cropped_data(
        data,
        (bytes_per_row / 4) as i32,
        height as i32,
        width as i32,
    );

    CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);

    Some(RGBFrame {
        display_time,
        width: width as i32, // width does not give accurate results - https://stackoverflow.com/questions/19587185/cvpixelbuffergetbytesperrow-for-cvimagebufferref-returns-unexpected-wrong-valu
        height: height as i32,
        data: convert_bgra_to_rgb(cropped_data),
    })
}
