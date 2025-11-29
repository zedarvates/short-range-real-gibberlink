//! # Laser Control Module
//!
//! Core laser communication operations, modulation schemes, and data transmission.

use std::sync::Arc;
use tokio::sync::Mutex;
use reed_solomon_erasure::ReedSolomon;
use crate::range_detector::{RangeDetector};
use crate::optical_ecc::OpticalECC;
use super::types::{LaserType, ModulationScheme, ChannelDiagnostics};
use crate::laser::AlignmentStatus;

/// Laser configuration parameters
#[derive(Debug, Clone)]
pub struct LaserConfig {
    pub laser_type: LaserType,
    pub modulation_scheme: ModulationScheme,
    pub max_power_mw: f32,
    pub wavelength_nm: u32,
    pub beam_diameter_mm: f32,
    pub range_meters: f32,
    pub safety_enabled: bool,
}

impl Default for LaserConfig {
    fn default() -> Self {
        Self::red() // Default to red laser
    }
}

impl LaserConfig {
    /// Create a red laser configuration (650nm)
    pub fn red() -> Self {
        Self {
            laser_type: LaserType::Red,
            modulation_scheme: ModulationScheme::Ook,
            max_power_mw: 5.0,
            wavelength_nm: 650,
            beam_diameter_mm: 2.0,
            range_meters: 100.0,
            safety_enabled: true,
        }
    }

    /// Create a green laser configuration (532nm)
    pub fn green() -> Self {
        Self {
            laser_type: LaserType::Green,
            modulation_scheme: ModulationScheme::Ook,
            max_power_mw: 5.0,
            wavelength_nm: 532,
            beam_diameter_mm: 2.0,
            range_meters: 100.0,
            safety_enabled: true,
        }
    }

    /// Create a blue laser configuration (450nm)
    pub fn blue() -> Self {
        Self {
            laser_type: LaserType::Blue,
            modulation_scheme: ModulationScheme::Ook,
            max_power_mw: 5.0,
            wavelength_nm: 450,
            beam_diameter_mm: 2.0,
            range_meters: 100.0,
            safety_enabled: true,
        }
    }

    /// Create an infrared laser configuration
    pub fn infrared() -> Self {
        Self {
            laser_type: LaserType::Infrared,
            modulation_scheme: ModulationScheme::Ook,
            max_power_mw: 10.0, // IR can handle higher power
            wavelength_nm: 980, // Common IR wavelength
            beam_diameter_mm: 2.0,
            range_meters: 200.0, // IR has better range
            safety_enabled: true,
        }
    }

    /// Create a UV laser configuration
    pub fn uv() -> Self {
        Self {
            laser_type: LaserType::UV,
            modulation_scheme: ModulationScheme::Ook,
            max_power_mw: 1.0, // UV lasers are typically lower power
            wavelength_nm: 405, // Near UV
            beam_diameter_mm: 1.5,
            range_meters: 50.0, // UV has shorter range
            safety_enabled: true,
        }
    }

    /// Create an external/custom laser configuration
    pub fn external(wavelength_nm: u32) -> Self {
        Self {
            laser_type: LaserType::External,
            modulation_scheme: ModulationScheme::Ook,
            max_power_mw: 5.0,
            wavelength_nm,
            beam_diameter_mm: 2.0,
            range_meters: 100.0,
            safety_enabled: true,
        }
    }
}

/// Reception configuration
#[derive(Debug, Clone)]
pub struct ReceptionConfig {
    pub photodiode_sensitivity: f32,
    pub camera_resolution: (u32, u32),
    pub frame_rate_hz: u32,
    pub exposure_time_us: u32,
}

impl Default for ReceptionConfig {
    fn default() -> Self {
        Self {
            photodiode_sensitivity: 0.8,
            camera_resolution: (640, 480),
            frame_rate_hz: 30,
            exposure_time_us: 1000,
        }
    }
}

/// Laser error types
#[derive(Debug, thiserror::Error)]
pub enum LaserError {
    #[error("Hardware initialization failed: {0}")]
    HardwareInit(String),
    #[error("Safety violation: {0}")]
    SafetyViolation(String),
    #[error("Modulation error: {0}")]
    ModulationError(String),
    #[error("Communication timeout")]
    Timeout,
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Environmental conditions unsafe")]
    EnvironmentalHazard,
    #[error("Beam alignment failed")]
    AlignmentFailed,
    #[error("Power management error: {0}")]
    PowerError(String),
    #[error("ECC encoding/decoding failed: {0}")]
    EccError(String),
}

/// Core laser engine for data transmission
pub struct LaserEngine {
    pub config: LaserConfig,
    pub rx_config: ReceptionConfig,
    pub optical_ecc: OpticalECC,
    pub range_detector: Option<Arc<Mutex<RangeDetector>>>,
    pub is_initialized: bool,
    pub current_power_mw: f32,
    pub modulation_scheme: ModulationScheme,
    pub rs_codec: ReedSolomon<reed_solomon_erasure::galois_8::Field>,
}

impl LaserEngine {
    /// Create a new laser engine with configuration
    pub fn new(config: LaserConfig, rx_config: ReceptionConfig) -> Self {
        let rs_codec = ReedSolomon::new(16, 4)
            .map_err(|e| LaserError::HardwareInit(format!("RS codec creation failed: {:?}", e)))
            .unwrap(); // This is safe as parameters are known good

        Self {
            config: config.clone(),
            rx_config,
            optical_ecc: OpticalECC::new(Default::default()),
            range_detector: None,
            is_initialized: false,
            current_power_mw: 0.0,
            modulation_scheme: config.modulation_scheme.clone(),
            rs_codec,
        }
    }

    /// Initialize the laser hardware
    pub async fn initialize(&mut self) -> Result<(), LaserError> {
        // Hardware initialization logic would go here
        // For now, just mark as initialized
        self.is_initialized = true;
        Ok(())
    }

    /// Shutdown the laser system
    pub async fn shutdown(&mut self) -> Result<(), LaserError> {
        self.current_power_mw = 0.0;
        self.is_initialized = false;
        Ok(())
    }

    /// Transmit data using the configured modulation scheme
    pub async fn transmit_data(&mut self, data: &[u8]) -> Result<(), LaserError> {
        if !self.is_initialized {
            return Err(LaserError::HardwareInit("Laser not initialized".to_string()));
        }

        // Encode with ECC first
        let encoded_data = self.optical_ecc.encode(data).await
            .map_err(|e| LaserError::EccError(format!("ECC encoding failed: {:?}", e)))?;

        match self.modulation_scheme {
            ModulationScheme::Ook => self.transmit_ook(&encoded_data).await,
            ModulationScheme::Pwm => self.transmit_pwm(&encoded_data).await,
            ModulationScheme::QrProjection => self.transmit_qr_projection(&encoded_data).await,
            ModulationScheme::Fsk => self.transmit_fsk(&encoded_data).await,
            ModulationScheme::Manchester => self.transmit_manchester(&encoded_data).await,
        }
    }

    /// Receive data using the configured reception method
    pub async fn receive_data(&mut self, _timeout_ms: u64) -> Result<Vec<u8>, LaserError> {
        if !self.is_initialized {
            return Err(LaserError::HardwareInit("Laser not initialized".to_string()));
        }

        let raw_data = match self.modulation_scheme {
            ModulationScheme::Ook => self.receive_ook().await,
            ModulationScheme::Pwm => self.receive_pwm().await,
            ModulationScheme::QrProjection => self.receive_qr_projection().await,
            ModulationScheme::Fsk => self.receive_fsk().await,
            ModulationScheme::Manchester => self.receive_manchester().await,
        }?;

        // Decode with ECC
        self.optical_ecc.decode(&raw_data).await
            .map_err(|e| LaserError::EccError(format!("ECC decoding failed: {:?}", e)))
    }

    // Modulation scheme implementations
    async fn transmit_ook(&mut self, data: &[u8]) -> Result<(), LaserError> {
        for &byte in data {
            for bit in 0..8 {
                let is_on = (byte & (1 << (7 - bit))) != 0;
                let intensity = if is_on { 1.0 } else { 0.0 };
                self.set_laser_intensity(intensity).await?;
                // Timing logic would go here
            }
        }
        Ok(())
    }

    async fn receive_ook(&mut self) -> Result<Vec<u8>, LaserError> {
        // OOK reception logic would go here
        // This is a placeholder implementation
        Ok(vec![0xAA, 0x55]) // Dummy received data
    }

    async fn transmit_pwm(&mut self, data: &[u8]) -> Result<(), LaserError> {
        for &byte in data {
            let duty_cycle = byte as f32 / 255.0;
            self.transmit_pwm_byte(duty_cycle).await?;
        }
        Ok(())
    }

    async fn receive_pwm(&mut self) -> Result<Vec<u8>, LaserError> {
        // PWM reception logic would go here
        Ok(vec![0xAA, 0x55]) // Dummy received data
    }

    async fn transmit_qr_projection(&mut self, _data: &[u8]) -> Result<(), LaserError> {
        // QR projection logic would go here
        // This would involve generating QR codes and projecting them
        Ok(())
    }

    async fn receive_qr_projection(&mut self) -> Result<Vec<u8>, LaserError> {
        // QR reception logic would go here
        Ok(vec![0xAA, 0x55]) // Dummy received data
    }

    async fn transmit_fsk(&mut self, _data: &[u8]) -> Result<(), LaserError> {
        // FSK transmission logic would go here
        Ok(())
    }

    async fn receive_fsk(&mut self) -> Result<Vec<u8>, LaserError> {
        // FSK reception logic would go here
        Ok(vec![0xAA, 0x55]) // Dummy received data
    }

    async fn transmit_manchester(&mut self, _data: &[u8]) -> Result<(), LaserError> {
        // Manchester encoding transmission logic would go here
        Ok(())
    }

    async fn receive_manchester(&mut self) -> Result<Vec<u8>, LaserError> {
        // Manchester decoding reception logic would go here
        Ok(vec![0xAA, 0x55]) // Dummy received data
    }

    /// Set laser intensity (0.0 to 1.0)
    pub async fn set_laser_intensity(&mut self, intensity: f32) -> Result<(), LaserError> {
        if intensity < 0.0 || intensity > 1.0 {
            return Err(LaserError::InvalidConfig("Intensity must be between 0.0 and 1.0".to_string()));
        }

        // Safety check
        if self.config.safety_enabled {
            let max_safe_power = self.config.max_power_mw;
            let requested_power = intensity * max_safe_power;

            if requested_power > max_safe_power {
                return Err(LaserError::SafetyViolation("Requested power exceeds safety limits".to_string()));
            }
        }

        self.current_power_mw = intensity * self.config.max_power_mw;
        // Hardware control logic would go here
        Ok(())
    }

    /// Set PWM duty cycle for transmission
    pub async fn transmit_pwm_byte(&self, duty_cycle: f32) -> Result<(), LaserError> {
        if duty_cycle < 0.0 || duty_cycle > 1.0 {
            return Err(LaserError::InvalidConfig("Duty cycle must be between 0.0 and 1.0".to_string()));
        }
        // PWM hardware control logic would go here
        Ok(())
    }

    /// Enable adaptive mode with range detector
    pub fn enable_adaptive_mode(&mut self, range_detector: Arc<Mutex<RangeDetector>>) {
        self.range_detector = Some(range_detector);
    }

    /// Disable adaptive mode
    pub fn disable_adaptive_mode(&mut self) {
        self.range_detector = None;
    }

    /// Get current power consumption
    pub fn get_current_power_consumption(&self) -> f32 {
        self.current_power_mw
    }

    /// Check if laser is active
    pub fn is_active(&self) -> bool {
        self.is_initialized && self.current_power_mw > 0.0
    }

    /// Get channel diagnostics
    pub async fn get_channel_diagnostics(&self) -> ChannelDiagnostics {
        ChannelDiagnostics {
            signal_strength: 0.8, // Placeholder
            alignment_quality: 0.9, // Placeholder
            error_rate: 0.001, // Placeholder
            last_update: tokio::time::Instant::now(),
            alignment_status: AlignmentStatus {
                is_aligned: true,
                beam_position_x: 0.0,
                beam_position_y: 0.0,
                signal_strength: 0.8,
                last_update: tokio::time::Instant::now(),
            },
            detected_failures: vec![], // No failures detected
        }
    }
}

// Placeholder implementations for methods that will be moved to other modules
impl LaserEngine {
    pub async fn encode_with_ecc(&mut self, data: &[u8]) -> Result<Vec<u8>, LaserError> {
        self.optical_ecc.encode(data).await
            .map_err(|e| LaserError::EccError(format!("ECC encoding failed: {:?}", e)))
    }

    pub async fn decode_with_ecc(&mut self, data: &[u8]) -> Result<Vec<u8>, LaserError> {
        self.optical_ecc.decode(data).await
            .map_err(|e| LaserError::EccError(format!("ECC decoding failed: {:?}", e)))
    }
}
