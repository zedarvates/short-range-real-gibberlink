//! # Laser Engine Module
//!
//! High-speed optical data transmission using laser diodes with multiple modulation schemes.
//! Supports visible and IR lasers, OOK/PWM modulation, QR projection, and photodiode/camera reception.

use crate::visual::{VisualEngine, VisualError, VisualPayload};
use crate::optical_ecc::{OpticalECC, OpticalQualityMetrics, AdaptiveECCConfig};
use crate::range_detector::{RangeDetector, RangeDetectorCategory, RangeEnvironmentalConditions, RangeMeasurement};
use crate::security::WeatherCondition;
use reed_solomon_erasure::galois_8::ReedSolomon;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

#[cfg(target_os = "android")]
use std::os::raw::{c_char, c_int};

#[cfg(target_os = "android")]
extern "C" {
    fn laser_init_hardware() -> c_int;
    fn laser_set_power(power_mw: f32) -> c_int;
    fn laser_get_photodiode_reading() -> f32;
    fn laser_get_camera_frame(buffer: *mut u8, size: usize) -> c_int;
    fn laser_set_alignment(x: f32, y: f32) -> c_int;
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum LaserError {
    #[error("Hardware not available")]
    HardwareUnavailable,
    #[error("Invalid modulation scheme")]
    InvalidModulation,
    #[error("Transmission failed")]
    TransmissionFailed,
    #[error("Reception failed")]
    ReceptionFailed,
    #[error("Safety violation")]
    SafetyViolation,
    #[error("Alignment lost")]
    AlignmentLost,
    #[error("Data corruption")]
    DataCorruption,
    #[error("Timeout")]
    Timeout,
    #[error("Visual engine error: {0}")]
    VisualError(#[from] VisualError),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LaserType {
    Visible,
    Infrared,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ModulationScheme {
    Ook,        // On-Off Keying - Best for close range, high speed
    Pwm,        // Pulse Width Modulation - Good for medium range, balanced
    QrProjection, // Dynamic QR code projection - Best for long range, robust
    Fsk,        // Frequency Shift Keying - Alternative for IR lasers
    Manchester, // Manchester encoding - Self-clocking, good for noisy channels
}

#[derive(Debug, Clone)]
pub struct LaserConfig {
    pub laser_type: LaserType,
    pub modulation: ModulationScheme,
    pub max_power_mw: f32,
    pub wavelength_nm: u32,
    pub beam_angle_deg: f32,
    pub range_meters: f32,
    pub data_rate_bps: u32,
}

impl Default for LaserConfig {
    fn default() -> Self {
        Self {
            laser_type: LaserType::Visible,
            modulation: ModulationScheme::Ook,
            max_power_mw: 5.0, // 5mW for safety
            wavelength_nm: 650, // Red visible light
            beam_angle_deg: 15.0,
            range_meters: 100.0,
            data_rate_bps: 1_000_000, // 1 Mbps
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReceptionConfig {
    pub use_photodiode: bool,
    pub use_camera: bool,
    pub sensitivity_threshold: f32,
    pub alignment_tolerance_px: u32,
}

impl Default for ReceptionConfig {
    fn default() -> Self {
        Self {
            use_photodiode: true,
            use_camera: false,
            sensitivity_threshold: 0.1,
            alignment_tolerance_px: 10,
        }
    }
}

/// Adaptive power profile for different range categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerProfile {
    pub max_power_mw: f32,
    pub optimal_power_mw: f32,
    pub min_power_mw: f32,
    pub data_rate_bps: u32,
    pub beam_angle_deg: f32,
    pub safety_margin: f32, // Additional safety margin for eye protection
}

impl Default for PowerProfile {
    fn default() -> Self {
        Self {
            max_power_mw: 5.0,
            optimal_power_mw: 3.0,
            min_power_mw: 1.0,
            data_rate_bps: 1_000_000,
            beam_angle_deg: 15.0,
            safety_margin: 1.0,
        }
    }
}

impl PowerProfile {
    /// Create power profile for close range (10-50m)
    pub fn close_range() -> Self {
        Self {
            max_power_mw: 5.0,
            optimal_power_mw: 2.0,
            min_power_mw: 0.5,
            data_rate_bps: 2_000_000,
            beam_angle_deg: 10.0,
            safety_margin: 1.0,
        }
    }

    /// Create power profile for medium range (50-100m)
    pub fn medium_range() -> Self {
        Self {
            max_power_mw: 15.0,
            optimal_power_mw: 8.0,
            min_power_mw: 3.0,
            data_rate_bps: 1_000_000,
            beam_angle_deg: 12.0,
            safety_margin: 1.5,
        }
    }

    /// Create power profile for far range (100-150m)
    pub fn far_range() -> Self {
        Self {
            max_power_mw: 50.0,
            optimal_power_mw: 25.0,
            min_power_mw: 10.0,
            data_rate_bps: 500_000,
            beam_angle_deg: 15.0,
            safety_margin: 2.0,
        }
    }

    /// Create power profile for extreme range (150-200m)
    pub fn extreme_range() -> Self {
        Self {
            max_power_mw: 100.0,
            optimal_power_mw: 60.0,
            min_power_mw: 20.0,
            data_rate_bps: 250_000,
            beam_angle_deg: 20.0,
            safety_margin: 3.0,
        }
    }

    /// Get power profile for a range category
    pub fn for_range_category(category: &RangeDetectorCategory) -> Self {
        match category {
            RangeDetectorCategory::Close => Self::close_range(),
            RangeDetectorCategory::Medium => Self::medium_range(),
            RangeDetectorCategory::Far => Self::far_range(),
            RangeDetectorCategory::Extreme => Self::extreme_range(),
        }
    }

    /// Calculate safe power limit considering eye safety
    pub fn safe_power_limit(&self, laser_type: &LaserType) -> f32 {
        let base_limit = match laser_type {
            LaserType::Visible => 1.0, // 1mW for visible light eye safety
            LaserType::Infrared => 10.0, // 10mW for IR (less restrictive)
        };

        // Apply safety margin
        (base_limit * self.safety_margin).min(self.max_power_mw)
    }
}

#[derive(Debug)]
pub struct LaserEngine {
    config: LaserConfig,
    rx_config: ReceptionConfig,
    visual_engine: VisualEngine,
    rs_codec: ReedSolomon,
    optical_ecc: Option<OpticalECC>,
    is_active: Arc<Mutex<bool>>,
    safety_monitor: Arc<Mutex<SafetyMonitor>>,
    alignment_tracker: Arc<Mutex<AlignmentTracker>>,
    range_detector: Option<Arc<Mutex<RangeDetector>>>,
    current_power_profile: Arc<Mutex<PowerProfile>>,
    adaptive_mode: bool,
}

#[derive(Debug)]
struct SafetyMonitor {
    last_activity: Instant,
    total_energy_joules: f64,
    eye_safety_violations: u32,
}

/// Simple Kalman filter for position tracking and prediction
#[derive(Debug)]
struct KalmanFilter {
    // State vector: [x, y, vx, vy] (position and velocity)
    state: [f32; 4],
    // State covariance matrix (simplified as diagonal)
    covariance: [f32; 4],
    // Process noise
    process_noise: f32,
    // Measurement noise
    measurement_noise: f32,
}

impl KalmanFilter {
    fn new() -> Self {
        Self {
            state: [0.0; 4],
            covariance: [1.0; 4], // Initial uncertainty
            process_noise: 0.1,
            measurement_noise: 0.5,
        }
    }

    /// Predict next state
    fn predict(&mut self, dt: f32) {
        // State transition: position += velocity * dt
        self.state[0] += self.state[2] * dt; // x += vx * dt
        self.state[1] += self.state[3] * dt; // y += vy * dt

        // Update covariance with process noise
        for i in 0..4 {
            self.covariance[i] += self.process_noise;
        }
    }

    /// Update with measurement
    fn update(&mut self, measurement: (f32, f32)) {
        // Kalman gain (simplified)
        let kx = self.covariance[0] / (self.covariance[0] + self.measurement_noise);
        let ky = self.covariance[1] / (self.covariance[1] + self.measurement_noise);

        // Update state
        let innovation_x = measurement.0 - self.state[0];
        let innovation_y = measurement.1 - self.state[1];

        self.state[0] += kx * innovation_x;
        self.state[1] += ky * innovation_y;

        // Update covariance
        self.covariance[0] *= 1.0 - kx;
        self.covariance[1] *= 1.0 - ky;
    }

    /// Get predicted position
    fn predict_position(&self, dt: f32) -> (f32, f32) {
        (
            self.state[0] + self.state[2] * dt,
            self.state[1] + self.state[3] * dt,
        )
    }
}

#[derive(Debug, Clone)]
pub struct AlignmentStatus {
    pub is_aligned: bool,
    pub beam_position_x: f32,
    pub beam_position_y: f32,
    pub signal_strength: f32,
    pub last_update: Instant,
}

#[derive(Debug)]
struct AlignmentTracker {
    target_position: (f32, f32),
    current_position: (f32, f32),
    tolerance_px: f32,
    last_alignment_check: Instant,
    alignment_attempts: u32,
    // Enhanced tracking for optimization
    position_history: VecDeque<((f32, f32), Instant)>,
    velocity_estimate: (f32, f32), // pixels per second
    prediction_enabled: bool,
    kalman_filter: Option<KalmanFilter>,
}

/// Battery state information
#[derive(Debug, Clone)]
pub struct BatteryState {
    pub voltage_v: f32,
    pub current_ma: f32,
    pub capacity_percent: f32,
    pub temperature_celsius: f32,
    pub estimated_runtime_hours: f32,
}

/// Advanced power management configuration
#[derive(Debug, Clone)]
pub struct PowerManagementConfig {
    pub battery_capacity_mah: f32,
    pub target_runtime_hours: f32,
    pub power_save_threshold_percent: f32,
    pub emergency_power_percent: f32,
    pub adaptive_power_enabled: bool,
    pub duty_cycle_optimization: bool,
}

/// Power usage statistics
#[derive(Debug, Clone)]
pub struct PowerStatistics {
    pub total_energy_consumed_joules: f64,
    pub average_power_mw: f32,
    pub peak_power_mw: f32,
    pub duty_cycle_percent: f32,
    pub efficiency_rating: f32,
}

/// Comprehensive laser channel diagnostics
#[derive(Debug, Clone)]
pub struct LaserChannelDiagnostics {
    pub is_active: bool,
    pub alignment_status: AlignmentStatus,
    pub power_consumption_mw: f32,
    pub power_efficiency: f32,
    pub power_safe: bool,
    pub battery_state: Option<BatteryState>,
    pub power_statistics: PowerStatistics,
    pub detected_failures: Vec<LaserError>,
    pub optical_ecc_enabled: bool,
    pub adaptive_mode: bool,
}

impl LaserEngine {
    pub fn new(config: LaserConfig, rx_config: ReceptionConfig) -> Self {
        let visual_engine = VisualEngine::new();
        // Reed-Solomon for error correction (16 data, 4 parity)
        let rs_codec = ReedSolomon::new(16, 4).expect("Failed to create RS codec");

        let tolerance_px = rx_config.alignment_tolerance_px as f32;

        Self {
            config,
            rx_config,
            visual_engine,
            rs_codec,
            optical_ecc: None,
            is_active: Arc::new(Mutex::new(false)),
            safety_monitor: Arc::new(Mutex::new(SafetyMonitor {
                last_activity: Instant::now(),
                total_energy_joules: 0.0,
                eye_safety_violations: 0,
            })),
            alignment_tracker: Arc::new(Mutex::new(AlignmentTracker {
                target_position: (0.0, 0.0),
                current_position: (0.0, 0.0),
                tolerance_px,
                last_alignment_check: Instant::now(),
                alignment_attempts: 0,
                position_history: VecDeque::with_capacity(20),
                velocity_estimate: (0.0, 0.0),
                prediction_enabled: true,
                kalman_filter: Some(KalmanFilter::new()),
            })),
            range_detector: None,
            current_power_profile: Arc::new(Mutex::new(PowerProfile::default())),
            adaptive_mode: false,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), LaserError> {
        #[cfg(target_os = "android")]
        {
            let result = unsafe { laser_init_hardware() };
            if result != 0 {
                return Err(LaserError::HardwareUnavailable);
            }
        }

        // Set active state
        *self.is_active.lock().await = true;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), LaserError> {
        *self.is_active.lock().await = false;
        Ok(())
    }

    pub async fn is_active(&self) -> bool {
        *self.is_active.lock().await
    }

    /// Transmit data using the configured modulation scheme
    pub async fn transmit_data(&mut self, data: &[u8]) -> Result<(), LaserError> {
        if !self.is_active().await {
            return Err(LaserError::HardwareUnavailable);
        }

        // Check safety before transmission
        self.check_safety().await?;

        // Monitor power safety
        self.monitor_power_safety().await?;

        // Update power profile, modulation scheme, and ECC if in adaptive mode
        if self.adaptive_mode {
            // Measure range and update power profile dynamically
            self.measure_range_and_update_power().await?;
            self.update_modulation_scheme().await?;
            self.update_ecc_for_range().await?;
        }

        // Use optimal modulation scheme in adaptive mode, otherwise use configured
        let modulation_scheme = if self.adaptive_mode {
            self.select_optimal_modulation().await
        } else {
            self.config.modulation
        };

        let result = match modulation_scheme {
            ModulationScheme::Ook => self.transmit_ook(data).await,
            ModulationScheme::Pwm => self.transmit_pwm(data).await,
            ModulationScheme::QrProjection => self.transmit_qr_projection(data).await,
            ModulationScheme::Fsk => self.transmit_fsk(data).await,
            ModulationScheme::Manchester => self.transmit_manchester(data).await,
        };

        // Final power safety check after transmission
        self.monitor_power_safety().await?;

        result
    }

    /// Receive data using configured reception method
    pub async fn receive_data(&mut self, timeout_ms: u64) -> Result<Vec<u8>, LaserError> {
        if !self.is_active().await {
            return Err(LaserError::HardwareUnavailable);
        }

        let timeout = Duration::from_millis(timeout_ms);
        let start = Instant::now();

        // Use optimal modulation scheme in adaptive mode, otherwise use configured
        let modulation_scheme = if self.adaptive_mode {
            self.select_optimal_modulation().await
        } else {
            self.config.modulation
        };

        loop {
            if start.elapsed() > timeout {
                return Err(LaserError::Timeout);
            }

            match modulation_scheme {
                ModulationScheme::Ook => {
                    if let Ok(data) = self.receive_ook().await {
                        return Ok(data);
                    }
                }
                ModulationScheme::Pwm => {
                    if let Ok(data) = self.receive_pwm().await {
                        return Ok(data);
                    }
                }
                ModulationScheme::QrProjection => {
                    if let Ok(data) = self.receive_qr_projection().await {
                        return Ok(data);
                    }
                }
                ModulationScheme::Fsk => {
                    if let Ok(data) = self.receive_fsk().await {
                        return Ok(data);
                    }
                }
                ModulationScheme::Manchester => {
                    if let Ok(data) = self.receive_manchester().await {
                        return Ok(data);
                    }
                }
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Transmit using On-Off Keying modulation
    async fn transmit_ook(&mut self, data: &[u8]) -> Result<(), LaserError> {
        // Encode data with error correction
        let encoded = self.encode_with_ecc(data).await?;

        // Get data rate from current power profile
        let data_rate_bps = self.current_power_profile.lock().await.data_rate_bps;

        // Convert to bit stream
        for byte in encoded {
            for bit in 0..8 {
                let is_on = (byte & (1 << (7 - bit))) != 0;
                self.set_laser_intensity(if is_on { 1.0 } else { 0.0 }).await?;
                tokio::time::sleep(Duration::from_micros(1_000_000 / data_rate_bps as u64)).await;
            }
        }

        Ok(())
    }

    /// Receive using On-Off Keying modulation
    async fn receive_ook(&mut self) -> Result<Vec<u8>, LaserError> {
        // Check alignment first
        let alignment = self.get_alignment_status().await;
        if !alignment.is_aligned {
            return Err(LaserError::AlignmentLost);
        }

        // Receive raw signal
        let raw_data = if self.rx_config.use_photodiode {
            self.receive_photodiode().await?
        } else if self.rx_config.use_camera {
            self.receive_camera().await?
        } else {
            return Err(LaserError::ReceptionFailed);
        };

        // Decode with error correction
        self.decode_with_ecc(&raw_data).await
    }

    /// Transmit using Pulse Width Modulation
    async fn transmit_pwm(&mut self, data: &[u8]) -> Result<(), LaserError> {
        let encoded = self.encode_with_ecc(data).await?;

        for byte in encoded {
            // PWM: duty cycle represents data value
            let duty_cycle = byte as f32 / 255.0;
            self.transmit_pwm_byte(duty_cycle).await?;
        }

        Ok(())
    }

    /// Transmit a single PWM byte
    async fn transmit_pwm_byte(&self, duty_cycle: f32) -> Result<(), LaserError> {
        let period_us = 1_000_000 / self.config.data_rate_bps as u64;
        let on_time_us = (period_us as f32 * duty_cycle) as u64;
        let off_time_us = period_us - on_time_us;

        self.set_laser_intensity(1.0).await?;
        tokio::time::sleep(Duration::from_micros(on_time_us)).await;

        self.set_laser_intensity(0.0).await?;
        tokio::time::sleep(Duration::from_micros(off_time_us)).await;

        Ok(())
    }

    /// Receive using Pulse Width Modulation
    async fn receive_pwm(&mut self) -> Result<Vec<u8>, LaserError> {
        // Check alignment first
        let alignment = self.get_alignment_status().await;
        if !alignment.is_aligned {
            return Err(LaserError::AlignmentLost);
        }

        // Receive raw signal
        let raw_data = if self.rx_config.use_photodiode {
            self.receive_photodiode().await?
        } else if self.rx_config.use_camera {
            self.receive_camera().await?
        } else {
            return Err(LaserError::ReceptionFailed);
        };

        // Decode with error correction
        self.decode_with_ecc(&raw_data).await
    }

    /// Transmit using dynamic QR code projection
    async fn transmit_qr_projection(&mut self, data: &[u8]) -> Result<(), LaserError> {
        // Encode data with error correction first
        let encoded_data = self.encode_with_ecc(data).await?;

        // Create visual payload from encoded data
        let payload = VisualPayload {
            session_id: [0; 16], // Would be set properly in real implementation
            public_key: encoded_data,
            nonce: [0; 16],
            signature: vec![],
        };

        // Generate QR code using VisualEngine
        let qr_svg = self.visual_engine.encode_payload(&payload)?;

        // Project the QR code (would control laser projector)
        self.project_qr_code(&qr_svg).await?;

        Ok(())
    }

    /// Receive using QR code scanning
    async fn receive_qr_projection(&mut self) -> Result<Vec<u8>, LaserError> {
        // Check alignment first
        let alignment = self.get_alignment_status().await;
        if !alignment.is_aligned {
            return Err(LaserError::AlignmentLost);
        }

        // Capture QR code from camera
        let raw_data = self.receive_camera().await?;

        // Decode QR data using VisualEngine
        let payload = self.visual_engine.decode_payload(&raw_data)?;

        // Decode with error correction
        self.decode_with_ecc(&payload.public_key).await
    }

    /// Transmit using Frequency Shift Keying
    async fn transmit_fsk(&mut self, data: &[u8]) -> Result<(), LaserError> {
        let encoded = self.encode_with_ecc(data).await?;

        // FSK: Use two different frequencies for 0 and 1
        // Frequency 1: base frequency, Frequency 2: base + offset
        let base_freq = 1000.0; // 1kHz base
        let freq_offset = 500.0; // 500Hz offset

        for byte in encoded {
            for bit in 0..8 {
                let is_high = (byte & (1 << (7 - bit))) != 0;
                let frequency = if is_high { base_freq + freq_offset } else { base_freq };

                // Transmit at the selected frequency for one bit period
                self.transmit_frequency(frequency, Duration::from_micros(1_000_000 / self.config.data_rate_bps as u64)).await?;
            }
        }

        Ok(())
    }

    /// Transmit using Manchester encoding
    async fn transmit_manchester(&mut self, data: &[u8]) -> Result<(), LaserError> {
        let encoded = self.encode_with_ecc(data).await?;

        // Manchester encoding: 0 = 01, 1 = 10
        // Self-clocking, good for noisy channels
        for byte in encoded {
            for bit in 0..8 {
                let bit_value = (byte & (1 << (7 - bit))) != 0;

                // Manchester: transition in middle of bit period
                let half_bit_duration = Duration::from_micros(500_000 / self.config.data_rate_bps as u64);

                if bit_value {
                    // 1: high-low
                    self.set_laser_intensity(1.0).await?;
                    tokio::time::sleep(half_bit_duration).await;
                    self.set_laser_intensity(0.0).await?;
                    tokio::time::sleep(half_bit_duration).await;
                } else {
                    // 0: low-high
                    self.set_laser_intensity(0.0).await?;
                    tokio::time::sleep(half_bit_duration).await;
                    self.set_laser_intensity(1.0).await?;
                    tokio::time::sleep(half_bit_duration).await;
                }
            }
        }

        Ok(())
    }

    /// Transmit at a specific frequency for a duration
    async fn transmit_frequency(&self, _frequency: f32, duration: Duration) -> Result<(), LaserError> {
        // In a real implementation, this would modulate the laser at the specified frequency
        // For now, simulate with on/off patterns
        self.set_laser_intensity(1.0).await?;
        tokio::time::sleep(duration).await;
        Ok(())
    }

    /// Receive using Frequency Shift Keying
    async fn receive_fsk(&mut self) -> Result<Vec<u8>, LaserError> {
        // Check alignment first
        let alignment = self.get_alignment_status().await;
        if !alignment.is_aligned {
            return Err(LaserError::AlignmentLost);
        }

        // Receive raw signal - would detect frequency shifts
        let raw_data = if self.rx_config.use_photodiode {
            self.receive_photodiode().await?
        } else if self.rx_config.use_camera {
            self.receive_camera().await?
        } else {
            return Err(LaserError::ReceptionFailed);
        };

        // Decode FSK signal (simplified - would analyze frequency content)
        self.decode_fsk_signal(&raw_data).await
    }

    /// Receive using Manchester encoding
    async fn receive_manchester(&mut self) -> Result<Vec<u8>, LaserError> {
        // Check alignment first
        let alignment = self.get_alignment_status().await;
        if !alignment.is_aligned {
            return Err(LaserError::AlignmentLost);
        }

        // Receive raw signal
        let raw_data = if self.rx_config.use_photodiode {
            self.receive_photodiode().await?
        } else if self.rx_config.use_camera {
            self.receive_camera().await?
        } else {
            return Err(LaserError::ReceptionFailed);
        };

        // Decode Manchester signal (simplified)
        self.decode_manchester_signal(&raw_data).await
    }

    /// Decode FSK signal (simplified implementation)
    async fn decode_fsk_signal(&self, _raw_data: &[u8]) -> Result<Vec<u8>, LaserError> {
        // In a real implementation, this would perform FFT analysis
        // to detect frequency shifts and decode the data
        // For now, return mock decoded data
        Ok(vec![0xAA, 0xBB, 0xCC]) // Mock data
    }

    /// Decode Manchester signal (simplified implementation)
    async fn decode_manchester_signal(&self, _raw_data: &[u8]) -> Result<Vec<u8>, LaserError> {
        // In a real implementation, this would detect transitions
        // and decode Manchester-encoded bits
        // For now, return mock decoded data
        Ok(vec![0x11, 0x22, 0x33]) // Mock data
    }

    /// Set laser intensity (0.0 to 1.0)
    async fn set_laser_intensity(&self, intensity: f32) -> Result<(), LaserError> {
        // Safety check
        if intensity > 1.0 || intensity < 0.0 {
            return Err(LaserError::SafetyViolation);
        }

        // Get effective power limit from current profile
        let effective_limit = self.get_effective_power_limit().await;
        let power = intensity * effective_limit;

        // Additional safety check against profile limits
        let profile = self.current_power_profile.lock().await;
        if power > profile.max_power_mw {
            return Err(LaserError::SafetyViolation);
        }

        // Update safety monitor
        let mut monitor = self.safety_monitor.lock().await;
        let energy = power as f64 * 0.001; // Convert mW to Joules for 1ms pulse
        monitor.total_energy_joules += energy;
        monitor.last_activity = Instant::now();

        // Hardware control
        #[cfg(target_os = "android")]
        {
            let result = unsafe { laser_set_power(power) };
            if result != 0 {
                return Err(LaserError::TransmissionFailed);
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            // Mock implementation for non-Android platforms
            // laser_hardware.set_power(power);
        }

        Ok(())
    }

    /// Encode data with error correction (OpticalECC if enabled, otherwise Reed-Solomon)
    async fn encode_with_ecc(&mut self, data: &[u8]) -> Result<Vec<u8>, LaserError> {
        if let Some(optical_ecc) = &mut self.optical_ecc {
            // Use enhanced optical ECC
            optical_ecc.encode(data).await
                .map_err(|_| LaserError::DataCorruption)
        } else {
            // Fall back to basic Reed-Solomon
            let shard_size = (data.len() + 15) / 16; // Ceiling division
            let mut shards: Vec<Vec<u8>> = Vec::with_capacity(20);

            // Split data into shards
            for i in 0..16 {
                let start = i * shard_size;
                let end = std::cmp::min(start + shard_size, data.len());
                let mut shard = data[start..end].to_vec();
                shard.resize(shard_size, 0);
                shards.push(shard);
            }

            // Add parity shards
            shards.resize(20, vec![0; shard_size]);
            self.rs_codec.encode(&mut shards).map_err(|_| LaserError::DataCorruption)?;

            // Flatten
            let mut encoded = Vec::new();
            for shard in shards {
                encoded.extend(shard);
            }

            Ok(encoded)
        }
    }

    /// Decode data with error correction (OpticalECC if enabled, otherwise Reed-Solomon)
    async fn decode_with_ecc(&mut self, data: &[u8]) -> Result<Vec<u8>, LaserError> {
        if let Some(optical_ecc) = &mut self.optical_ecc {
            // Use enhanced optical ECC
            optical_ecc.decode(data).await
                .map_err(|_| LaserError::DataCorruption)
        } else {
            // Fall back to basic Reed-Solomon
            let total_size = data.len();
            let shard_size = (total_size + 19) / 20;
            let mut shards: Vec<Option<Vec<u8>>> = Vec::with_capacity(20);

            for i in 0..20 {
                let start = i * shard_size;
                let end = std::cmp::min(start + shard_size, total_size);
                shards.push(Some(data[start..end].to_vec()));
            }

            self.rs_codec.reconstruct(&mut shards).map_err(|_| LaserError::DataCorruption)?;

            let mut decoded = Vec::new();
            for shard in shards.into_iter().take(16).flatten() {
                decoded.extend(shard);
            }

            Ok(decoded)
        }
    }

    /// Project QR code (laser projector control)
    async fn project_qr_code(&self, _qr_svg: &str) -> Result<(), LaserError> {
        // Would control laser projector to display QR code
        // For now, just simulate
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Check safety constraints
    async fn check_safety(&self) -> Result<(), LaserError> {
        let monitor = self.safety_monitor.lock().await;
        let profile = self.current_power_profile.lock().await;

        // Check eye safety limits based on current profile
        let safe_limit = profile.safe_power_limit(&self.config.laser_type);
        if profile.optimal_power_mw > safe_limit {
            return Err(LaserError::SafetyViolation);
        }

        // Check total energy usage
        if monitor.total_energy_joules > 1000.0 { // 1kJ limit
            return Err(LaserError::SafetyViolation);
        }

        Ok(())
    }

    /// Get current alignment status
    pub async fn get_alignment_status(&self) -> AlignmentStatus {
        let tracker = self.alignment_tracker.lock().await;
        let distance = ((tracker.target_position.0 - tracker.current_position.0).powi(2)
                       + (tracker.target_position.1 - tracker.current_position.1).powi(2)).sqrt();

        AlignmentStatus {
            is_aligned: distance <= tracker.tolerance_px,
            beam_position_x: tracker.current_position.0,
            beam_position_y: tracker.current_position.1,
            signal_strength: self.measure_signal_strength().await,
            last_update: tracker.last_alignment_check,
        }
    }

    /// Set target alignment position
    pub async fn set_alignment_target(&self, x: f32, y: f32) -> Result<(), LaserError> {
        let mut tracker = self.alignment_tracker.lock().await;
        tracker.target_position = (x, y);
        tracker.last_alignment_check = Instant::now();
        Ok(())
    }

    /// Perform automatic alignment with predictive tracking
    pub async fn auto_align(&self, max_attempts: u32) -> Result<(), LaserError> {
        let mut tracker = self.alignment_tracker.lock().await;

        for attempt in 0..max_attempts {
            tracker.alignment_attempts = attempt + 1;

            // Measure current position (would use camera feedback)
            let current_pos = self.detect_beam_position().await?;
            let measurement_time = Instant::now();

            // Update position history
            tracker.position_history.push_back((current_pos, measurement_time));
            if tracker.position_history.len() > 20 {
                tracker.position_history.pop_front();
            }

            // Update Kalman filter if enabled
            if let Some(kalman) = &mut tracker.kalman_filter {
                // Update with measurement
                kalman.update(current_pos);

                // Use filtered position
                tracker.current_position = (kalman.state[0], kalman.state[1]);
            } else {
                tracker.current_position = current_pos;
            }

            // Estimate velocity from recent measurements (after position update)
            if tracker.position_history.len() >= 2 {
                self.update_velocity_estimate(&mut tracker).await;
            }

            tracker.last_alignment_check = measurement_time;

            let distance = ((tracker.target_position.0 - tracker.current_position.0).powi(2)
                           + (tracker.target_position.1 - tracker.current_position.1).powi(2)).sqrt();

            if distance <= tracker.tolerance_px {
                return Ok(());
            }

            // Predictive adjustment using velocity estimate
            let adjustment = if tracker.prediction_enabled && tracker.position_history.len() >= 3 {
                self.calculate_predictive_adjustment(&tracker).await
            } else {
                // Simple proportional adjustment
                (tracker.target_position.0 - tracker.current_position.0,
                 tracker.target_position.1 - tracker.current_position.1)
            };

            // Adjust beam position (would control beam steering)
            self.adjust_beam_position(adjustment.0, adjustment.1).await?;

            // Update Kalman filter prediction
            if let Some(kalman) = &mut tracker.kalman_filter {
                // Predict next position (50ms ahead)
                kalman.predict(0.05);
            }

            // Small delay for stabilization
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Err(LaserError::AlignmentLost)
    }

    /// Update velocity estimate from position history
    async fn update_velocity_estimate(&self, tracker: &mut AlignmentTracker) {
        if tracker.position_history.len() < 2 {
            return;
        }

        // Calculate velocity from recent measurements
        let _len = tracker.position_history.len();
        let recent_positions: Vec<&((f32, f32), Instant)> = tracker.position_history.iter().rev().take(3).collect();

        if recent_positions.len() >= 2 {
            let (pos1, time1) = recent_positions[0];
            let (pos2, time2) = recent_positions[1];

            let dt = time1.duration_since(*time2).as_secs_f32();
            if dt > 0.0 {
                let vx = (pos1.0 - pos2.0) / dt;
                let vy = (pos1.1 - pos2.1) / dt;

                // Smooth velocity estimate
                tracker.velocity_estimate.0 = 0.7 * tracker.velocity_estimate.0 + 0.3 * vx;
                tracker.velocity_estimate.1 = 0.7 * tracker.velocity_estimate.1 + 0.3 * vy;
            }
        }
    }

    /// Calculate predictive adjustment using velocity and Kalman prediction
    async fn calculate_predictive_adjustment(&self, tracker: &AlignmentTracker) -> (f32, f32) {
        let dt = 0.1; // Look ahead 100ms

        // Use Kalman prediction if available
        if let Some(kalman) = &tracker.kalman_filter {
            let predicted_pos = kalman.predict_position(dt);
            return (tracker.target_position.0 - predicted_pos.0,
                    tracker.target_position.1 - predicted_pos.1);
        }

        // Fallback to velocity-based prediction
        let predicted_x = tracker.current_position.0 + tracker.velocity_estimate.0 * dt;
        let predicted_y = tracker.current_position.1 + tracker.velocity_estimate.1 * dt;

        (tracker.target_position.0 - predicted_x,
         tracker.target_position.1 - predicted_y)
    }

    /// Detect beam position using camera feedback
    async fn detect_beam_position(&self) -> Result<(f32, f32), LaserError> {
        // Would analyze camera frame to detect laser spot
        // For now, return mock position
        Ok((0.0, 0.0))
    }

    /// Adjust beam position (beam steering)
    async fn adjust_beam_position(&self, delta_x: f32, delta_y: f32) -> Result<(), LaserError> {
        #[cfg(target_os = "android")]
        {
            let result = unsafe { laser_set_alignment(delta_x, delta_y) };
            if result != 0 {
                return Err(LaserError::AlignmentLost);
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            // Mock implementation
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Measure signal strength
    async fn measure_signal_strength(&self) -> f32 {
        // Would measure received signal strength
        // For now, return mock value
        0.8
    }

    /// Receive using photodiode
    async fn receive_photodiode(&self) -> Result<Vec<u8>, LaserError> {
        #[cfg(target_os = "android")]
        {
            // Read analog value from photodiode
            let reading = unsafe { laser_get_photodiode_reading() };
            // Convert analog reading to digital data
            // This is a simplified implementation
            let digital_value = if reading > self.rx_config.sensitivity_threshold { 1 } else { 0 };
            Ok(vec![digital_value])
        }

        #[cfg(not(target_os = "android"))]
        {
            // Mock implementation
            Err(LaserError::ReceptionFailed)
        }
    }

    /// Receive using camera
    async fn receive_camera(&self) -> Result<Vec<u8>, LaserError> {
        // Would capture and analyze camera frames
        // For now, return mock data
        Err(LaserError::ReceptionFailed)
    }

    /// Power management: reduce power when not transmitting
    pub async fn set_standby_mode(&self, standby: bool) -> Result<(), LaserError> {
        if standby {
            self.set_laser_intensity(0.0).await?;
        }
        Ok(())
    }

    /// Get safety statistics
    pub async fn get_safety_stats(&self) -> (f64, u32, Duration) {
        let monitor = self.safety_monitor.lock().await;
        let _uptime = monitor.last_activity.elapsed();
        (monitor.total_energy_joules, monitor.eye_safety_violations, _uptime)
    }

    /// Get current power consumption
    pub async fn get_current_power_consumption(&self) -> f32 {
        let profile = self.current_power_profile.lock().await;
        profile.optimal_power_mw
    }

    /// Check if power usage is within safe limits
    pub async fn is_power_safe(&self) -> bool {
        let monitor = self.safety_monitor.lock().await;
        let profile = self.current_power_profile.lock().await;

        // Check energy limits
        if monitor.total_energy_joules > 1000.0 {
            return false;
        }

        // Check power limits
        let safe_limit = profile.safe_power_limit(&self.config.laser_type);
        if profile.optimal_power_mw > safe_limit {
            return false;
        }

        true
    }

    /// Reset energy monitoring (for new sessions)
    pub async fn reset_energy_monitoring(&self) {
        let mut monitor = self.safety_monitor.lock().await;
        monitor.total_energy_joules = 0.0;
        monitor.eye_safety_violations = 0;
        monitor.last_activity = Instant::now();
    }

    /// Get power efficiency metrics
    pub async fn get_power_efficiency(&self) -> f32 {
        let monitor = self.safety_monitor.lock().await;
        let uptime_seconds = monitor.last_activity.elapsed().as_secs_f32();

        if uptime_seconds > 0.0 {
            // Efficiency as energy per second (lower is better)
            (monitor.total_energy_joules as f32) / uptime_seconds
        } else {
            0.0
        }
    }

    /// Emergency power shutdown
    pub async fn emergency_shutdown(&self) -> Result<(), LaserError> {
        // Force laser off immediately
        self.set_laser_intensity(0.0).await?;

        // Update safety monitor
        let mut monitor = self.safety_monitor.lock().await;
        monitor.eye_safety_violations += 1;

        Ok(())
    }

    /// Monitor power usage and trigger safety actions if needed
    pub async fn monitor_power_safety(&self) -> Result<(), LaserError> {
        if !self.is_power_safe().await {
            // Log safety violation
            let mut monitor = self.safety_monitor.lock().await;
            monitor.eye_safety_violations += 1;

            // Emergency shutdown if violations exceed threshold
            if monitor.eye_safety_violations > 3 {
                return self.emergency_shutdown().await;
            }

            // Reduce power to safe levels
            let profile = self.current_power_profile.lock().await;
            let safe_limit = profile.safe_power_limit(&self.config.laser_type);

            if profile.optimal_power_mw > safe_limit {
                // Would adjust power profile here
                // For now, just return error
                return Err(LaserError::SafetyViolation);
            }
        }

        Ok(())
    }

    /// Enable enhanced optical ECC with atmospheric adaptation
    pub fn enable_optical_ecc(&mut self, config: AdaptiveECCConfig) -> Result<(), LaserError> {
        self.optical_ecc = Some(OpticalECC::new(config));
        Ok(())
    }

    /// Disable optical ECC (fall back to basic Reed-Solomon)
    pub fn disable_optical_ecc(&mut self) {
        self.optical_ecc = None;
    }

    /// Check if optical ECC is enabled
    pub fn is_optical_ecc_enabled(&self) -> bool {
        self.optical_ecc.is_some()
    }

    /// Update optical quality metrics for adaptive ECC
    pub async fn update_optical_quality(&mut self, metrics: OpticalQualityMetrics) -> Result<(), LaserError> {
        if let Some(optical_ecc) = &mut self.optical_ecc {
            optical_ecc.update_quality_metrics(metrics).await
                .map_err(|e| LaserError::DataCorruption)?;
        }
        Ok(())
    }

    /// Update ECC strength based on current range detection
    pub async fn update_ecc_for_range(&mut self) -> Result<(), LaserError> {
        if self.optical_ecc.is_none() || self.range_detector.is_none() {
            return Ok(());
        }

        let range_measurement = self.range_detector.as_ref().unwrap().lock().await
            .measure_distance_averaged().await
            .map_err(|_| LaserError::TransmissionFailed)?;

        // Create optical quality metrics based on range
        let metrics = OpticalQualityMetrics {
            ber: 0.0, // Would be measured from actual transmission
            per: 0.0,
            signal_strength: 0.8, // Default good signal
            atmospheric_attenuation: self.calculate_attenuation_for_range(range_measurement.distance_m),
            turbulence_index: 0.1, // Low turbulence assumed
            background_noise: 0.1,
            range_meters: range_measurement.distance_m,
            timestamp: Instant::now(),
        };

        self.update_optical_quality(metrics).await
    }

    /// Calculate atmospheric attenuation based on range
    fn calculate_attenuation_for_range(&self, distance_m: f32) -> f32 {
        // Simplified attenuation calculation
        // In clear air, attenuation increases with distance and frequency
        // This is a rough approximation
        let base_attenuation = 0.1; // dB per 100m
        let frequency_factor = (self.config.wavelength_nm as f32 / 650.0).powi(2); // Higher frequency = more attenuation

        base_attenuation * (distance_m / 100.0) * frequency_factor
    }

    /// Start continuous range monitoring and profile switching
    pub async fn start_continuous_monitoring(&self) -> Result<(), LaserError> {
        if !self.adaptive_mode || self.range_detector.is_none() {
            return Err(LaserError::HardwareUnavailable);
        }

        // Spawn a background task for continuous monitoring
        let range_detector = self.range_detector.as_ref().unwrap().clone();
        let current_profile = self.current_power_profile.clone();

        tokio::spawn(async move {
            let mut last_range_category: Option<RangeDetectorCategory> = None;
            let monitoring_active = true;

            while monitoring_active {
                // Measure distance
                let measurement_result = range_detector.lock().await.measure_distance_averaged().await;

                match measurement_result {
                    Ok(measurement) => {
                        let current_category = RangeDetectorCategory::from_distance(measurement.distance_m);

                        // Check if range category changed
                        if last_range_category != Some(current_category) {
                            println!("Range category changed from {:?} to {:?} ({}m)",
                                   last_range_category, current_category, measurement.distance_m);

                            // Update power profile for new range
                            let new_profile = PowerProfile::for_range_category(&current_category);
                            *current_profile.lock().await = new_profile;

                            last_range_category = Some(current_category);
                        }
                    }
                    Err(_e) => {
                        eprintln!("Range measurement failed: {:?}", _e);
                        // Continue monitoring despite errors
                    }
                }

                // Monitor every 2 seconds
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        Ok(())
    }

    /// Stop continuous monitoring (would need a cancellation token in real implementation)
    pub async fn stop_continuous_monitoring(&self) -> Result<(), LaserError> {
        // In a real implementation, this would signal the monitoring task to stop
        // For now, just return success
        Ok(())
    }

    /// Get current monitoring status
    pub async fn get_monitoring_status(&self) -> (bool, Option<RangeDetectorCategory>) {
        let is_adaptive = self.adaptive_mode;
        let current_category = if self.range_detector.is_some() {
            self.range_detector.as_ref().unwrap().lock().await.get_current_range_category().await
        } else {
            None
        };

        (is_adaptive, current_category)
    }

    /// Update environmental conditions for compensation
    pub async fn update_environmental_conditions(&self, weather: WeatherCondition, visibility_m: f32) -> Result<(), LaserError> {
        if self.range_detector.is_none() {
            return Err(LaserError::HardwareUnavailable);
        }

        // Convert weather condition to environmental parameters
        let (temperature, humidity, pressure, wind_speed) = match weather {
            WeatherCondition::Clear => (20.0, 50.0, 1013.25, 2.0),
            WeatherCondition::Rain => (15.0, 85.0, 1008.0, 5.0),
            WeatherCondition::Fog => (10.0, 98.0, 1010.0, 1.0),
            WeatherCondition::Storm => (12.0, 95.0, 1005.0, 8.0),
            WeatherCondition::Snow => (0.0, 90.0, 1012.0, 3.0),
            WeatherCondition::HeavyRain => (14.0, 95.0, 1006.0, 7.0),
            WeatherCondition::LightRain => (16.0, 80.0, 1009.0, 4.0),
            WeatherCondition::Cloudy => (18.0, 60.0, 1012.0, 2.5),
        };

        let conditions = RangeEnvironmentalConditions {
            temperature_celsius: temperature,
            humidity_percent: humidity,
            pressure_hpa: pressure,
            wind_speed_mps: wind_speed,
            visibility_meters: visibility_m,
        };

        self.range_detector.as_ref().unwrap().lock().await
            .update_environmental_conditions(conditions).await;

        // Update power profile based on environmental conditions
        self.adjust_power_for_weather(weather, visibility_m).await?;

        Ok(())
    }

    /// Adjust power profile based on weather conditions
    async fn adjust_power_for_weather(&self, weather: WeatherCondition, visibility_m: f32) -> Result<(), LaserError> {
        let mut profile = self.current_power_profile.lock().await;

        // Calculate weather-based power multiplier
        let weather_multiplier = match weather {
            WeatherCondition::Clear => 1.0,
            WeatherCondition::Rain => 1.5,
            WeatherCondition::Fog => 3.0,  // Significant attenuation in fog
            WeatherCondition::Storm => 2.0,
            WeatherCondition::Snow => 2.5,
            WeatherCondition::HeavyRain => 2.0,
            WeatherCondition::LightRain => 1.3,
            WeatherCondition::Cloudy => 1.1,
        };

        // Calculate visibility-based multiplier
        let visibility_multiplier = if visibility_m < 100.0 {
            3.0  // Very poor visibility
        } else if visibility_m < 500.0 {
            2.0  // Poor visibility
        } else if visibility_m < 1000.0 {
            1.5  // Moderate visibility
        } else {
            1.0  // Good visibility
        };

        // Apply environmental compensation
        let environmental_factor = weather_multiplier * visibility_multiplier;
        profile.optimal_power_mw *= environmental_factor;

        // Ensure we don't exceed absolute safety limits
        let laser_type_limit = profile.safe_power_limit(&self.config.laser_type);
        profile.optimal_power_mw = profile.optimal_power_mw.min(laser_type_limit);

        Ok(())
    }

    /// Get environmental impact on transmission
    pub async fn get_environmental_impact(&self) -> Option<(WeatherCondition, f32, f32)> {
        if self.range_detector.is_none() {
            return None;
        }

        let conditions = self.range_detector.as_ref().unwrap().lock().await
            .get_environmental_conditions().await;

        // Infer weather condition from environmental data
        let weather = self.infer_weather_from_conditions(&conditions);
        let visibility = conditions.visibility_meters;
        let attenuation_factor = self.calculate_environmental_attenuation(&conditions);

        Some((weather, visibility, attenuation_factor))
    }

    /// Infer weather condition from environmental parameters
    fn infer_weather_from_conditions(&self, conditions: &RangeEnvironmentalConditions) -> WeatherCondition {
        if conditions.humidity_percent > 95.0 && conditions.temperature_celsius < 15.0 {
            WeatherCondition::Fog
        } else if conditions.temperature_celsius < 5.0 && conditions.humidity_percent > 80.0 {
            WeatherCondition::Snow
        } else if conditions.humidity_percent > 90.0 && conditions.pressure_hpa < 1010.0 {
            WeatherCondition::HeavyRain
        } else if conditions.humidity_percent > 75.0 && conditions.pressure_hpa < 1012.0 {
            WeatherCondition::LightRain
        } else if conditions.humidity_percent > 60.0 {
            WeatherCondition::Cloudy
        } else {
            WeatherCondition::Clear
        }
    }

    /// Calculate environmental attenuation factor
    fn calculate_environmental_attenuation(&self, conditions: &RangeEnvironmentalConditions) -> f32 {
        // Simplified environmental attenuation model
        let humidity_factor = conditions.humidity_percent / 100.0;
        let temperature_factor = (20.0 - conditions.temperature_celsius).abs() / 20.0; // Deviation from 20°C
        let pressure_factor = (1013.25 - conditions.pressure_hpa).abs() / 10.0; // Deviation from standard pressure

        // Combine factors (simplified model)
        1.0 + humidity_factor * 0.5 + temperature_factor * 0.3 + pressure_factor * 0.2
    }

    /// Get recommended safety margins for current conditions
    pub async fn get_safety_margins(&self) -> (f32, f32, f32) {
        // Return (power_margin, range_margin, alignment_margin)
        if let Some((weather, visibility, attenuation)) = self.get_environmental_impact().await {
            let power_margin = match weather {
                WeatherCondition::Clear => 1.0,
                WeatherCondition::Rain => 1.5,
                WeatherCondition::Fog => 3.0,
                WeatherCondition::Storm => 2.0,
                WeatherCondition::Snow => 2.5,
                WeatherCondition::HeavyRain => 2.0,
                WeatherCondition::LightRain => 1.3,
                WeatherCondition::Cloudy => 1.1,
            };

            let range_margin = if visibility < 500.0 { 0.8 } else { 1.0 };
            let alignment_margin = attenuation.clamp(1.0, 2.0);

            (power_margin, range_margin, alignment_margin)
        } else {
            (1.0, 1.0, 1.0) // Default margins
        }
    }

    /// Get current optical ECC adaptation state
    pub async fn get_optical_ecc_state(&self) -> Option<crate::optical_ecc::AdaptationState> {
        if let Some(optical_ecc) = &self.optical_ecc {
            Some(optical_ecc.get_adaptation_state().await)
        } else {
            None
        }
    }

    /// Detect specific laser channel failures
    pub async fn detect_channel_failures(&self) -> Vec<LaserError> {
        let mut failures = Vec::new();

        // Check if laser is active
        if !self.is_active().await {
            failures.push(LaserError::HardwareUnavailable);
            return failures;
        }

        // Check alignment status
        let alignment = self.get_alignment_status().await;
        if !alignment.is_aligned {
            failures.push(LaserError::AlignmentLost);
        }

        // Check signal strength
        if alignment.signal_strength < 0.2 {
            failures.push(LaserError::TransmissionFailed);
        }

        // Check safety violations
        if !self.is_power_safe().await {
            failures.push(LaserError::SafetyViolation);
        }

        // Check for beam obstruction (low signal despite good alignment)
        if alignment.is_aligned && alignment.signal_strength < 0.3 {
            failures.push(LaserError::DataCorruption); // Indicates obstruction
        }

        failures
    }

    /// Get detailed channel diagnostics
    pub async fn get_channel_diagnostics(&self) -> LaserChannelDiagnostics {
        let alignment = self.get_alignment_status().await;
        let power_stats = (self.get_current_power_consumption().await,
                          self.get_power_efficiency().await,
                          self.is_power_safe().await);
        let failures = self.detect_channel_failures().await;

        LaserChannelDiagnostics {
            is_active: self.is_active().await,
            alignment_status: alignment,
            power_consumption_mw: power_stats.0,
            power_efficiency: power_stats.1,
            power_safe: power_stats.2,
            battery_state: None, // Would be populated by battery monitoring system
            power_statistics: PowerStatistics {
                total_energy_consumed_joules: self.safety_monitor.lock().await.total_energy_joules,
                average_power_mw: power_stats.0,
                peak_power_mw: self.current_power_profile.lock().await.max_power_mw,
                duty_cycle_percent: 50.0, // Would be calculated from actual usage
                efficiency_rating: power_stats.1,
            },
            detected_failures: failures,
            optical_ecc_enabled: self.is_optical_ecc_enabled(),
            adaptive_mode: self.is_adaptive_mode(),
        }
    }

    /// Enable adaptive power mode with range detector
    pub fn enable_adaptive_mode(&mut self, range_detector: Arc<Mutex<RangeDetector>>) {
        self.range_detector = Some(range_detector);
        self.adaptive_mode = true;
    }

    /// Perform range measurement and update power profile
    pub async fn measure_range_and_update_power(&self) -> Result<(), LaserError> {
        if !self.adaptive_mode || self.range_detector.is_none() {
            return Err(LaserError::HardwareUnavailable);
        }

        let range_detector = self.range_detector.as_ref().unwrap();
        let measurement = range_detector.lock().await.measure_distance_averaged().await
            .map_err(|_| LaserError::TransmissionFailed)?;

        // Update power profile based on measured range
        let category = RangeDetectorCategory::from_distance(measurement.distance_m);
        let new_profile = PowerProfile::for_range_category(&category);

        // Apply environmental compensation if available
        if let Some((weather, visibility, _)) = self.get_environmental_impact().await {
            let mut adjusted_profile = new_profile;
            let weather_multiplier = match weather {
                WeatherCondition::Clear => 1.0,
                WeatherCondition::Rain => 1.5,
                WeatherCondition::Fog => 3.0,
                WeatherCondition::Storm => 2.0,
                WeatherCondition::Snow => 2.5,
                WeatherCondition::HeavyRain => 2.0,
                WeatherCondition::LightRain => 1.3,
                WeatherCondition::Cloudy => 1.1,
            };

            let visibility_multiplier = if visibility < 100.0 {
                3.0
            } else if visibility < 500.0 {
                2.0
            } else if visibility < 1000.0 {
                1.5
            } else {
                1.0
            };

            let environmental_factor = weather_multiplier * visibility_multiplier;
            adjusted_profile.optimal_power_mw *= environmental_factor;
            adjusted_profile.optimal_power_mw = adjusted_profile.optimal_power_mw.min(adjusted_profile.max_power_mw);

            *self.current_power_profile.lock().await = adjusted_profile;
        } else {
            *self.current_power_profile.lock().await = new_profile;
        }

        Ok(())
    }

    /// Get current range measurement from detector
    pub async fn get_current_range_measurement(&self) -> Option<RangeMeasurement> {
        if let Some(range_detector) = &self.range_detector {
            range_detector.lock().await.get_measurement_history().await.last().cloned()
        } else {
            None
        }
    }

    /// Update range detector with current environmental conditions
    pub async fn update_range_detector_environment(&self, weather: WeatherCondition, visibility_m: f32) -> Result<(), LaserError> {
        if self.range_detector.is_none() {
            return Err(LaserError::HardwareUnavailable);
        }

        // Convert weather to environmental parameters
        let (temperature, humidity, pressure, wind_speed) = match weather {
            WeatherCondition::Clear => (20.0, 50.0, 1013.25, 2.0),
            WeatherCondition::Rain => (15.0, 85.0, 1008.0, 5.0),
            WeatherCondition::Fog => (10.0, 98.0, 1010.0, 1.0),
            WeatherCondition::Storm => (12.0, 95.0, 1005.0, 8.0),
            WeatherCondition::Snow => (0.0, 90.0, 1012.0, 3.0),
            WeatherCondition::HeavyRain => (14.0, 95.0, 1006.0, 7.0),
            WeatherCondition::LightRain => (16.0, 80.0, 1009.0, 4.0),
            WeatherCondition::Cloudy => (18.0, 60.0, 1012.0, 2.5),
        };

        let conditions = RangeEnvironmentalConditions {
            temperature_celsius: temperature,
            humidity_percent: humidity,
            pressure_hpa: pressure,
            wind_speed_mps: wind_speed,
            visibility_meters: visibility_m,
        };

        self.range_detector.as_ref().unwrap().lock().await
            .update_environmental_conditions(conditions).await;

        Ok(())
    }

    /// Disable adaptive power mode
    pub async fn disable_adaptive_mode(&mut self) {
        self.range_detector = None;
        self.adaptive_mode = false;
        // Reset to default profile
        *self.current_power_profile.lock().await = PowerProfile::default();
    }

    /// Check if adaptive mode is enabled
    pub fn is_adaptive_mode(&self) -> bool {
        self.adaptive_mode
    }

    /// Update power profile based on current range measurement
    pub async fn update_power_profile(&self) -> Result<(), LaserError> {
        if !self.adaptive_mode || self.range_detector.is_none() {
            return Ok(());
        }

        let range_detector = self.range_detector.as_ref().unwrap();
        let range_category = range_detector.lock().await.get_current_range_category().await;

        if let Some(category) = range_category {
            let new_profile = PowerProfile::for_range_category(&category);
            *self.current_power_profile.lock().await = new_profile;
        }

        Ok(())
    }

    /// Get current power profile
    pub async fn get_current_power_profile(&self) -> PowerProfile {
        self.current_power_profile.lock().await.clone()
    }

    /// Manually set power profile
    pub async fn set_power_profile(&self, profile: PowerProfile) -> Result<(), LaserError> {
        // Validate profile against laser type safety limits
        let safe_limit = profile.safe_power_limit(&self.config.laser_type);
        if profile.optimal_power_mw > safe_limit {
            return Err(LaserError::SafetyViolation);
        }

        *self.current_power_profile.lock().await = profile;
        Ok(())
    }

    /// Get effective power limit considering current profile and safety
    pub async fn get_effective_power_limit(&self) -> f32 {
        let profile = self.current_power_profile.lock().await;
        let safe_limit = profile.safe_power_limit(&self.config.laser_type);
        profile.max_power_mw.min(safe_limit)
    }

    /// Select optimal modulation scheme based on range, conditions, and performance metrics
    pub async fn select_optimal_modulation(&self) -> ModulationScheme {
        if !self.adaptive_mode || self.range_detector.is_none() {
            return self.config.modulation;
        }

        let range_category = self.range_detector.as_ref().unwrap().lock().await
            .get_current_range_category().await;

        // Get environmental conditions for modulation selection
        let environmental_impact = self.get_environmental_impact().await;
        let signal_quality = self.measure_signal_strength().await;

        // Advanced modulation selection based on multiple factors
        match range_category {
            Some(RangeDetectorCategory::Close) => {
                // Close range (<50m): Prioritize speed
                if signal_quality > 0.8 && environmental_impact.map_or(true, |(_, _, att)| att < 1.2) {
                    ModulationScheme::Ook // Highest speed
                } else {
                    ModulationScheme::Manchester // Better noise immunity
                }
            }
            Some(RangeDetectorCategory::Medium) => {
                // Medium range (50-100m): Balance speed and reliability
                if signal_quality > 0.6 {
                    ModulationScheme::Pwm // Good balance
                } else {
                    ModulationScheme::Fsk // Better for moderate interference
                }
            }
            Some(RangeDetectorCategory::Far) => {
                // Far range (100-150m): Prioritize reliability
                if environmental_impact.map_or(false, |(_, vis, _)| vis < 300.0) {
                    // Poor visibility: Use most robust scheme
                    ModulationScheme::QrProjection
                } else {
                    ModulationScheme::Manchester // Good robustness for distance
                }
            }
            Some(RangeDetectorCategory::Extreme) => {
                // Extreme range (150-200m): Maximum robustness
                ModulationScheme::QrProjection // Best error correction and robustness
            }
            None => {
                // No range data: Use environmental conditions to decide
                if signal_quality < 0.5 {
                    ModulationScheme::QrProjection
                } else if environmental_impact.map_or(false, |(_, _, att)| att > 1.5) {
                    ModulationScheme::Manchester
                } else {
                    self.config.modulation
                }
            }
        }
    }

    /// Update modulation scheme based on current conditions
    pub async fn update_modulation_scheme(&self) -> Result<(), LaserError> {
        if !self.adaptive_mode {
            return Ok(());
        }

        let optimal_scheme = self.select_optimal_modulation().await;
        // Note: In a real implementation, this would update the modulation scheme
        // For now, we just validate that the scheme is supported
        match optimal_scheme {
            ModulationScheme::Ook | ModulationScheme::Pwm | ModulationScheme::QrProjection |
            ModulationScheme::Fsk | ModulationScheme::Manchester => Ok(()),
        }
    }

    /// Advanced power management: optimize power usage based on battery state and requirements
    pub async fn optimize_power_usage(&mut self, battery_state: Option<&BatteryState>) -> Result<(), LaserError> {
        let mut profile = self.current_power_profile.lock().await;

        if let Some(battery) = battery_state {
            // Adaptive power scaling based on battery level
            let power_multiplier = if battery.capacity_percent < 20.0 {
                // Emergency power mode - reduce power significantly
                0.3
            } else if battery.capacity_percent < 50.0 {
                // Power saving mode
                0.6
            } else if battery.estimated_runtime_hours < 2.0 {
                // Low runtime - moderate power reduction
                0.8
            } else {
                // Normal operation
                1.0
            };

            // Apply temperature compensation
            let temp_multiplier = if battery.temperature_celsius > 40.0 {
                // High temperature - reduce power to prevent overheating
                0.7
            } else if battery.temperature_celsius < 0.0 {
                // Cold temperature - may need more power for efficiency
                1.1
            } else {
                1.0
            };

            let total_multiplier = power_multiplier * temp_multiplier;
            profile.optimal_power_mw *= total_multiplier;
            profile.optimal_power_mw = profile.optimal_power_mw.min(profile.max_power_mw);
            profile.optimal_power_mw = profile.optimal_power_mw.max(profile.min_power_mw);
        }

        Ok(())
    }

    /// Calculate optimal duty cycle for power efficiency
    pub async fn calculate_optimal_duty_cycle(&self, data_rate_bps: u32, required_power_mw: f32) -> f32 {
        // Duty cycle optimization for pulsed operation
        // Higher data rates may require higher duty cycles
        let base_duty_cycle = (data_rate_bps as f32 / 1_000_000.0).min(1.0); // Max 100% at 1Mbps

        // Adjust based on power requirements
        let power_factor = (required_power_mw / self.current_power_profile.lock().await.max_power_mw).min(1.0);

        (base_duty_cycle * power_factor).max(0.1).min(1.0) // Keep between 10% and 100%
    }

    /// Implement burst transmission mode for power saving
    pub async fn enable_burst_mode(&mut self, burst_duration_ms: u32, idle_duration_ms: u32) -> Result<(), LaserError> {
        // Configure burst transmission pattern
        // This would modify the transmission timing to use short bursts with idle periods
        // to reduce average power consumption

        // Calculate burst efficiency
        let total_cycle = burst_duration_ms + idle_duration_ms;
        let duty_cycle = burst_duration_ms as f32 / total_cycle as f32;

        // Adjust power profile for burst mode
        let mut profile = self.current_power_profile.lock().await;
        profile.optimal_power_mw /= duty_cycle.sqrt(); // Compensate for burst power requirements

        Ok(())
    }

    /// Monitor and predict battery drain
    pub async fn predict_battery_drain(&self, operation_duration_seconds: f32) -> f32 {
        let current_power = self.get_current_power_consumption().await;
        let energy_consumed_joules = current_power as f32 * operation_duration_seconds / 1000.0;

        // Convert to battery percentage (simplified model)
        // Assuming 3000mAh battery at 3.7V = ~11.1Wh = 40,000J
        const BATTERY_CAPACITY_JOULES: f32 = 40_000.0;
        (energy_consumed_joules / BATTERY_CAPACITY_JOULES) * 100.0
    }

    /// Get power management recommendations
    pub async fn get_power_recommendations(&self, battery_state: Option<&BatteryState>) -> Vec<String> {
        let mut recommendations = Vec::new();

        if let Some(battery) = battery_state {
            if battery.capacity_percent < 15.0 {
                recommendations.push("Battery critically low. Switching to emergency power mode.".to_string());
            } else if battery.capacity_percent < 30.0 {
                recommendations.push("Battery low. Consider reducing transmission power.".to_string());
            }

            if battery.estimated_runtime_hours < 1.0 {
                recommendations.push("Estimated runtime very low. Enable burst mode for power saving.".to_string());
            }

            if battery.temperature_celsius > 45.0 {
                recommendations.push("High battery temperature detected. Reducing power to prevent damage.".to_string());
            }
        }

        let efficiency = self.get_power_efficiency().await;
        if efficiency < 0.5 {
            recommendations.push("Low power efficiency detected. Consider duty cycle optimization.".to_string());
        }

        recommendations
    }

    /// Emergency power shutdown with graceful degradation
    pub async fn emergency_power_shutdown(&self) -> Result<(), LaserError> {
        // Reduce power to minimum safe level first
        self.set_laser_intensity(0.0).await?;

        // Log emergency shutdown
        let mut monitor = self.safety_monitor.lock().await;
        monitor.eye_safety_violations += 1;

        // In a real implementation, this would:
        // 1. Save current state
        // 2. Notify application of emergency
        // 3. Switch to ultra-low power mode
        // 4. Prepare for graceful recovery

        Ok(())
    }

    /// Calculate power budget for a given operation
    pub async fn calculate_power_budget(&self, operation: &str, duration_seconds: f32) -> PowerBudget {
        let current_power = self.get_current_power_consumption().await;
        let energy_required = current_power as f64 * duration_seconds as f64 / 1000.0; // Joules

        let battery_capacity = 40_000.0; // 40kJ typical battery capacity
        let available_energy = battery_capacity * 0.8; // 80% usable capacity

        let can_complete = energy_required <= available_energy;
        let estimated_drain_percent = (energy_required / battery_capacity * 100.0) as f32;

        PowerBudget {
            operation: operation.to_string(),
            energy_required_joules: energy_required,
            estimated_duration_seconds: duration_seconds,
            can_complete_operation: can_complete,
            estimated_battery_drain_percent: estimated_drain_percent,
            recommended_power_level_mw: if can_complete {
                current_power
            } else {
                (available_energy / duration_seconds as f64 * 1000.0) as f32
            },
        }
    }
}

/// Power budget analysis for operations
#[derive(Debug, Clone)]
pub struct PowerBudget {
    pub operation: String,
    pub energy_required_joules: f64,
    pub estimated_duration_seconds: f32,
    pub can_complete_operation: bool,
    pub estimated_battery_drain_percent: f32,
    pub recommended_power_level_mw: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_laser_engine_creation() {
        let config = LaserConfig::default();
        let rx_config = ReceptionConfig::default();
        let engine = LaserEngine::new(config, rx_config);

        assert!(!engine.is_active().await);
    }

    #[tokio::test]
    async fn test_laser_engine_initialization() {
        let config = LaserConfig::default();
        let rx_config = ReceptionConfig::default();
        let mut engine = LaserEngine::new(config, rx_config);

        // Initialization should succeed (even with mock hardware)
        let result = engine.initialize().await;
        assert!(result.is_ok());
        assert!(engine.is_active().await);
    }

    #[tokio::test]
    async fn test_alignment_tracking() {
        let config = LaserConfig::default();
        let rx_config = ReceptionConfig::default();
        let engine = LaserEngine::new(config, rx_config);

        // Set alignment target
        let result = engine.set_alignment_target(100.0, 200.0).await;
        assert!(result.is_ok());

        // Check alignment status
        let status = engine.get_alignment_status().await;
        assert!(!status.is_aligned); // Should not be aligned initially
    }

    #[tokio::test]
    async fn test_error_correction() {
        let config = LaserConfig::default();
        let rx_config = ReceptionConfig::default();
        let mut engine = LaserEngine::new(config, rx_config);

        let test_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        // Encode data
        let encoded = engine.encode_with_ecc(&test_data).await;
        assert!(encoded.is_ok());
        let encoded_data = encoded.unwrap();

        // Decode data
        let decoded = engine.decode_with_ecc(&encoded_data).await;
        assert!(decoded.is_ok());
        let decoded_data = decoded.unwrap();

        assert_eq!(test_data, decoded_data);
    }

    #[tokio::test]
    async fn test_safety_limits() {
        let config = LaserConfig::default();
        let rx_config = ReceptionConfig::default();
        let engine = LaserEngine::new(config, rx_config);

        // Test invalid intensity values
        let result = engine.set_laser_intensity(1.5).await;
        assert!(matches!(result, Err(LaserError::SafetyViolation)));

        let result = engine.set_laser_intensity(-0.1).await;
        assert!(matches!(result, Err(LaserError::SafetyViolation)));
    }

    #[tokio::test]
    async fn test_power_management() {
        let config = LaserConfig::default();
        let rx_config = ReceptionConfig::default();
        let engine = LaserEngine::new(config, rx_config);

        // Test standby mode
        let result = engine.set_standby_mode(true).await;
        assert!(result.is_ok());

        // Check safety stats
        let (energy, violations, uptime) = engine.get_safety_stats().await;
        assert!(energy >= 0.0);
        assert_eq!(violations, 0);
    }
}