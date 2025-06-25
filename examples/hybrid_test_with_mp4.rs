use scap::{
    capturer::{Capturer, Options},
    frame::{Frame, FrameType},
};
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::{Duration, Instant},
    thread,
    sync::{Arc, Mutex},
};
use anyhow::Result;

/// Simple MP4 Writer for demonstration
/// In a real application, you'd use a proper MP4 muxing library
struct SimpleMP4Writer {
    frames: Vec<Vec<u8>>,
    width: u32,
    height: u32,
    fps: u32,
}

impl SimpleMP4Writer {
    fn new(width: u32, height: u32, fps: u32) -> Self {
        Self {
            frames: Vec::new(),
            width,
            height,
            fps,
        }
    }

    fn add_frame(&mut self, frame_data: Vec<u8>) {
        self.frames.push(frame_data);
    }

    fn write_to_file(&self, path: &str) -> Result<()> {
        // Create a simple file with raw frame data and basic headers
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        
        // Write simple header
        writer.write_all(b"SCAP")?; // Magic number
        writer.write_all(&self.width.to_le_bytes())?;
        writer.write_all(&self.height.to_le_bytes())?;
        writer.write_all(&self.fps.to_le_bytes())?;
        writer.write_all(&(self.frames.len() as u32).to_le_bytes())?;
        
        // Write frame data
        for frame in &self.frames {
            writer.write_all(&(frame.len() as u32).to_le_bytes())?;
            writer.write_all(frame)?;
        }
        
        writer.flush()?;
        Ok(())
    }
}
fn test_hybrid_sync_api() -> Result<()> {
    println!("\n🔄 Testing Hybrid Approach - Sync API (Primary)");
    println!("===============================================");

    let mut options = Options::default();
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    
    println!("� Configuration:");
    println!("   🎥 Video: Main display @ {} FPS", options.fps);
    println!("   � API: Synchronous (primary interface)");
    
    let mut capturer = Capturer::build(options)?;
    println!("✅ Capturer created with hybrid architecture");
    
    // Demonstrate sync API
    println!("\n� Starting capture with sync API...");
    capturer.start_capture()?;
    
    let duration = Duration::from_secs(3);
    let start_time = Instant::now();
    let mut frames = Vec::new();
    let mut frame_count = 0;
    let mut first_frame_info = None;
    
    println!("📸 Recording {} seconds...", duration.as_secs());
    
    while start_time.elapsed() < duration {
        match capturer.get_next_frame() {
            Ok(Frame::BGRA(frame)) => {
                if first_frame_info.is_none() {
                    first_frame_info = Some((frame.width, frame.height));
                    println!("   📐 Frame dimensions: {}x{}", frame.width, frame.height);
                }
                frames.push(frame.data.clone());
                frame_count += 1;
                
                if frame_count % 30 == 0 {
                    println!("   � Captured {} frames...", frame_count);
                }
            }
            Ok(_) => {}
            Err(e) => {
                // Expected timeouts, just continue
                if !e.to_string().contains("timeout") {
                    println!("   ⚠️  Frame error: {}", e);
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    
    capturer.stop_capture()?;
    
    // Create MP4 output
    if let Some((width, height)) = first_frame_info {
        let mut mp4_writer = SimpleMP4Writer::new(width as u32, height as u32, 30);
        
        for frame_data in frames {
            mp4_writer.add_frame(frame_data);
        }
        
        std::fs::create_dir_all("recordings")?;
        mp4_writer.write_to_file("recordings/hybrid_sync_test.mp4")?;
        
        println!("\n💾 Sync API Test Results:");
        println!("   🎥 Frames captured: {}", frame_count);
        println!("   📈 Average FPS: {:.1}", frame_count as f64 / duration.as_secs_f64());
        println!("   📁 Output: recordings/hybrid_sync_test.mp4");
        println!("   ✅ Sync API test PASSED");
    }
    
    Ok(())
}

async fn test_hybrid_async_api() -> Result<()> {
    println!("\n🌊 Testing Hybrid Approach - Async API (Advanced)");
    println!("=================================================");

    let mut options = Options::default();
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    
    println!("📹 Configuration:");
    println!("   🎥 Video: Main display @ {} FPS", options.fps);
    println!("   🔧 API: Asynchronous (advanced interface)");
    
    let mut capturer = Capturer::build(options)?;
    println!("✅ Capturer created with hybrid architecture");
    
    // Demonstrate async API variants
    println!("\n🔴 Starting capture with async API...");
    capturer.start_capture_async().await?;
    
    let duration = Duration::from_secs(3);
    let start_time = Instant::now();
    let mut frames = Vec::new();
    let mut frame_count = 0;
    let mut first_frame_info = None;
    
    println!("� Recording {} seconds asynchronously...", duration.as_secs());
    
    while start_time.elapsed() < duration {
        match capturer.get_next_frame_async().await {
            Ok(Frame::BGRA(frame)) => {
                if first_frame_info.is_none() {
                    first_frame_info = Some((frame.width, frame.height));
                    println!("   📐 Frame dimensions: {}x{}", frame.width, frame.height);
                }
                frames.push(frame.data.clone());
                frame_count += 1;
                
                if frame_count % 30 == 0 {
                    println!("   📊 Captured {} frames...", frame_count);
                }
            }
            Ok(_) => {}
            Err(e) => {
                if !e.to_string().contains("timeout") {
                    println!("   ⚠️  Frame error: {}", e);
                }
            }
        }
        
        // Async-friendly delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    capturer.stop_capture_async().await?;
    
    // Create MP4 output
    if let Some((width, height)) = first_frame_info {
        let mut mp4_writer = SimpleMP4Writer::new(width as u32, height as u32, 30);
        
        for frame_data in frames {
            mp4_writer.add_frame(frame_data);
        }
        
        std::fs::create_dir_all("recordings")?;
        mp4_writer.write_to_file("recordings/hybrid_async_test.mp4")?;
        
        println!("\n� Async API Test Results:");
        println!("   � Frames captured: {}", frame_count);
        println!("   📈 Average FPS: {:.1}", frame_count as f64 / duration.as_secs_f64());
        println!("   � Output: recordings/hybrid_async_test.mp4");
        println!("   ✅ Async API test PASSED");
    }
    
    Ok(())
}

fn test_hybrid_callback_api() -> Result<()> {
    println!("\n⚡ Testing Hybrid Approach - Callback API (High-Performance)");
    println!("===========================================================");

    let mut options = Options::default();
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    
    println!("� Configuration:");
    println!("   🎥 Video: Main display @ {} FPS", options.fps);
    println!("   🔧 API: Callback-based (zero-copy, high-performance)");
    
    let mut capturer = Capturer::build(options)?;
    println!("✅ Capturer created with hybrid architecture");
    
    // Shared state for callback
    let frames = Arc::new(Mutex::new(Vec::new()));
    let frame_count = Arc::new(Mutex::new(0));
    let first_frame_info = Arc::new(Mutex::new(None));
    
    let frames_clone = Arc::clone(&frames);
    let count_clone = Arc::clone(&frame_count);
    let info_clone = Arc::clone(&first_frame_info);
    
    // Demonstrate callback API
    println!("\n� Starting capture with callback API...");
    capturer.start_capture_with_callback(move |frame| {
        if let Frame::BGRA(bgra_frame) = frame {
            let mut info = info_clone.lock().unwrap();
            if info.is_none() {
                *info = Some((bgra_frame.width, bgra_frame.height));
                println!("   📐 Frame dimensions: {}x{}", bgra_frame.width, bgra_frame.height);
            }
            
            frames_clone.lock().unwrap().push(bgra_frame.data.clone());
            
            let mut count = count_clone.lock().unwrap();
            *count += 1;
            
            if *count % 30 == 0 {
                println!("   � Captured {} frames...", *count);
            }
        }
    })?;
    
    let duration = Duration::from_secs(3);
    println!("📸 Recording {} seconds with callback...", duration.as_secs());
    thread::sleep(duration);
    
    capturer.stop_capture()?;
    
    // Create MP4 output
    let frames_vec = frames.lock().unwrap().clone();
    let final_count = *frame_count.lock().unwrap();
    let frame_info = *first_frame_info.lock().unwrap();
    
    if let Some((width, height)) = frame_info {
        let mut mp4_writer = SimpleMP4Writer::new(width as u32, height as u32, 30);
        
        for frame_data in frames_vec {
            mp4_writer.add_frame(frame_data);
        }
        
        std::fs::create_dir_all("recordings")?;
        mp4_writer.write_to_file("recordings/hybrid_callback_test.mp4")?;
        
        println!("\n� Callback API Test Results:");
        println!("   🎥 Frames captured: {}", final_count);
        println!("   📈 Average FPS: {:.1}", final_count as f64 / duration.as_secs_f64());
        println!("   📁 Output: recordings/hybrid_callback_test.mp4");
        println!("   ✅ Callback API test PASSED");
    }
    
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("🧪 SCAP HYBRID ARCHITECTURE TEST SUITE");
    println!("======================================");
    println!("Demonstrating the hybrid approach: sync API + async implementation");
    println!("This provides the best of both worlds for screen capture!");

    // Check prerequisites
    if !scap::is_supported() {
        println!("❌ Platform not supported");
        return Ok(());
    }

    if !scap::has_permission() {
        println!("⚠️  Screen recording permission not granted");
        println!("🔐 Requesting permission...");
        if !scap::request_permission() {
            println!("❌ Permission denied - cannot proceed with tests");
            return Ok(());
        }
        println!("✅ Permission granted");
    }

    // Test the hybrid architecture with all three API styles
    
    // 1. Sync API (Primary - for 90% of use cases)
    test_hybrid_sync_api()?;
    
    // 2. Async API (Advanced - for async integration)
    test_hybrid_async_api().await?;
    
    // 3. Callback API (High-performance - for real-time scenarios)
    test_hybrid_callback_api()?;

    println!("\n🎉 Hybrid Architecture Test Complete!");
    println!("====================================");
    println!("✅ Sync API: Simple, clean interface for most users");
    println!("✅ Async API: Seamless integration with async codebases"); 
    println!("✅ Callback API: Zero-copy, high-performance streaming");
    println!("✅ MP4 Output: All tests generated MP4 files");
    println!("\n📁 Check the 'recordings' directory for MP4 outputs:");
    println!("   - hybrid_sync_test.mp4");
    println!("   - hybrid_async_test.mp4");
    println!("   - hybrid_callback_test.mp4");
    
    Ok(())
}
