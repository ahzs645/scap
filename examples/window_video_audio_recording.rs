use scap::{
    capturer::{Capturer, Options},
    frame::{Frame, FrameType},
    Target,
    get_all_targets,
};
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🪟🎬🎵 Window-Specific Video + Audio Recording");
    println!("==============================================");

    // Create output directory
    std::fs::create_dir_all("recordings")?;
    
    // Get all available targets (windows and displays)
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
                window.title.len() > 3
            } else {
                false
            }
        })
        .collect();
    
    if windows.is_empty() {
        println!("❌ No suitable windows found to capture. Please open some applications with named windows.");
        return Ok(());
    }
    
    println!("🪟 Available windows:");
    for (i, target) in windows.iter().enumerate() {
        if let Target::Window(window) = target {
            println!("   {}. {} (ID: {})", i + 1, window.title, window.id);
        }
    }
    
    // For demo, let's use the first suitable window, but in practice you'd let user choose
    let selected_window = windows[0].clone();
    
    if let Target::Window(window) = &selected_window {
        println!("\n🎯 Selected window: \"{}\"", window.title);
    }
    
    // Configure for window-specific video + audio recording
    let mut options = Options::default();
    options.target = Some(selected_window); // Capture specific window
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    
    // Enable system audio capture - this will capture audio from the selected window
    options.capture_system_audio = Some(true);
    options.exclude_current_process_audio = Some(true); // Don't record our own app
    options.audio_sample_rate = Some(48000); // High quality audio
    options.audio_channel_count = Some(2); // Stereo
    
    println!("\n📹 Recording Configuration:");
    println!("   🎥 Video: Window content @ 30 FPS");
    println!("   🔊 System Audio: 48kHz stereo from window");
    println!("   ⚠️  Note: Audio includes all system audio, not just the window");
    println!("   💡 Tip: For best results, focus on the target window and play media");
    
    let mut capturer = Capturer::build(options)?;

    println!("\n🔴 Starting window video + audio capture...");
    capturer.start_capture();

    // Record for 8 seconds to get good samples
    let recording_duration = Duration::from_secs(8);
    let start_time = Instant::now();
    
    println!("📸 Recording {} seconds of window video + audio...", recording_duration.as_secs());
    println!("   💡 Try interacting with the window or playing media in it!");
    
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
            Err(_) => {
                // Continue trying
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
        
        // Create conversion command
        println!("\n🔧 To convert audio to WAV:");
        println!("ffmpeg -f f32le -ar 48000 -ac 2 -i recordings/window_audio.raw recordings/window_audio.wav -y");
    }
    
    if video_frame_count > 0 && audio_frame_count > 0 {
        println!("\n🎉 Success! Window video + audio capture completed!");
        println!("   📈 Capture rate: {:.1} video FPS, {:.1} audio frames/sec", 
            video_frame_count as f64 / recording_duration.as_secs_f64(),
            audio_frame_count as f64 / recording_duration.as_secs_f64());
    } else {
        println!("\n⚠️  Warning: Limited data captured. Try:");
        println!("   • Ensuring the target window is visible and active");
        println!("   • Playing media or making noise during capture");
        println!("   • Checking system audio permissions");
    }
    
    Ok(())
} 