use anyhow::{Context, Result};
use core_media_rs::cm_sample_buffer::CMSampleBuffer;
use crate::frame::AudioFrame;

/// Process audio sample buffer from ScreenCaptureKit
pub fn process_audio_sample_buffer(sample_buffer: CMSampleBuffer) -> Result<AudioFrame> {
    // Get audio buffer list from the sample buffer
    let audio_buffer_list = sample_buffer
        .get_audio_buffer_list()
        .context("Failed to get audio buffer list")?;
    
    // Use current time as display time (timing will be handled by the stream)
    let display_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    
    // Extract audio format parameters
    // For ScreenCaptureKit audio, we typically get:
    // - 48kHz sample rate
    // - 2 channels (stereo)
    // - 32-bit float samples
    let sample_rate = 48000u32; // ScreenCaptureKit typically uses 48kHz
    let channel_count = 2u32;   // Typically stereo
    let bits_per_sample = 32u32; // 32-bit float
    let bytes_per_sample = 4usize; // 4 bytes per float32 sample
    
    // Process all audio buffers and combine the data
    let mut combined_data = Vec::new();
    let mut total_sample_count = 0u32;
    
    // Process each buffer in the buffer list
    for buffer_index in 0..audio_buffer_list.num_buffers() {
        let buffer = audio_buffer_list
            .get(buffer_index)
            .context("Failed to get audio buffer")?;
        
        // Get the raw audio data from this buffer
        let audio_data = buffer.data();
        let buffer_channels = buffer.number_channels as u32;
        
        // Validate buffer data
        if audio_data.is_empty() {
            continue;
        }
        
        // Calculate the number of sample frames in this buffer
        // Each sample frame contains one sample for each channel
        let bytes_per_frame = bytes_per_sample * buffer_channels as usize;
        let sample_frames_in_buffer = audio_data.len() / bytes_per_frame;
        
        // Ensure we have complete sample frames
        let valid_data_size = sample_frames_in_buffer * bytes_per_frame;
        if valid_data_size > 0 {
            combined_data.extend_from_slice(&audio_data[..valid_data_size]);
            total_sample_count += sample_frames_in_buffer as u32;
        }
        
        // Debug info for first few buffers
        if buffer_index < 3 {
            println!("Audio buffer {}: {} channels, {} bytes, {} sample frames", 
                buffer_index, buffer_channels, audio_data.len(), sample_frames_in_buffer);
        }
    }
    
    // Validate final audio data
    if combined_data.is_empty() {
        return Err(anyhow::anyhow!("No valid audio data found in sample buffer"));
    }
    
    // Additional validation: ensure data size matches expected format
    let expected_total_bytes = total_sample_count as usize * bytes_per_sample * channel_count as usize;
    if combined_data.len() != expected_total_bytes {
        println!("Warning: Audio data size mismatch. Expected: {}, Got: {}", 
            expected_total_bytes, combined_data.len());
        
        // Truncate to expected size to avoid format issues
        if combined_data.len() > expected_total_bytes {
            combined_data.truncate(expected_total_bytes);
        }
    }
    
    Ok(AudioFrame {
        display_time,
        sample_rate,
        channel_count,
        sample_count: total_sample_count,
        data: combined_data,
        bits_per_sample,
        source: crate::frame::AudioSource::System,
    })
}

/// Simplified audio processing that closely follows the reference implementation
pub fn process_audio_sample_buffer_simple(sample_buffer: CMSampleBuffer) -> Result<AudioFrame> {
    // Get audio buffer list from the sample buffer
    let audio_buffer_list = sample_buffer
        .get_audio_buffer_list()
        .context("Failed to get audio buffer list")?;
    
    // Use current time as display time
    let display_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    
    let num_buffers = audio_buffer_list.num_buffers();
    if num_buffers == 0 {
        return Err(anyhow::anyhow!("No audio buffers found"));
    }
    
    // Collect all buffers first to understand the structure
    let mut buffers = Vec::new();
    let mut total_channels = 0u32;
    let mut max_buffer_size = 0usize;
    
    for buffer_index in 0..num_buffers {
        let buffer = audio_buffer_list
            .get(buffer_index)
            .with_context(|| format!("Failed to get audio buffer {}", buffer_index))?;
        
        let audio_data = buffer.data();
        let buffer_channels = buffer.number_channels as u32;
        
        if audio_data.is_empty() {
            continue;
        }
        
        buffers.push((audio_data, buffer_channels));
        total_channels += buffer_channels;
        max_buffer_size = max_buffer_size.max(audio_data.len());
        
        // Debug output like reference
        println!("{}: channels={}, size={}", 
            buffer_index, buffer_channels, audio_data.len());
    }
    
    if buffers.is_empty() {
        return Err(anyhow::anyhow!("No valid audio data found in any buffer"));
    }
    
    // Determine the actual audio format
    let bytes_per_sample = 4usize; // 32-bit float
    let sample_frames_per_buffer = max_buffer_size / bytes_per_sample;
    
    let final_audio_data;
    let final_channel_count;
    
    if num_buffers == 2 && buffers.iter().all(|(_, channels)| *channels == 1) {
        // Stereo audio split into separate left/right channel buffers
        // Need to interleave them
        println!("Audio format detected: {} separate mono buffers (stereo)", num_buffers);
        
        let left_data = buffers[0].0;
        let right_data = buffers[1].0;
        
        // Ensure both buffers have the same size
        let min_size = left_data.len().min(right_data.len());
        let aligned_size = (min_size / bytes_per_sample) * bytes_per_sample;
        
        // Interleave the left and right channels
        let mut interleaved_data = Vec::with_capacity(aligned_size * 2);
        
        // Convert bytes to f32 samples and interleave
        let left_samples = bytemuck::cast_slice::<u8, f32>(&left_data[..aligned_size]);
        let right_samples = bytemuck::cast_slice::<u8, f32>(&right_data[..aligned_size]);
        
        for (left, right) in left_samples.iter().zip(right_samples.iter()) {
            interleaved_data.extend_from_slice(&left.to_le_bytes());
            interleaved_data.extend_from_slice(&right.to_le_bytes());
        }
        
        final_audio_data = interleaved_data;
        final_channel_count = 2;
        
        println!("Interleaved stereo: {} bytes -> {} bytes", 
            aligned_size * 2, final_audio_data.len());
    } else if num_buffers == 1 {
        // Single buffer - use as-is
        println!("Audio format detected: single buffer with {} channels", buffers[0].1);
        final_audio_data = buffers[0].0.to_vec();
        final_channel_count = buffers[0].1;
    } else {
        // Multiple buffers - concatenate them (fallback)
        println!("Audio format detected: {} buffers, concatenating", num_buffers);
        let mut all_data = Vec::new();
        for (data, _) in &buffers {
            all_data.extend_from_slice(data);
        }
        final_audio_data = all_data;
        final_channel_count = total_channels;
    }
    
    let total_sample_frames = final_audio_data.len() / (bytes_per_sample * final_channel_count as usize);
    
    println!("Final audio: {} channels, {} bytes, {} sample frames", 
        final_channel_count, final_audio_data.len(), total_sample_frames);
    
    Ok(AudioFrame {
        display_time,
        sample_rate: 48000,
        channel_count: final_channel_count,
        sample_count: total_sample_frames as u32,
        data: final_audio_data,
        bits_per_sample: 32,
        source: crate::frame::AudioSource::System,
    })
}

/// Enhanced audio processing with format validation
pub fn process_audio_sample_buffer_enhanced(sample_buffer: CMSampleBuffer) -> Result<AudioFrame> {
    let audio_buffer_list = sample_buffer
        .get_audio_buffer_list()
        .context("Failed to get audio buffer list")?;
    
    // Use current time as display time (ScreenCaptureKit handles timing)
    let display_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    
    // Process buffers with better error handling
    let mut all_audio_data = Vec::new();
    let detected_sample_rate = 48000u32;
    let mut detected_channels = 2u32;
    let mut total_samples = 0u32;
    
    for buffer_index in 0..audio_buffer_list.num_buffers() {
        let buffer = audio_buffer_list
            .get(buffer_index)
            .with_context(|| format!("Failed to get audio buffer {}", buffer_index))?;
        
        let audio_data = buffer.data();
        let buffer_channels = buffer.number_channels as u32;
        
        if audio_data.is_empty() {
            continue;
        }
        
        // Update detected format from first valid buffer
        if buffer_index == 0 {
            detected_channels = buffer_channels;
        }
        
        // Validate channel consistency
        if buffer_channels != detected_channels {
            println!("Warning: Channel count mismatch in buffer {}. Expected: {}, Got: {}", 
                buffer_index, detected_channels, buffer_channels);
        }
        
        // Process audio data with format validation
        let bytes_per_sample = 4usize; // float32
        let bytes_per_frame = bytes_per_sample * buffer_channels as usize;
        
        if audio_data.len() % bytes_per_frame != 0 {
            println!("Warning: Audio buffer {} size not aligned to frame boundary", buffer_index);
            // Align to frame boundary
            let aligned_size = (audio_data.len() / bytes_per_frame) * bytes_per_frame;
            all_audio_data.extend_from_slice(&audio_data[..aligned_size]);
            total_samples += (aligned_size / bytes_per_frame) as u32;
        } else {
            all_audio_data.extend_from_slice(audio_data);
            total_samples += (audio_data.len() / bytes_per_frame) as u32;
        }
    }
    
    if all_audio_data.is_empty() {
        return Err(anyhow::anyhow!("No audio data found in any buffer"));
    }
    
    Ok(AudioFrame {
        display_time,
        sample_rate: detected_sample_rate,
        channel_count: detected_channels,
        sample_count: total_samples,
        data: all_audio_data,
        bits_per_sample: 32,
        source: crate::frame::AudioSource::System,
    })
}

/// Placeholder for microphone capture functionality
pub struct MicrophoneCapture {
    // Future implementation for microphone capture
}

impl MicrophoneCapture {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn start_capture(&self) -> Result<()> {
        // TODO: Implement microphone capture using AVAudioEngine or similar
        Ok(())
    }
    
    pub fn stop_capture(&self) -> Result<()> {
        // TODO: Implement microphone capture stop
        Ok(())
    }
} 