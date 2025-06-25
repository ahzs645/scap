use anyhow::{Context, Result};
use core_media_rs::cm_sample_buffer::CMSampleBuffer;
use crate::frame::{AudioFrame, AudioSource};
use crate::capturer::frame_pool::FramePool;

/// Audio format configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: Option<u32>,
    pub channel_count: Option<u32>,
    pub buffer_duration: std::time::Duration,
    pub enable_echo_cancellation: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: None, // Will be detected from stream
            channel_count: None, // Will be detected from stream
            buffer_duration: std::time::Duration::from_millis(20),
            enable_echo_cancellation: false,
        }
    }
}

/// Process audio sample buffer from ScreenCaptureKit with proper format detection
pub fn process_audio_sample_buffer(sample_buffer: CMSampleBuffer) -> Result<AudioFrame> {
    // Get audio format description from the sample buffer
    let format_description = sample_buffer
        .get_format_description()
        .context("Failed to get format description")?;
    
    let audio_format = format_description
        .get_audio_stream_basic_description()
        .context("Failed to get audio stream description")?;
    
    // Extract actual format parameters
    let sample_rate = audio_format.sample_rate as u32;
    let channel_count = audio_format.channels_per_frame as u32;
    let bits_per_sample = audio_format.bits_per_channel as u32;
    let bytes_per_sample = (bits_per_sample / 8) as usize;
    
    // Get audio buffer list
    let audio_buffer_list = sample_buffer
        .get_audio_buffer_list()
        .context("Failed to get audio buffer list")?;
    
    // Get timing info
    let display_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    
    // Process all audio buffers
    let mut combined_data = Vec::new();
    let mut total_sample_count = 0u32;
    
    // Handle different buffer layouts
    match audio_buffer_list.num_buffers() {
        0 => return Err(anyhow::anyhow!("No audio buffers found")),
        
        1 => {
            // Single interleaved buffer
            let buffer = audio_buffer_list.get(0).context("Failed to get audio buffer")?;
            let audio_data = buffer.data();
            
            if audio_data.is_empty() {
                return Err(anyhow::anyhow!("Empty audio buffer"));
            }
            
            // Validate buffer format
            let buffer_channels = buffer.number_channels as u32;
            if buffer_channels != channel_count {
                log::warn!(
                    "Channel count mismatch. Format: {}, Buffer: {}",
                    channel_count,
                    buffer_channels
                );
            }
            
            // Calculate frame count
            let bytes_per_frame = bytes_per_sample * channel_count as usize;
            let sample_frames = audio_data.len() / bytes_per_frame;
            
            combined_data = audio_data.to_vec();
            total_sample_count = sample_frames as u32;
        }
        
        2 => {
            // Likely separate left/right channels
            let left = audio_buffer_list.get(0).context("Failed to get left buffer")?;
            let right = audio_buffer_list.get(1).context("Failed to get right buffer")?;
            
            if left.number_channels != 1 || right.number_channels != 1 {
                return Err(anyhow::anyhow!("Expected mono buffers for stereo audio"));
            }
            
            let left_data = left.data();
            let right_data = right.data();
            
            if left_data.is_empty() || right_data.is_empty() {
                return Err(anyhow::anyhow!("Empty channel buffer"));
            }
            
            // Ensure equal buffer sizes
            let samples_per_channel = left_data.len() / bytes_per_sample;
            if left_data.len() != right_data.len() {
                log::warn!(
                    "Channel size mismatch. L: {}, R: {}",
                    left_data.len(),
                    right_data.len()
                );
            }
            
            // Interleave channels
            combined_data = Vec::with_capacity(samples_per_channel * bytes_per_sample * 2);
            
            for i in 0..samples_per_channel {
                let start = i * bytes_per_sample;
                let end = start + bytes_per_sample;
                
                combined_data.extend_from_slice(&left_data[start..end]);
                combined_data.extend_from_slice(&right_data[start..end]);
            }
            
            total_sample_count = samples_per_channel as u32;
        }
        
        n => {
            // Multi-channel audio
            let mut max_samples = 0;
            let mut channel_data = Vec::with_capacity(n as usize);
            
            for i in 0..n {
                let buffer = audio_buffer_list
                    .get(i)
                    .with_context(|| format!("Failed to get buffer {}", i))?;
                
                let data = buffer.data();
                if data.is_empty() {
                    continue;
                }
                
                let samples = data.len() / bytes_per_sample;
                max_samples = max_samples.max(samples);
                channel_data.push(data);
            }
            
            // Interleave all channels
            combined_data = Vec::with_capacity(max_samples * bytes_per_sample * n as usize);
            
            for sample_idx in 0..max_samples {
                for channel in &channel_data {
                    let start = sample_idx * bytes_per_sample;
                    if start + bytes_per_sample <= channel.len() {
                        combined_data.extend_from_slice(&channel[start..start + bytes_per_sample]);
                    } else {
                        // Pad missing samples with silence
                        combined_data.extend_from_slice(&vec![0; bytes_per_sample]);
                    }
                }
            }
            
            total_sample_count = max_samples as u32;
        }
    }
    
    Ok(AudioFrame {
        display_time,
        sample_rate,
        channel_count,
        sample_count: total_sample_count,
        data: combined_data,
        bits_per_sample,
        source: AudioSource::System,
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
pub fn process_audio_sample_buffer_enhanced(
    sample_buffer: CMSampleBuffer,
    frame_pool: &FramePool,
) -> Result<AudioFrame> {
    // Get audio format description from the sample buffer
    let format_description = sample_buffer
        .get_format_description()
        .context("Failed to get format description")?;
    
    let audio_format = format_description
        .get_audio_stream_basic_description()
        .context("Failed to get audio stream description")?;
    
    // Extract actual format parameters
    let sample_rate = audio_format.sample_rate as u32;
    let channel_count = audio_format.channels_per_frame as u32;
    let bits_per_sample = audio_format.bits_per_channel as u32;
    let bytes_per_sample = (bits_per_sample / 8) as usize;
    
    // Get audio buffer list
    let audio_buffer_list = sample_buffer
        .get_audio_buffer_list()
        .context("Failed to get audio buffer list")?;
    
    // Get timing info
    let display_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    
    // Handle different buffer layouts
    match audio_buffer_list.num_buffers() {
        0 => return Err(anyhow::anyhow!("No audio buffers found")),
        
        1 => {
            // Single interleaved buffer
            let buffer = audio_buffer_list.get(0).context("Failed to get audio buffer")?;
            let audio_data = buffer.data();
            
            if audio_data.is_empty() {
                return Err(anyhow::anyhow!("Empty audio buffer"));
            }
            
            // Validate buffer format
            let buffer_channels = buffer.number_channels as u32;
            if buffer_channels != channel_count {
                log::warn!(
                    "Channel count mismatch. Format: {}, Buffer: {}",
                    channel_count,
                    buffer_channels
                );
            }
            
            // Calculate frame count
            let bytes_per_frame = bytes_per_sample * channel_count as usize;
            let sample_frames = audio_data.len() / bytes_per_frame;
            
            // Get a buffer from the pool
            let mut combined_data = frame_pool.get_audio_buffer(audio_data.len());
            combined_data.extend_from_slice(audio_data);
            
            Ok(AudioFrame {
                display_time,
                sample_rate,
                channel_count,
                sample_count: sample_frames as u32,
                data: combined_data,
                bits_per_sample,
                source: AudioSource::System,
            })
        }
        
        2 => {
            // Likely separate left/right channels
            let left = audio_buffer_list.get(0).context("Failed to get left buffer")?;
            let right = audio_buffer_list.get(1).context("Failed to get right buffer")?;
            
            if left.number_channels != 1 || right.number_channels != 1 {
                return Err(anyhow::anyhow!("Expected mono buffers for stereo audio"));
            }
            
            let left_data = left.data();
            let right_data = right.data();
            
            if left_data.is_empty() || right_data.is_empty() {
                return Err(anyhow::anyhow!("Empty channel buffer"));
            }
            
            // Ensure equal buffer sizes
            let samples_per_channel = left_data.len() / bytes_per_sample;
            if left_data.len() != right_data.len() {
                log::warn!(
                    "Channel size mismatch. L: {}, R: {}",
                    left_data.len(),
                    right_data.len()
                );
            }
            
            // Get a buffer from the pool for interleaved data
            let mut combined_data = frame_pool.get_audio_buffer(samples_per_channel * bytes_per_sample * 2);
            
            // Interleave channels
            for i in 0..samples_per_channel {
                let start = i * bytes_per_sample;
                let end = start + bytes_per_sample;
                
                combined_data.extend_from_slice(&left_data[start..end]);
                combined_data.extend_from_slice(&right_data[start..end]);
            }
            
            Ok(AudioFrame {
                display_time,
                sample_rate,
                channel_count: 2,
                sample_count: samples_per_channel as u32,
                data: combined_data,
                bits_per_sample,
                source: AudioSource::System,
            })
        }
        
        n => {
            // Multi-channel audio
            let mut max_samples = 0;
            let mut channel_data = Vec::with_capacity(n as usize);
            
            for i in 0..n {
                let buffer = audio_buffer_list
                    .get(i)
                    .with_context(|| format!("Failed to get buffer {}", i))?;
                
                let data = buffer.data();
                if data.is_empty() {
                    continue;
                }
                
                let samples = data.len() / bytes_per_sample;
                max_samples = max_samples.max(samples);
                channel_data.push(data);
            }
            
            // Get a buffer from the pool for interleaved data
            let mut combined_data = frame_pool.get_audio_buffer(max_samples * bytes_per_sample * n as usize);
            
            // Interleave all channels
            for sample_idx in 0..max_samples {
                for channel in &channel_data {
                    let start = sample_idx * bytes_per_sample;
                    if start + bytes_per_sample <= channel.len() {
                        combined_data.extend_from_slice(&channel[start..start + bytes_per_sample]);
                    } else {
                        // Pad missing samples with silence
                        combined_data.extend_from_slice(&vec![0; bytes_per_sample]);
                    }
                }
            }
            
            Ok(AudioFrame {
                display_time,
                sample_rate,
                channel_count: n as u32,
                sample_count: max_samples as u32,
                data: combined_data,
                bits_per_sample,
                source: AudioSource::System,
            })
        }
    }
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