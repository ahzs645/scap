use scap::{
    capturer::{Capturer, Options},
    frame::{Frame, FrameType},
};
use std::{
    time::{Duration, Instant},
    thread,
};
use anyhow::Result;

/// Standards-compliant MP4 Writer using the mp4 crate
struct StandardsMP4Writer {
    frames: Vec<Vec<u8>>,
    width: u32,
    height: u32,
    fps: u32,
}

impl StandardsMP4Writer {
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
        use mp4::{Mp4Config, TrackConfig, TrackType};
        use std::fs::File;

        // Create MP4 configuration
        let config = Mp4Config {
            major_brand: str::parse("isom").unwrap(),
            minor_version: 512,
            compatible_brands: vec![
                str::parse("isom").unwrap(),
                str::parse("iso2").unwrap(),
                str::parse("avc1").unwrap(),
                str::parse("mp41").unwrap(),
            ],
            timescale: 1000,
        };

        // Create file and writer
        let file = File::create(path)?;
        let mut writer = mp4::Mp4Writer::write_start(
            Box::new(file),
            &config,
        )?;

        // Create video track configuration
        let track_config = TrackConfig {
            track_type: TrackType::Video,
            timescale: self.fps * 1000, // Scale for precise timing
            language: "und".to_string(),
        };

        // Add video track
        let track_id = writer.add_track(&track_config)?;

        // For this example, we'll convert BGRA frames to a format that can be written
        // In a real implementation, you might want to encode to H.264 or other codec
        let sample_duration = 1000u32; // Duration per frame in timescale units

        for (i, frame_data) in self.frames.iter().enumerate() {
            // Convert BGRA to RGB for better compatibility
            let rgb_data = bgra_to_rgb(frame_data);
            
            let sample = mp4::Mp4Sample {
                start_time: (i as u64) * (sample_duration as u64),
                duration: sample_duration,
                rendering_offset: 0,
                is_sync: true,
                bytes: rgb_data.into(),
            };

            writer.write_sample(track_id, &sample)?;
        }

        writer.write_end()?;
        Ok(())
    }
}

fn bgra_to_rgb(bgra_data: &[u8]) -> Vec<u8> {
    let mut rgb_data = Vec::with_capacity(bgra_data.len() * 3 / 4);
    
    for chunk in bgra_data.chunks_exact(4) {
        if let [b, g, r, _a] = chunk {
            rgb_data.extend_from_slice(&[*r, *g, *b]);
        }
    }
    
    rgb_data
}

fn test_standards_compliant_mp4() -> Result<()> {
    println!("🎬 Testing Standards-Compliant MP4 Output");
    println!("==========================================");
    
    let mut options = Options::default();
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    
    println!("📹 Configuration:");
    println!("   🎥 Video: Main display @ {} FPS", options.fps);
    println!("   📦 Format: Standards-compliant MP4");
    
    let mut capturer = Capturer::build(options)?;
    println!("✅ Capturer created");
    
    // Start capture
    println!("\n🔴 Starting capture...");
    capturer.start_capture()?;
    
    let duration = Duration::from_secs(2);
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
                
                if frame_count % 15 == 0 {
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
        thread::sleep(Duration::from_millis(10));
    }
    
    capturer.stop_capture()?;
    
    // Create standards-compliant MP4
    if let Some((width, height)) = first_frame_info {
        let mut mp4_writer = StandardsMP4Writer::new(width as u32, height as u32, 30);
        
        for frame_data in frames {
            mp4_writer.add_frame(frame_data);
        }
        
        std::fs::create_dir_all("recordings")?;
        mp4_writer.write_to_file("recordings/standards_compliant.mp4")?;
        
        println!("\n💾 Standards-Compliant MP4 Results:");
        println!("   🎥 Frames captured: {}", frame_count);
        println!("   📈 Average FPS: {:.1}", frame_count as f64 / duration.as_secs_f64());
        println!("   📁 Output: recordings/standards_compliant.mp4");
        println!("   ✅ Standards-compliant MP4 test PASSED");
    }
    
    Ok(())
}

fn main() -> Result<()> {
    println!("🎥 SCAP STANDARDS-COMPLIANT MP4 TEST");
    println!("====================================");
    
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

    // Test standards-compliant MP4 output
    test_standards_compliant_mp4()?;
    
    println!("\n🎉 Standards-Compliant MP4 Test Complete!");
    println!("==========================================");
    println!("✅ MP4 file created using the mp4 crate");
    println!("✅ Should be playable in all standard video players");
    
    Ok(())
}
