use scap::{
    capturer::{Capturer, Options},
    frame::{Frame, FrameType},
    Target,
    get_all_targets,
};
use std::{
    fs::File,
    io::{BufWriter, Write, stdout, stdin},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🪟🎬🎵 Window-Specific Video + Audio Recording");
    println!("==============================================");
    println!("💡 This example demonstrates improved window capture with debugging");

    // Create output directory
    std::fs::create_dir_all("recordings")?;
    
    // Get all available targets (windows and displays)
    println!("🔍 Discovering available capture targets...");
    let targets = get_all_targets()?;
    
    // Find windows (filter out displays and system windows)
    let windows: Vec<_> = targets.iter()
        .filter(|target| {
            if let Target::Window(window) = target {
                // Filter out system windows and untitled windows
                !window.title.starts_with("Untitled Window") &&
                !window.title.starts_with("Item-") &&
                !window.title.starts_with("Display Safe Area") &&
                !window.title.contains("Menubar") &&
                !window.title.contains("Dock") &&
                !window.title.contains("Desktop") &&
                !window.title.contains("Wallpaper") &&
                !window.title.contains("window_video_audio_recording") && // Don't capture ourselves
                window.title.len() > 3
            } else {
                false
            }
        })
        .collect();
    
    if windows.is_empty() {
        println!("❌ No suitable windows found to capture.");
        println!("💡 To fix this:");
        println!("   • Open some applications (Safari, Terminal, TextEdit, etc.)");
        println!("   • Make sure the windows have titles and are visible");
        println!("   • Ensure you have screen recording permissions");
        return Ok(());
    }
    
    println!("\n🪟 Available windows for capture:");
    for (i, target) in windows.iter().enumerate() {
        if let Target::Window(window) = target {
            println!("   {}. \"{}\" (ID: {})", i + 1, window.title, window.id);
        }
    }
    
    // Let user choose a window
    print!("\n🎯 Select window number (1-{}): ", windows.len());
    stdout().flush()?;
    
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    
    let choice: usize = input.trim().parse().unwrap_or(1);
    let selected_window = windows.get(choice.saturating_sub(1))
        .unwrap_or(&windows[0])
        .clone();
    
    if let Target::Window(window) = &selected_window {
        println!("✅ Selected window: \"{}\" (ID: {})", window.title, window.id);
    }
    
    // Configure for window-specific video + audio recording
    let mut options = Options::default();
    options.target = Some(selected_window.clone()); // Capture specific window
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    
    // Enable system audio capture - this will capture audio from the selected window
    options.capture_system_audio = Some(true);
    options.exclude_current_process_audio = Some(true); // Don't record our own app
    options.audio_sample_rate = Some(48000); // High quality audio
    options.audio_channel_count = Some(2); // Stereo
    
    println!("\n📹 Recording Configuration:");
    if let Target::Window(window) = &selected_window {
        println!("   🪟 Target: \"{}\"", window.title);
    }
    println!("   🎥 Video: Window content @ 30 FPS");
    println!("   🔊 System Audio: 48kHz stereo");
    println!("   ⚠️  Note: Audio includes all system audio, not just the window");
    
    println!("\n🔧 Creating capturer...");
    let mut capturer = match Capturer::build(options) {
        Ok(c) => {
            println!("✅ Capturer created successfully!");
            c
        }
        Err(e) => {
            println!("❌ Failed to create capturer: {}", e);
            println!("\n🔧 Troubleshooting tips:");
            println!("   • Make sure the selected window is still open and visible");
            println!("   • Check that you have screen recording permissions:");
            println!("     System Preferences → Security & Privacy → Privacy → Screen Recording");
            println!("   • Try selecting a different window");
            println!("   • The window might not support ScreenCaptureKit capture");
            println!("   • Consider using display capture instead of window capture");
            
            // Show some debug info about the selected window
            if let Target::Window(window) = &selected_window {
                println!("\n🔍 Debug info for selected window:");
                println!("   Title: \"{}\"", window.title);
                println!("   ID: {}", window.id);
                println!("   Raw Handle: {:?}", window.raw_handle);
            }
            
            return Err(e.into());
        }
    };

    println!("\n🔴 Starting window video + audio capture...");
    println!("💡 The capture will include detailed debug information");
    
    capturer.start_capture();

    // Record for 8 seconds to get good samples
    let recording_duration = Duration::from_secs(8);
    let start_time = Instant::now();
    
    println!("📸 Recording {} seconds of window video + audio...", recording_duration.as_secs());
    println!("   💡 Try interacting with the window or playing media in it!");
    println!("   🔍 Watch for debug messages in the output");
    
    let mut video_frames = Vec::new();
    let mut audio_frames = Vec::new();
    
    let mut video_frame_count = 0;
    let mut audio_frame_count = 0;
    let mut last_progress_time = Instant::now();
    
    while start_time.elapsed() < recording_duration {
        match capturer.get_next_frame() {
            Ok(Frame::BGRA(video_frame)) => {
                video_frames.push(video_frame);
                video_frame_count += 1;
            }
            Ok(Frame::SystemAudio(audio_frame)) => {
                // Store audio frame data
                audio_frames.push(audio_frame.data.clone());
                audio_frame_count += 1;
            }
            Ok(_) => {
                // Handle other frame types if needed
            }
            Err(e) => {
                // Show error but continue trying
                if last_progress_time.elapsed() >= Duration::from_secs(2) {
                    println!("   ⚠️  Frame capture error: {}", e);
                }
            }
        }
        
        // Show progress every second
        if last_progress_time.elapsed() >= Duration::from_secs(1) {
            let elapsed = start_time.elapsed().as_secs();
            println!("   📊 {}s: {} video frames, {} audio frames", 
                elapsed, video_frame_count, audio_frame_count);
            last_progress_time = Instant::now();
        }
        
        std::thread::sleep(Duration::from_millis(5));
    }

    capturer.stop_capture();
    
    println!("\n💾 Saving captured data...");
    println!("   📊 Total: {} video frames, {} audio frames", video_frame_count, audio_frame_count);
    
    // Analyze the results
    if video_frame_count == 0 && audio_frame_count == 0 {
        println!("\n❌ No data was captured!");
        println!("🔧 This suggests:");
        println!("   • The window capture failed completely");
        println!("   • Screen recording permissions may not be granted");
        println!("   • The selected window may not be capturable");
        println!("   • Check the debug messages above for more details");
    } else if video_frame_count == 0 {
        println!("\n⚠️  Only audio was captured, no video frames");
        println!("🔧 This suggests:");
        println!("   • Window video capture failed but system audio worked");
        println!("   • The window may be minimized or occluded");
        println!("   • Try keeping the window visible and active during capture");
    } else if audio_frame_count == 0 {
        println!("\n⚠️  Only video was captured, no audio frames");
        println!("🔧 This suggests:");
        println!("   • Window video capture worked but audio capture failed");
        println!("   • Audio permissions may not be granted");
        println!("   • Try playing some audio during capture");
    } else {
        println!("\n🎉 Success! Both video and audio were captured!");
    }
    
    // Save video info (just metadata for now)
    if !video_frames.is_empty() {
        let first_frame = &video_frames[0];
        println!("   🎥 Video: {}x{} pixels, {} total frames", 
            first_frame.width, first_frame.height, video_frames.len());
        
        // Save first frame as raw data for verification
        let video_file = File::create("recordings/window_video_sample.raw")?;
        let mut writer = BufWriter::new(video_file);
        writer.write_all(&first_frame.data)?;
        writer.flush()?;
        println!("   ✅ Sample video frame saved to recordings/window_video_sample.raw");
        
        // Calculate average frame rate
        let actual_fps = video_frame_count as f64 / recording_duration.as_secs_f64();
        println!("   📈 Actual video frame rate: {:.1} FPS (target: 30 FPS)", actual_fps);
        
        if actual_fps < 15.0 {
            println!("   ⚠️  Low frame rate detected - this may indicate performance issues");
        }
    }
    
    // Save audio data
    if !audio_frames.is_empty() {
        let mut all_audio_data = Vec::new();
        for frame_data in &audio_frames {
            all_audio_data.extend_from_slice(frame_data);
        }
        
        let audio_file = File::create("recordings/window_audio.raw")?;
        let mut writer = BufWriter::new(audio_file);
        writer.write_all(&all_audio_data)?;
        writer.flush()?;
        
        println!("   🔊 Audio: {} bytes saved to recordings/window_audio.raw", all_audio_data.len());
        
        // Calculate audio statistics
        let audio_duration = all_audio_data.len() as f64 / (48000.0 * 2.0 * 4.0); // 48kHz, stereo, 32-bit float
        println!("   📈 Audio duration: {:.1} seconds", audio_duration);
        
        // Create conversion command
        println!("\n🔧 To convert audio to WAV:");
        println!("ffmpeg -f f32le -ar 48000 -ac 2 -i recordings/window_audio.raw recordings/window_audio.wav -y");
    }
    
    if video_frame_count > 0 && audio_frame_count > 0 {
        println!("\n🎉 Window capture test completed successfully!");
        println!("   📈 Capture rates: {:.1} video FPS, {:.1} audio frames/sec", 
            video_frame_count as f64 / recording_duration.as_secs_f64(),
            audio_frame_count as f64 / recording_duration.as_secs_f64());
        println!("   💡 The improved error handling and debugging should help identify any issues");
    } else {
        println!("\n⚠️  Partial or failed capture detected");
        println!("   📋 Check the debug output above for specific error messages");
        println!("   💡 Try the suggestions provided for troubleshooting");
    }
    
    Ok(())
} 