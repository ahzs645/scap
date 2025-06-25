use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// A pool of reusable frame buffers to minimize allocations
pub struct FramePool {
    video_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    audio_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    max_pool_size: usize,
}

impl FramePool {
    /// Creates a new frame pool with the specified maximum size
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            video_buffers: Arc::new(Mutex::new(VecDeque::with_capacity(max_pool_size))),
            audio_buffers: Arc::new(Mutex::new(VecDeque::with_capacity(max_pool_size))),
            max_pool_size,
        }
    }

    /// Gets a video buffer from the pool or creates a new one
    pub fn get_video_buffer(&self, min_size: usize) -> Vec<u8> {
        let mut pool = self.video_buffers.lock().unwrap();
        
        // Try to find a buffer of suitable size
        for i in 0..pool.len() {
            if pool[i].capacity() >= min_size {
                let mut buffer = pool.remove(i).unwrap();
                buffer.clear(); // Clear but preserve capacity
                return buffer;
            }
        }
        
        // Create new buffer if none found
        Vec::with_capacity(min_size)
    }

    /// Gets an audio buffer from the pool or creates a new one
    pub fn get_audio_buffer(&self, min_size: usize) -> Vec<u8> {
        let mut pool = self.audio_buffers.lock().unwrap();
        
        // Try to find a buffer of suitable size
        for i in 0..pool.len() {
            if pool[i].capacity() >= min_size {
                let mut buffer = pool.remove(i).unwrap();
                buffer.clear(); // Clear but preserve capacity
                return buffer;
            }
        }
        
        // Create new buffer if none found
        Vec::with_capacity(min_size)
    }

    /// Returns a video buffer to the pool
    pub fn return_video_buffer(&self, buffer: Vec<u8>) {
        let mut pool = self.video_buffers.lock().unwrap();
        if pool.len() < self.max_pool_size {
            pool.push_back(buffer);
        }
    }

    /// Returns an audio buffer to the pool
    pub fn return_audio_buffer(&self, buffer: Vec<u8>) {
        let mut pool = self.audio_buffers.lock().unwrap();
        if pool.len() < self.max_pool_size {
            pool.push_back(buffer);
        }
    }

    /// Creates a clone of the pool that shares the same underlying buffers
    pub fn clone(&self) -> Self {
        Self {
            video_buffers: Arc::clone(&self.video_buffers),
            audio_buffers: Arc::clone(&self.audio_buffers),
            max_pool_size: self.max_pool_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_buffer_reuse() {
        let pool = FramePool::new(2);
        
        // Get a buffer and fill it
        let mut buf1 = pool.get_video_buffer(1024);
        buf1.extend_from_slice(&[1; 1024]);
        assert_eq!(buf1.capacity(), 1024);
        
        // Return it to the pool
        pool.return_video_buffer(buf1);
        
        // Get another buffer - should reuse the first one
        let buf2 = pool.get_video_buffer(512);
        assert_eq!(buf2.capacity(), 1024); // Same capacity as original
        assert!(buf2.is_empty()); // But should be empty
    }

    #[test]
    fn test_audio_buffer_reuse() {
        let pool = FramePool::new(2);
        
        // Get a buffer and fill it
        let mut buf1 = pool.get_audio_buffer(1024);
        buf1.extend_from_slice(&[1; 1024]);
        assert_eq!(buf1.capacity(), 1024);
        
        // Return it to the pool
        pool.return_audio_buffer(buf1);
        
        // Get another buffer - should reuse the first one
        let buf2 = pool.get_audio_buffer(512);
        assert_eq!(buf2.capacity(), 1024); // Same capacity as original
        assert!(buf2.is_empty()); // But should be empty
    }

    #[test]
    fn test_pool_size_limit() {
        let pool = FramePool::new(1);
        
        // Add two buffers but only one should be stored
        pool.return_video_buffer(Vec::with_capacity(1024));
        pool.return_video_buffer(Vec::with_capacity(2048));
        
        let buffers = pool.video_buffers.lock().unwrap();
        assert_eq!(buffers.len(), 1);
    }
} 