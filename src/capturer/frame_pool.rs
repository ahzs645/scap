use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};
use anyhow::Result;
use crate::frame::Frame;

/// A pool of reusable frame buffers to minimize allocations
pub struct FramePool {
    video_buffers: Mutex<HashMap<usize, Vec<Vec<u8>>>>,
    audio_buffers: Mutex<HashMap<usize, Vec<Vec<u8>>>>,
    frame_queue: Mutex<VecDeque<Frame>>,
    last_frame_time: Mutex<Option<Instant>>,
}

impl FramePool {
    /// Creates a new frame pool
    pub fn new() -> Self {
        Self {
            video_buffers: Mutex::new(HashMap::new()),
            audio_buffers: Mutex::new(HashMap::new()),
            frame_queue: Mutex::new(VecDeque::new()),
            last_frame_time: Mutex::new(None),
        }
    }

    /// Gets a video buffer from the pool or creates a new one
    pub fn get_video_buffer(&self, size: usize) -> Result<Vec<u8>> {
        let mut buffers = self.video_buffers.lock().unwrap();
        
        // Try to get a buffer of the exact size
        if let Some(pool) = buffers.get_mut(&size) {
            if let Some(buffer) = pool.pop() {
                return Ok(buffer);
            }
        }
        
        // Create a new buffer if none available
        Ok(Vec::with_capacity(size))
    }

    /// Gets an audio buffer from the pool or creates a new one
    pub fn get_audio_buffer(&self, size: usize) -> Result<Vec<u8>> {
        let mut buffers = self.audio_buffers.lock().unwrap();
        
        // Try to get a buffer of the exact size
        if let Some(pool) = buffers.get_mut(&size) {
            if let Some(buffer) = pool.pop() {
                return Ok(buffer);
            }
        }
        
        // Create a new buffer if none available
        Ok(Vec::with_capacity(size))
    }

    /// Returns a video buffer to the pool
    pub fn return_video_buffer(&self, mut buffer: Vec<u8>) {
        let size = buffer.capacity();
        buffer.clear();
        
        let mut buffers = self.video_buffers.lock().unwrap();
        buffers.entry(size).or_insert_with(Vec::new).push(buffer);
    }

    /// Returns an audio buffer to the pool
    pub fn return_audio_buffer(&self, mut buffer: Vec<u8>) {
        let size = buffer.capacity();
        buffer.clear();
        
        let mut buffers = self.audio_buffers.lock().unwrap();
        buffers.entry(size).or_insert_with(Vec::new).push(buffer);
    }

    /// Gets the next frame from the pool
    pub fn get_next_frame(&self) -> Option<Frame> {
        let timeout = Duration::from_millis(100);
        let start_time = Instant::now();
        
        while start_time.elapsed() < timeout {
            if let Ok(mut queue) = self.frame_queue.try_lock() {
                if let Some(frame) = queue.pop_front() {
                    return Some(frame);
                }
            }
            // Small sleep to avoid busy waiting
            std::thread::sleep(Duration::from_millis(1));
        }
        None
    }
    
    /// Pushes a frame into the pool
    pub fn push_frame(&self, frame: Frame) {
        if let Ok(mut queue) = self.frame_queue.try_lock() {
            // Keep queue size reasonable
            while queue.len() > 30 {
                queue.pop_front();
            }
            queue.push_back(frame);
            *self.last_frame_time.lock().unwrap() = Some(Instant::now());
        }
    }
    
    /// Pops a frame from the pool
    pub fn pop_frame(&self) -> Option<Frame> {
        self.frame_queue.lock().ok()?.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_buffer_reuse() {
        let pool = FramePool::new();
        
        // Get a new buffer
        let mut buffer = pool.get_video_buffer(1024).unwrap();
        buffer.extend_from_slice(&[1; 1024]);
        
        // Return it to the pool
        pool.return_video_buffer(buffer);
        
        // Get it back
        let buffer2 = pool.get_video_buffer(1024).unwrap();
        assert_eq!(buffer2.capacity(), 1024);
        assert!(buffer2.is_empty());
    }

    #[test]
    fn test_audio_buffer_reuse() {
        let pool = FramePool::new();
        
        // Get a new buffer
        let mut buffer = pool.get_audio_buffer(512).unwrap();
        buffer.extend_from_slice(&[1; 512]);
        
        // Return it to the pool
        pool.return_audio_buffer(buffer);
        
        // Get it back
        let buffer2 = pool.get_audio_buffer(512).unwrap();
        assert_eq!(buffer2.capacity(), 512);
        assert!(buffer2.is_empty());
    }

    #[test]
    fn test_different_size_buffers() {
        let pool = FramePool::new();
        
        // Get buffers of different sizes
        let buffer1 = pool.get_video_buffer(1024).unwrap();
        let buffer2 = pool.get_video_buffer(2048).unwrap();
        
        assert_eq!(buffer1.capacity(), 1024);
        assert_eq!(buffer2.capacity(), 2048);
        
        // Return them to the pool
        pool.return_video_buffer(buffer1);
        pool.return_video_buffer(buffer2);
        
        // Get them back
        let buffer3 = pool.get_video_buffer(1024).unwrap();
        let buffer4 = pool.get_video_buffer(2048).unwrap();
        
        assert_eq!(buffer3.capacity(), 1024);
        assert_eq!(buffer4.capacity(), 2048);
    }
}