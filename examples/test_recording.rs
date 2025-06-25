use scap::{
    capturer::{Capturer, Options, WindowAudioOptions},
    frame::{Frame, FrameType},
    Target,
    get_all_targets,
};
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::{Duration, Instant},
};
use tokio::time::sleep;
use anyhow::Result;

async fn test_screen_recording() -> Result<()> {
    println!("\n🖥️  Testing Screen Recording");
    println!("==========================");

    // Create output directory
    std::fs::create_dir_all("recordings")?;
    
    // Configure for display recording
    let mut options = Options::default();
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    options.capture_system_audio = Some(true);
    options.exclude_current_process_audio = Some(true);
    options.audio_sample_rate = Some(48000);
    options.audio_channel_count = Some(2);
    
    println!("📹 Screen Recording Configuration:");
    println!("   🎥 Video: Main display @ 30 FPS");
    println!("   🔊 Audio: 48kHz stereo system audio");
    
    let mut capturer = Capturer::build(options)?;
    println!("✅ Screen capturer created successfully");

    println!("\n🔴 Starting screen recording test...");
    capturer.start_capture().await?;

    // Record for 5 seconds
    let recording_duration = Duration::from_secs(5);
    let start_time = Instant::now();
    
    let mut video_frames = 0;
    let mut audio_frames = 0;
    let mut first_frame_data = None;
    let mut audio_data = Vec::new();
    
    println!("📸 Recording {} seconds of screen content...", recording_duration.as_secs());
    
    while start_time.elapsed() < recording_duration {
        match capturer.get_next_frame().await {
            Ok(Frame::BGRA(frame)) => {
                if first_frame_data.is_none() {
                    println!("   📐 Screen dimensions: {}x{}", frame.width, frame.height);
                    first_frame_data = Some((frame.data.clone(), frame.width, frame.height));
                }
                video_frames += 1;
            }
            Ok(Frame::SystemAudio(frame)) => {
                audio_data.extend_from_slice(&frame.data);
                audio_frames += 1;
            }
            Ok(_) => {}
            Err(e) => {
                println!("   ⚠️  Frame error: {}", e);
            }
        }
        sleep(Duration::from_millis(5)).await;
    }

    capturer.stop_capture().await?;
    
    // Save sample frame
    if let Some((data, width, height)) = first_frame_data {
        let video_file = File::create("recordings/screen_test.raw")?;
        let mut writer = BufWriter::new(video_file);
        writer.write_all(&data)?;
        writer.flush()?;
        
        println!("\n💾 Test Results:");
        println!("   🎥 Video: {}x{} pixels, {} frames captured", width, height, video_frames);
        println!("   🔊 Audio: {} frames captured", audio_frames);
        println!("   📈 Frame rate: {:.1} FPS", video_frames as f64 / recording_duration.as_secs_f64());
        
        if video_frames > 0 && audio_frames > 0 {
            println!("✅ Screen recording test PASSED");
        } else {
            println!("❌ Screen recording test FAILED");
            if video_frames == 0 { println!("   - No video frames captured"); }
            if audio_frames == 0 { println!("   - No audio frames captured"); }
        }
    } else {
        println!("❌ Screen recording test FAILED - No frames captured");
    }

    Ok(())
}

async fn test_window_recording() -> Result<()> {
    println!("\n🪟 Testing Window Recording");
    println!("=========================");

    // Get available windows
    println!("🔍 Looking for windows to capture...");
    let targets = get_all_targets()?;
    
    let windows: Vec<_> = targets.iter()
        .filter_map(|target| {
            if let Target::Window(window) = target {
                Some(window)
            } else {
                None
            }
        })
        .collect();
    
    if windows.is_empty() {
        println!("❌ No suitable windows found for testing");
        println!("💡 Please open some windows (e.g., Terminal, Browser) and try again");
        return Ok(());
    }
    
    // Select the first suitable window
    let test_window = windows[0].clone();
    println!("📝 Selected window for testing: \"{}\"", test_window.title);
    
    // Configure window-specific recording
    let mut options = Options::default();
    options.target = Some(Target::Window(test_window.clone()));
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    
    // Configure window-specific audio
    options.window_audio = Some(WindowAudioOptions {
        capture_window_audio_only: true,
        include_system_notifications: false,
        audio_ducking: false,
    });
    
    // Add window-specific options
    options.exclude_overlapping_windows = Some(true);
    options.window_frame_padding = Some(10.0);
    options.match_window_resolution = Some(true);
    options.include_window_shadow = Some(true);
    
    println!("\n📹 Window Recording Configuration:");
    println!("   🪟 Target: \"{}\"", test_window.title);
    println!("   🎥 Video: Window content @ 30 FPS");
    println!("   🔊 Audio: Window-specific audio only");
    println!("   ✨ Enhanced: Shadow capture, frame padding");
    
    let mut capturer = match Capturer::build(options) {
        Ok(c) => {
            println!("✅ Window capturer created successfully");
            c
        }
        Err(e) => {
            println!("❌ Failed to create window capturer: {}", e);
            return Ok(());
        }
    };

    println!("\n🔴 Starting window recording test...");
    capturer.start_capture().await?;

    // Record for 5 seconds
    let recording_duration = Duration::from_secs(5);
    let start_time = Instant::now();
    
    let mut video_frames = 0;
    let mut audio_frames = 0;
    let mut first_frame_data = None;
    let mut audio_data = Vec::new();
    
    println!("📸 Recording {} seconds of window content...", recording_duration.as_secs());
    println!("💡 Try interacting with the window during recording!");
    
    while start_time.elapsed() < recording_duration {
        match capturer.get_next_frame().await {
            Ok(Frame::BGRA(frame)) => {
                if first_frame_data.is_none() {
                    println!("   📐 Window capture dimensions: {}x{}", frame.width, frame.height);
                    first_frame_data = Some((frame.data.clone(), frame.width, frame.height));
                }
                video_frames += 1;
            }
            Ok(Frame::SystemAudio(frame)) => {
                audio_data.extend_from_slice(&frame.data);
                audio_frames += 1;
            }
            Ok(_) => {}
            Err(e) => {
                println!("   ⚠️  Frame error: {}", e);
            }
        }
        sleep(Duration::from_millis(5)).await;
    }

    capturer.stop_capture().await?;
    
    // Save sample frame
    if let Some((data, width, height)) = first_frame_data {
        let video_file = File::create("recordings/window_test.raw")?;
        let mut writer = BufWriter::new(video_file);
        writer.write_all(&data)?;
        writer.flush()?;
        
        println!("\n💾 Test Results:");
        println!("   🎥 Video: {}x{} pixels, {} frames captured", width, height, video_frames);
        println!("   🔊 Audio: {} frames captured", audio_frames);
        println!("   📈 Frame rate: {:.1} FPS", video_frames as f64 / recording_duration.as_secs_f64());
        
        if video_frames > 0 {
            println!("✅ Window video capture PASSED");
        } else {
            println!("❌ Window video capture FAILED");
        }
        
        if audio_frames > 0 {
            println!("✅ Window audio capture PASSED");
        } else {
            println!("⚠️  Window audio capture produced no frames");
            println!("   💡 This is normal if there was no audio playing in the window");
        }
    } else {
        println!("❌ Window recording test FAILED - No frames captured");
        println!("💡 Possible reasons:");
        println!("   - Window might be minimized or hidden");
        println!("   - Window might not support capture");
        println!("   - Screen recording permissions might be missing");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 SCAP RECORDING TEST SUITE");
    println!("===========================");

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

    // Run tests
    test_screen_recording().await?;
    test_window_recording().await?;

    println!("\n🎉 Testing completed!");
    Ok(())
} 