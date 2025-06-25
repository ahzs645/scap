// This program is just a testing application
// Refer to `lib.rs` for the library source code

use scap::{
    capturer::{Area, Capturer, Options, Point, Size, Resolution},
    frame::Frame,
};
use std::process;

fn main() {
    // Check if the platform is supported
    if !scap::is_supported() {
        println!("❌ Platform not supported");
        return;
    }

    // Check if we have permission to capture screen
    // If we don't, request it.
    if !scap::has_permission() {
        println!("❌ Permission not granted. Requesting permission...");
        if !scap::request_permission() {
            println!("❌ Permission denied");
            return;
        }
    }

    // Get recording targets
    let targets = scap::get_all_targets();
    match targets {
        Ok(targets) => {
            println!("Found {} capture targets", targets.len());
            for (i, target) in targets.iter().enumerate() {
                match target {
                    scap::Target::Display(display) => {
                        println!("  Display {}: {} ({}x{})", i, display.title, display.width, display.height);
                    }
                    scap::Target::Window(window) => {
                        println!("  Window {}: {} ({}x{})", i, window.title, window.width, window.height);
                    }
                }
            }
        }
        Err(e) => {
            println!("Failed to get targets: {}", e);
        }
    }

    // Create Options
    let options = Options {
        fps: 60,
        show_cursor: true,
        show_highlight: true,
        excluded_targets: None,
        output_type: scap::frame::FrameType::BGRAFrame,
        output_resolution: Resolution::_720p,
        crop_area: Some(Area {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 500.0,
                height: 500.0,
            },
        }),
        ..Default::default()
    };

    // Create Recorder with options
    let mut recorder = Capturer::build(options).unwrap_or_else(|err| {
        println!("Problem with building Capturer: {err}");
        process::exit(1);
    });

    // Start Capture
    println!("Starting capture...");
    if let Err(e) = recorder.start_capture_sync() {
        println!("Failed to start capture: {}", e);
        return;
    }

    // Capture 100 frames
    let mut start_time: u64 = 0;
    for i in 0..100 {
        let frame = match recorder.get_next_frame_sync() {
            Ok(frame) => frame,
            Err(e) => {
                println!("Error getting frame {}: {}", i, e);
                continue;
            }
        };

        match frame {
            Frame::YUVFrame(frame) => {
                println!(
                    "Received YUV frame {} of width {} and height {} and pts {}",
                    i, frame.width, frame.height, frame.display_time
                );
            }
            Frame::BGR0(frame) => {
                println!(
                    "Received BGR0 frame of width {} and height {}",
                    frame.width, frame.height
                );
            }
            Frame::RGB(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "Received RGB frame {} of width {} and height {} and time {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );
            }
            Frame::SystemAudio(frame) => {
                println!(
                    "Received system audio frame {} with {} samples at {} Hz",
                    i, frame.sample_count, frame.sample_rate
                );
            }
            Frame::MicrophoneAudio(frame) => {
                println!(
                    "Received microphone audio frame {} with {} samples at {} Hz",
                    i, frame.sample_count, frame.sample_rate
                );
            }
            Frame::BGRA(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "Received BGRA frame {} of width {} and height {} and time {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );
            }
            Frame::RGBx(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "Received RGBx frame {} of width {} and height {} and time {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );
            }
            Frame::XBGR(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "Received XBGR frame {} of width {} and height {} and time {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );
            }
            Frame::BGRx(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "Received BGRx frame {} of width {} and height {} and time {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );
            }
        }
    }

    // Stop Capture
    println!("Stopping capture...");
    if let Err(e) = recorder.stop_capture_sync() {
        println!("Failed to stop capture: {}", e);
    }
    
    println!("Capture completed successfully!");
}