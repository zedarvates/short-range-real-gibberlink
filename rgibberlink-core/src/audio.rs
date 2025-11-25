
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum AudioMode {
    /// Short-range ultrasonic communication (18-22kHz)
    Ultrasonic,
    /// Standard audio for testing/debugging
    Standard,
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Audio transmission failed: {0}")]
    TransmissionError(String),
    #[error("Audio reception failed: {0}")]
    ReceptionError(String),
    #[error("Audio device not available")]
    DeviceUnavailable,
    #[error("Invalid audio parameters")]
    InvalidParameters,
    #[error("Buffer overflow")]
    BufferOverflow,
    #[error("Timeout")]
    Timeout,
}

/// Audio configuration for different modes
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub buffer_size: usize,
    pub mode: AudioMode,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 1,
            bits_per_sample: 16,
            buffer_size: 1024,
            mode: AudioMode::Ultrasonic,
        }
    }
}

/// Audio buffer for managing transmission/reception
#[derive(Clone)]
struct AudioBuffer {
    data: VecDeque<f32>,
    max_size: usize,
}

impl AudioBuffer {
    fn new(max_size: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    fn push(&mut self, sample: f32) -> Result<(), AudioError> {
        if self.data.len() >= self.max_size {
            return Err(AudioError::BufferOverflow);
        }
        self.data.push_back(sample);
        Ok(())
    }

    fn pop(&mut self) -> Option<f32> {
        self.data.pop_front()
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn clear(&mut self) {
        self.data.clear();
    }
}

/// Cross-platform audio engine with ultrasonic support
pub struct AudioEngine {
    config: AudioConfig,
    transmit_buffer: Arc<Mutex<AudioBuffer>>,
    receive_buffer: Arc<Mutex<AudioBuffer>>,
    is_initialized: bool,
    last_transmission: Instant,
    transmission_timeout: Duration,
}

impl AudioEngine {
    /// Create new audio engine with default configuration
    pub fn new() -> Self {
        Self::with_config(AudioConfig::default())
    }

    /// Create audio engine with custom configuration
    pub fn with_config(config: AudioConfig) -> Self {
        // Use larger buffer sizes for data transmission
        let buffer_size = config.buffer_size.max(65536); // At least 64KB buffer
        Self {
            config,
            transmit_buffer: Arc::new(Mutex::new(AudioBuffer::new(buffer_size))),
            receive_buffer: Arc::new(Mutex::new(AudioBuffer::new(buffer_size))),
            is_initialized: false,
            last_transmission: Instant::now(),
            transmission_timeout: Duration::from_millis(100),
        }
    }

    /// Initialize the audio engine
    pub async fn initialize(&mut self) -> Result<(), AudioError> {
        // Platform-specific initialization would go here
        // For now, this is a software implementation

        match self.config.mode {
            AudioMode::Ultrasonic => {
                // Validate ultrasonic parameters
                if self.config.sample_rate < 44100 {
                    return Err(AudioError::InvalidParameters);
                }
            }
            AudioMode::Standard => {
                // Standard audio validation
            }
        }

        self.is_initialized = true;
        Ok(())
    }

    /// Shutdown the audio engine
    pub async fn shutdown(&mut self) -> Result<(), AudioError> {
        self.is_initialized = false;
        let mut tx_buf = self.transmit_buffer.lock().await;
        let mut rx_buf = self.receive_buffer.lock().await;
        tx_buf.clear();
        rx_buf.clear();
        Ok(())
    }

    /// Force initialization for testing (bypasses async initialization)
    pub fn force_initialize_for_testing(&mut self) {
        self.is_initialized = true;
    }

    /// Send data via audio transmission
    pub async fn send_data(&mut self, data: &[u8]) -> Result<(), AudioError> {
        if !self.is_initialized {
            return Err(AudioError::DeviceUnavailable);
        }

        // Convert data to audio samples
        let audio_samples = self.encode_data_to_audio(data).await?;

        // Queue samples for transmission
        let mut buffer = self.transmit_buffer.lock().await;
        for sample in audio_samples {
            buffer.push(sample)?;
        }

        self.last_transmission = Instant::now();

        // In a real implementation, this would trigger actual audio playback
        // For now, we simulate transmission timing
        tokio::time::sleep(self.transmission_timeout).await;

        Ok(())
    }

    /// Receive data via audio reception
    pub async fn receive_data(&self) -> Result<Vec<u8>, AudioError> {
        if !self.is_initialized {
            return Err(AudioError::DeviceUnavailable);
        }

        // Check if we have received data within timeout
        let timeout = Duration::from_millis(500);
        let start_time = Instant::now();

        loop {
            let buffer = self.receive_buffer.lock().await;
            if !buffer.data.is_empty() {
                break;
            }

            if start_time.elapsed() > timeout {
                return Err(AudioError::Timeout);
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Decode received audio samples back to data
        let mut buffer = self.receive_buffer.lock().await;
        let mut samples = Vec::new();
        while let Some(sample) = buffer.pop() {
            samples.push(sample);
        }

        self.decode_audio_to_data(&samples).await
    }

    /// Check if currently receiving audio data
    pub async fn is_receiving(&self) -> bool {
        if !self.is_initialized {
            return false;
        }

        let buffer = self.receive_buffer.lock().await;
        !buffer.data.is_empty()
    }

    /// Get current audio configuration
    pub fn get_config(&self) -> &AudioConfig {
        &self.config
    }

    /// Update audio configuration
    pub async fn update_config(&mut self, config: AudioConfig) -> Result<(), AudioError> {
        self.config = config;
        // Reinitialize with new config
        self.shutdown().await?;
        self.initialize().await
    }

    /// Simulate receiving audio data (for testing)
    pub async fn simulate_receive(&self, data: &[u8]) -> Result<(), AudioError> {
        let audio_samples = self.encode_data_to_audio(data).await?;
        let mut buffer = self.receive_buffer.lock().await;

        for sample in audio_samples {
            buffer.push(sample)?;
        }

        Ok(())
    }

    /// Encode binary data to audio samples
    async fn encode_data_to_audio(&self, data: &[u8]) -> Result<Vec<f32>, AudioError> {
        let mut samples = Vec::new();

        match self.config.mode {
            AudioMode::Ultrasonic => {
                // Encode data using ultrasonic frequency modulation
                for &byte in data {
                    // Convert each bit to ultrasonic tone
                    for bit in 0..8 {
                        let bit_value = (byte >> (7 - bit)) & 1;
                        let frequency = if bit_value == 1 { 20000.0 } else { 18000.0 }; // 18-20kHz

                        // Generate tone samples
                        let samples_per_bit = (self.config.sample_rate as f32 / 100.0) as usize; // 10ms per bit
                        for i in 0..samples_per_bit {
                            let t = i as f32 / self.config.sample_rate as f32;
                            let sample = (t * frequency * 2.0 * std::f32::consts::PI).sin() * 0.5;
                            samples.push(sample);
                        }
                    }
                }
            }
            AudioMode::Standard => {
                // Simple amplitude modulation for standard audio
                for &byte in data {
                    for bit in 0..8 {
                        let bit_value = (byte >> (7 - bit)) & 1;
                        let amplitude = if bit_value == 1 { 0.8 } else { 0.2 };

                        // Generate samples for this bit
                        let samples_per_bit = (self.config.sample_rate as f32 / 50.0) as usize; // 20ms per bit
                        for _ in 0..samples_per_bit {
                            samples.push(amplitude);
                        }
                    }
                }
            }
        }

        Ok(samples)
    }

    /// Decode audio samples back to binary data
    async fn decode_audio_to_data(&self, samples: &[f32]) -> Result<Vec<u8>, AudioError> {
        let mut data = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        match self.config.mode {
            AudioMode::Ultrasonic => {
                // Decode ultrasonic frequency modulation
                let chunk_size = self.config.sample_rate as usize / 100; // 10ms chunks

                for chunk in samples.chunks(chunk_size) {
                    if chunk.is_empty() {
                        continue;
                    }

                    // Simple frequency detection (in real implementation, use FFT)
                    let avg_amplitude = chunk.iter().map(|s| s.abs()).sum::<f32>() / chunk.len() as f32;
                    let bit = if avg_amplitude > 0.3 { 1 } else { 0 };

                    current_byte = (current_byte << 1) | bit;
                    bit_count += 1;

                    if bit_count == 8 {
                        data.push(current_byte);
                        current_byte = 0;
                        bit_count = 0;
                    }
                }
            }
            AudioMode::Standard => {
                // Decode amplitude modulation
                let chunk_size = self.config.sample_rate as usize / 50; // 20ms chunks

                for chunk in samples.chunks(chunk_size) {
                    if chunk.is_empty() {
                        continue;
                    }

                    let avg_amplitude = chunk.iter().sum::<f32>() / chunk.len() as f32;
                    let bit = if avg_amplitude > 0.5 { 1 } else { 0 };

                    current_byte = (current_byte << 1) | bit;
                    bit_count += 1;

                    if bit_count == 8 {
                        data.push(current_byte);
                        current_byte = 0;
                        bit_count = 0;
                    }
                }
            }
        }

        Ok(data)
    }

    /// Get audio engine status
    pub fn get_status(&self) -> AudioEngineStatus {
        AudioEngineStatus {
            initialized: self.is_initialized,
            mode: self.config.mode.clone(),
            transmit_buffer_size: self.transmit_buffer.try_lock().map(|b| b.len()).unwrap_or(0),
            receive_buffer_size: self.receive_buffer.try_lock().map(|b| b.len()).unwrap_or(0),
            last_transmission: self.last_transmission,
        }
    }
}

/// Audio engine status information
#[derive(Debug, Clone)]
pub struct AudioEngineStatus {
    pub initialized: bool,
    pub mode: AudioMode,
    pub transmit_buffer_size: usize,
    pub receive_buffer_size: usize,
    pub last_transmission: Instant,
}