use scap::{
    capturer::{Capturer, Options},
    frame::{Frame, FrameType},
    Target,
    get_all_targets,
};
use std::{
    fs::File,
    io::{BufWriter, Write, stdin, stdout},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 COMPREHENSIVE SCREEN CAPTURE TEST");
    println!("===================================");
    println!("This test validates both full screen and window-specific audio/video capture");
    println!("and identifies any issues to help consolidate the existing examples.\n");

    // Create output directory
    std::fs::create_dir_all("recordings")?;
    
    // Test 1: Full Screen Capture
    println!("🖥️ TEST 1: Full Screen Audio + Video Capture");
    println!("============================================");
    let display_result = test_display_capture()?;
    
    println!("\n{}\n", "=".repeat(60));
    
    // Test 2: Window-Specific Capture
    println!("🪟 TEST 2: Window-Specific Audio + Video Capture");
    println!("===============================================");
    let window_result = test_window_capture()?;
    
    println!("\n{}\n", "=".repeat(60));
    
    // Summary Report
    println!("📋 COMPREHENSIVE TEST SUMMARY");
    println!("============================");
    
    // Display results
    match display_result {
        TestResult::Success { video_frames, audio_frames, duration, dimensions } => {
            println!("✅ DISPLAY CAPTURE: SUCCESS");
            println!("   🎥 Video: {} frames ({}x{}) over {:.1}s = {:.1} FPS", 
                video_frames, dimensions.0, dimensions.1, duration, 
                video_frames as f64 / duration);
            println!("   🔊 Audio: {} frames = {:.1} frames/sec", 
                audio_frames, audio_frames as f64 / duration);
        }
        TestResult::PartialSuccess { ref issues } => {
            println!("⚠️  DISPLAY CAPTURE: PARTIAL SUCCESS");
            for issue in issues {
                println!("   ⚠️  {}", issue);
            }
        }
        TestResult::Failure { ref error } => {
            println!("❌ DISPLAY CAPTURE: FAILED");
            println!("   💥 Error: {}", error);
        }
    }
    
    // Window results
    match window_result {
        TestResult::Success { video_frames, audio_frames, duration, dimensions } => {
            println!("✅ WINDOW CAPTURE: SUCCESS"); 
            println!("   🎥 Video: {} frames ({}x{}) over {:.1}s = {:.1} FPS", 
                video_frames, dimensions.0, dimensions.1, duration,
                video_frames as f64 / duration);
            println!("   🔊 Audio: {} frames = {:.1} frames/sec", 
                audio_frames, audio_frames as f64 / duration);
        }
        TestResult::PartialSuccess { ref issues } => {
            println!("⚠️  WINDOW CAPTURE: PARTIAL SUCCESS");
            for issue in issues {
                println!("   ⚠️  {}", issue);
            }
        }
        TestResult::Failure { ref error } => {
            println!("❌ WINDOW CAPTURE: FAILED");
            println!("   💥 Error: {}", error);
        }
    }
    
    // Recommendations
    println!("\n🎯 CONSOLIDATION RECOMMENDATIONS");
    println!("===============================");
    
    match (&display_result, &window_result) {
        (TestResult::Success { .. }, TestResult::Success { .. }) => {
            println!("🎉 Both capture modes work perfectly!");
            println!("💡 Recommended consolidation:");
            println!("   • Keep `display_video_audio_recording.rs` for full screen");
            println!("   • Keep `window_video_audio_recording.rs` for window capture");
            println!("   • Archive older/redundant examples:");
            for example in get_redundant_examples() {
                println!("     - {}", example);
            }
        }
        (TestResult::Success { .. }, _) => {
            println!("✅ Display capture works, window capture has issues");
            println!("💡 Focus on fixing window capture implementation");
        }
        (_, TestResult::Success { .. }) => {
            println!("✅ Window capture works, display capture has issues");
            println!("💡 Focus on fixing display capture implementation");
        }
        _ => {
            println!("⚠️  Both capture modes have issues");
            println!("💡 Review system permissions and audio setup");
        }
    }
    
    println!("\n📁 Generated test files in 'recordings/' directory:");
    println!("   • display_test_video.raw & display_test_audio.raw");
    println!("   • window_test_video.raw & window_test_audio.raw");
    println!("   • Use FFmpeg commands shown above to create viewable videos");
    
    Ok(())
}

#[derive(Debug)]
enum TestResult {
    Success {
        video_frames: usize,
        audio_frames: usize,
        duration: f64,
        dimensions: (u32, u32),
    },
    PartialSuccess {
        issues: Vec<String>,
    },
    Failure {
        error: String,
    },
}

fn test_display_capture() -> Result<TestResult, Box<dyn std::error::Error>> {
    println!("🔧 Configuring display capture...");
    
    let mut options = Options::default();
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    options.capture_system_audio = Some(true);
    options.exclude_current_process_audio = Some(true);
    options.audio_sample_rate = Some(48000);
    options.audio_channel_count = Some(2);
    
    println!("📹 Configuration:");
    println!("   🎥 Target: Main display @ 30 FPS");
    println!("   🔊 Audio: 48kHz stereo system audio");
    
    let mut capturer = match Capturer::build(options) {
        Ok(c) => {
            println!("✅ Display capturer created successfully");
            c
        }
        Err(e) => {
            return Ok(TestResult::Failure { 
                error: format!("Failed to create display capturer: {}", e) 
            });
        }
    };
    
    println!("🔴 Starting 5-second display capture test...");
    println!("💡 Try playing a video or music for audio testing!");
    
    capturer.start_capture();
    let recording_duration = Duration::from_secs(5);
    let start_time = Instant::now();
    
    let mut video_frames = Vec::new();
    let mut audio_data = Vec::new();
    let mut video_frame_count = 0;
    let mut audio_frame_count = 0;
    let mut video_width = 0;
    let mut video_height = 0;
    let mut last_progress = Instant::now();
    
    while start_time.elapsed() < recording_duration {
        match capturer.get_next_frame() {
            Ok(Frame::BGRA(video_frame)) => {
                if video_width == 0 {
                    video_width = video_frame.width;
                    video_height = video_frame.height;
                    println!("   📐 Display dimensions: {}x{}", video_width, video_height);
                }
                
                // Store first few frames for verification
                if video_frames.len() < 10 {
                    video_frames.push(video_frame.data.clone());
                }
                video_frame_count += 1;
            }
            Ok(Frame::SystemAudio(audio_frame)) => {
                audio_data.extend_from_slice(&audio_frame.data);
                audio_frame_count += 1;
            }
            Ok(_) => {}
            Err(e) => {
                println!("   ⚠️  Frame error: {}", e);
            }
        }
        
        // Progress update
        if last_progress.elapsed() >= Duration::from_secs(1) {
            let elapsed = start_time.elapsed().as_secs();
            println!("   📊 {}s: {} video, {} audio frames", 
                elapsed, video_frame_count, audio_frame_count);
            last_progress = Instant::now();
        }
        
        std::thread::sleep(Duration::from_millis(5));
    }
    
    capturer.stop_capture();
    let actual_duration = start_time.elapsed().as_secs_f64();
    
    println!("💾 Saving display test results...");
    
    // Save video sample
    if !video_frames.is_empty() {
        let video_file = File::create("recordings/display_test_video.raw")?;
        let mut writer = BufWriter::new(video_file);
        for frame_data in &video_frames {
            writer.write_all(frame_data)?;
        }
        writer.flush()?;
    }
    
    // Save audio
    if !audio_data.is_empty() {
        let audio_file = File::create("recordings/display_test_audio.raw")?;
        let mut writer = BufWriter::new(audio_file);
        writer.write_all(&audio_data)?;
        writer.flush()?;
        
        println!("🔧 FFmpeg command for display test:");
        println!("ffmpeg -f rawvideo -pixel_format bgra -video_size {}x{} -framerate 30 -i recordings/display_test_video.raw -f f32le -ar 48000 -ac 2 -i recordings/display_test_audio.raw -pix_fmt yuv420p -c:a aac recordings/display_test.mp4 -y", video_width, video_height);
    }
    
    // Analyze results
    let mut issues = Vec::new();
    
    if video_frame_count == 0 {
        issues.push("No video frames captured".to_string());
    } else if (video_frame_count as f64 / actual_duration) < 20.0 {
        issues.push(format!("Low video frame rate: {:.1} FPS", video_frame_count as f64 / actual_duration));
    }
    
    if audio_frame_count == 0 {
        issues.push("No audio frames captured - check system audio permissions".to_string());
    }
    
    if issues.is_empty() {
        Ok(TestResult::Success {
            video_frames: video_frame_count,
            audio_frames: audio_frame_count,
            duration: actual_duration,
            dimensions: (video_width as u32, video_height as u32),
        })
    } else if video_frame_count > 0 || audio_frame_count > 0 {
        Ok(TestResult::PartialSuccess { issues })
    } else {
        Ok(TestResult::Failure { 
            error: "No video or audio captured".to_string() 
        })
    }
}

fn test_window_capture() -> Result<TestResult, Box<dyn std::error::Error>> {
    println!("🔧 Finding available windows...");
    
    let targets = get_all_targets()?;
    let windows: Vec<_> = targets.iter()
        .filter(|target| {
            if let Target::Window(window) = target {
                !window.title.is_empty() && 
                !window.title.starts_with("Window Server") &&
                !window.title.starts_with("Dock") &&
                !window.title.starts_with("Control Center") &&
                !window.title.starts_with("SystemUIServer") &&
                !window.title.contains("MenuBar") &&
                !window.title.contains("comprehensive_capture_test") && // Don't capture ourselves
                window.title != "Desktop" &&
                window.title.len() > 3
            } else {
                false
            }
        })
        .collect();
    
    if windows.is_empty() {
        return Ok(TestResult::Failure { 
            error: "No suitable windows found. Open some applications (Safari, Terminal, etc.) and try again.".to_string() 
        });
    }
    
    println!("🪟 Available windows:");
    for (i, target) in windows.iter().enumerate() {
        if let Target::Window(window) = target {
            println!("   {}. \"{}\" (ID: {})", i + 1, window.title, window.id);
        }
    }
    
    // Let user choose or auto-select first window
    print!("\n🎯 Select window number (1-{}) [Enter for auto-select]: ", windows.len());
    stdout().flush()?;
    
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    
    let choice: usize = if input.trim().is_empty() {
        1
    } else {
        input.trim().parse().unwrap_or(1)
    };
    
    let selected_window = windows.get(choice.saturating_sub(1))
        .unwrap_or(&windows[0])
        .clone();
    
    let window_title = if let Target::Window(window) = &selected_window {
        println!("✅ Selected: \"{}\"", window.title);
        window.title.clone()
    } else {
        "Unknown".to_string()
    };
    
    println!("🔧 Configuring window capture...");
    
    let mut options = Options::default();
    options.target = Some(selected_window.clone());
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    options.capture_system_audio = Some(true);
    options.exclude_current_process_audio = Some(true);
    options.audio_sample_rate = Some(48000);
    options.audio_channel_count = Some(2);
    
    println!("📹 Configuration:");
    println!("   🪟 Target: \"{}\"", window_title);  
    println!("   🎥 Video: 30 FPS, window content only");
    println!("   🔊 Audio: 48kHz stereo system audio");
    
    let mut capturer = match Capturer::build(options) {
        Ok(c) => {
            println!("✅ Window capturer created successfully");
            c
        }
        Err(e) => {
            return Ok(TestResult::Failure { 
                error: format!("Failed to create window capturer: {}", e) 
            });
        }
    };
    
    println!("🔴 Starting 5-second window capture test...");
    println!("💡 Try interacting with the selected window!");
    
    capturer.start_capture();
    let recording_duration = Duration::from_secs(5);
    let start_time = Instant::now();
    
    let mut video_frames = Vec::new();
    let mut audio_data = Vec::new();
    let mut video_frame_count = 0;
    let mut audio_frame_count = 0;
    let mut video_width = 0;
    let mut video_height = 0;
    let mut last_progress = Instant::now();
    
    while start_time.elapsed() < recording_duration {
        match capturer.get_next_frame() {
            Ok(Frame::BGRA(video_frame)) => {
                if video_width == 0 {
                    video_width = video_frame.width;
                    video_height = video_frame.height;
                    println!("   📐 Window dimensions: {}x{}", video_width, video_height);
                }
                
                // Store first few frames
                if video_frames.len() < 10 {
                    video_frames.push(video_frame.data.clone());
                }
                video_frame_count += 1;
            }
            Ok(Frame::SystemAudio(audio_frame)) => {
                audio_data.extend_from_slice(&audio_frame.data);
                audio_frame_count += 1;
            }
            Ok(_) => {}
            Err(e) => {
                println!("   ⚠️  Frame error: {}", e);
            }
        }
        
        // Progress update
        if last_progress.elapsed() >= Duration::from_secs(1) {
            let elapsed = start_time.elapsed().as_secs();
            println!("   📊 {}s: {} video, {} audio frames", 
                elapsed, video_frame_count, audio_frame_count);
            last_progress = Instant::now();
        }
        
        std::thread::sleep(Duration::from_millis(5));
    }
    
    capturer.stop_capture();
    let actual_duration = start_time.elapsed().as_secs_f64();
    
    println!("💾 Saving window test results...");
    
    // Save video sample
    if !video_frames.is_empty() {
        let video_file = File::create("recordings/window_test_video.raw")?;
        let mut writer = BufWriter::new(video_file);
        for frame_data in &video_frames {
            writer.write_all(frame_data)?;
        }
        writer.flush()?;
    }
    
    // Save audio
    if !audio_data.is_empty() {
        let audio_file = File::create("recordings/window_test_audio.raw")?;
        let mut writer = BufWriter::new(audio_file);
        writer.write_all(&audio_data)?;
        writer.flush()?;
        
        println!("🔧 FFmpeg command for window test:");
        println!("ffmpeg -f rawvideo -pixel_format bgra -video_size {}x{} -framerate 30 -i recordings/window_test_video.raw -f f32le -ar 48000 -ac 2 -i recordings/window_test_audio.raw -pix_fmt yuv420p -c:a aac recordings/window_test.mp4 -y", video_width, video_height);
    }
    
    // Analyze results
    let mut issues = Vec::new();
    
    if video_frame_count == 0 {
        issues.push("No video frames captured - window might be minimized or hidden".to_string());
    } else if (video_frame_count as f64 / actual_duration) < 20.0 {
        issues.push(format!("Low video frame rate: {:.1} FPS", video_frame_count as f64 / actual_duration));
    }
    
    if audio_frame_count == 0 {
        issues.push("No audio frames captured - check system audio permissions".to_string());
    }
    
    // Check if window dimensions seem reasonable
    if video_width > 0 && video_height > 0 {
        if video_width < 100 || video_height < 100 {
            issues.push(format!("Window dimensions seem too small: {}x{}", video_width, video_height));
        }
    }
    
    if issues.is_empty() {
        Ok(TestResult::Success {
            video_frames: video_frame_count,
            audio_frames: audio_frame_count,
            duration: actual_duration,
            dimensions: (video_width as u32, video_height as u32),
        })
    } else if video_frame_count > 0 || audio_frame_count > 0 {
        Ok(TestResult::PartialSuccess { issues })
    } else {
        Ok(TestResult::Failure { 
            error: "No video or audio captured".to_string() 
        })
    }
}

fn get_redundant_examples() -> Vec<&'static str> {
    vec![
        "audio_test.rs",
        "debug_video_test.rs", 
        "basic_window_demo.rs",
        "simple_window_capture.rs",
        "video_capture_test.rs",
        "recording_test.rs",
        "sample_video_recording.rs",
        "video_with_audio_export.rs",
        "working_window_capture.rs",
        "reliable_window_capture.rs",
        "window_specific_capture.rs",
        "window_test_final.rs",
    ]
} 