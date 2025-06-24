use scap::{
    capturer::{Capturer, Options},
    frame::{Frame, FrameType},
};
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥️🎬🎵 Display Video + Audio Recording");
    println!("====================================");

    // Create output directory
    std::fs::create_dir_all("recordings")?;
    
    // Configure for display video + audio recording
    let mut options = Options::default();
    // Don't set a specific target - this will capture the main display
    options.output_type = FrameType::BGRAFrame;
    options.fps = 30;
    
    // Enable system audio capture
    options.capture_system_audio = Some(true);
    options.exclude_current_process_audio = Some(true); // Don't record our own app
    options.audio_sample_rate = Some(48000); // High quality audio
    options.audio_channel_count = Some(2); // Stereo
    
    println!("📹 Recording Configuration:");
    println!("   🎥 Video: Main display @ 30 FPS");
    println!("   🔊 System Audio: 48kHz stereo");
    println!("   💡 Tip: Play some music or video for best audio demonstration");
    
    let mut capturer = Capturer::build(options)?;

    println!("\n🔴 Starting display video + audio capture...");
    capturer.start_capture();

    // Record for 5 seconds to get good samples
    let recording_duration = Duration::from_secs(5);
    let start_time = Instant::now();
    
    println!("📸 Recording {} seconds of display video + audio...", recording_duration.as_secs());
    println!("   💡 Try playing media or making noise to test audio capture!");
    
    let mut video_frames = Vec::new();
    let mut audio_frames = Vec::new();
    
    let mut video_frame_count = 0;
    let mut audio_frame_count = 0;
    let mut last_progress_time = Instant::now();
    
    while start_time.elapsed() < recording_duration {
        match capturer.get_next_frame() {
            Ok(Frame::BGRA(video_frame)) => {
                if video_frames.len() < 5 { // Keep only first few frames for memory
                    video_frames.push(video_frame);
                }
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
            first_frame.width, first_frame.height, video_frame_count);
        
        // Save first frame as raw data for verification
        let video_file = File::create("recordings/display_video_sample.raw")?;
        let mut writer = BufWriter::new(video_file);
        writer.write_all(&first_frame.data)?;
        writer.flush()?;
        println!("   ✅ Sample video frame saved to recordings/display_video_sample.raw");
    }
    
    // Save audio data
    if !audio_frames.is_empty() {
        let mut all_audio_data = Vec::new();
        for frame_data in &audio_frames {
            all_audio_data.extend_from_slice(frame_data);
        }
        
        let audio_file = File::create("recordings/display_audio.raw")?;
        let mut writer = BufWriter::new(audio_file);
        writer.write_all(&all_audio_data)?;
        writer.flush()?;
        
        println!("   🔊 Audio: {} bytes saved to recordings/display_audio.raw", all_audio_data.len());
        
        // Create conversion command
        println!("\n🔧 To convert audio to WAV:");
        println!("ffmpeg -f f32le -ar 48000 -ac 2 -i recordings/display_audio.raw recordings/display_audio.wav -y");
    }
    
    if video_frame_count > 0 && audio_frame_count > 0 {
        println!("\n🎉 Success! Display video + audio capture completed!");
        println!("   📈 Capture rate: {:.1} video FPS, {:.1} audio frames/sec", 
            video_frame_count as f64 / recording_duration.as_secs_f64(),
            audio_frame_count as f64 / recording_duration.as_secs_f64());
        
        println!("\n✨ This demonstrates that scap can simultaneously capture:");
        println!("   🖥️  Full display video content");
        println!("   🔊 High-quality system audio");
        println!("   🎯 The same technique works for window-specific capture");
        println!("   💡 Window capture would isolate specific application content + audio");
    } else {
        println!("\n⚠️  Warning: Limited data captured. Try:");
        println!("   • Playing media or making noise during capture");
        println!("   • Checking system audio permissions");
    }
    
    Ok(())
} 