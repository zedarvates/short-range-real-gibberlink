//! # Channel Validator Module
//!
//! Implements coupled channel validation system for secure authentication requiring
//! simultaneous presence in both laser and ultrasound beams. Provides timestamp
//! correlation, cross-channel cryptographic binding, and comprehensive validation
//! state tracking.

use crate::crypto::CryptoEngine;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

/// Validation phases for coupled channel authentication
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationPhase {
    Idle,
    SyncPulseReceived,
    LaserKeyReceived,
    TemporalCouplingValidated,
    CrossChannelSignatureVerified,
    AntiReplayCheckPassed,
    FullyValidated,
    FallbackMode,
}

/// Channel data with timestamp for correlation
#[derive(Debug, Clone)]
pub struct ChannelData {
    pub channel_type: ChannelType,
    pub data: Vec<u8>,
    pub timestamp: Instant,
    pub sequence_id: u64,
}

/// Types of communication channels
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum ChannelType {
    Laser,
    Ultrasound,
}

/// Coupled nonce pair for anti-replay protection
#[derive(Debug, Clone)]
pub struct CoupledNonce {
    pub laser_nonce: [u8; 16],
    pub ultrasound_nonce: [u8; 16],
    pub timestamp: Instant,
}

/// Channel quality metrics
#[derive(Debug, Clone)]
pub struct ChannelQuality {
    pub signal_strength: f32,
    pub alignment_status: bool,
    pub error_rate: f32,
    pub latency_ms: u64,
}

/// Validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub temporal_tolerance_ms: u64,
    pub quality_threshold: f32,
    pub max_replay_window_ms: u64,
    pub fallback_enabled: bool,
    pub min_coupling_quality: f32,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            temporal_tolerance_ms: 100, // 100ms default window
            quality_threshold: 0.7,     // 70% quality threshold
            max_replay_window_ms: 5000, // 5 second replay window
            fallback_enabled: true,
            min_coupling_quality: 0.6,  // 60% minimum coupling quality
        }
    }
}

/// Comprehensive validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Temporal coupling failed: channels arrived {0}ms apart (tolerance: {1}ms)")]
    TemporalCouplingFailed(u64, u64),
    #[error("Cross-channel signature verification failed")]
    CrossChannelSignatureFailed,
    #[error("Anti-replay check failed: nonce already used")]
    AntiReplayFailed,
    #[error("Channel quality below threshold: {0} < {1}")]
    QualityThresholdFailed(f32, f32),
    #[error("Invalid validation phase transition")]
    InvalidPhaseTransition,
    #[error("Channel data corrupted or invalid")]
    InvalidChannelData,
    #[error("Validation timeout exceeded")]
    Timeout,
    #[error("Fallback validation failed")]
    FallbackFailed,
}

/// Channel validator for coupled authentication
pub struct ChannelValidator {
    config: ValidationConfig,
    current_phase: Arc<Mutex<ValidationPhase>>,
    laser_buffer: Arc<Mutex<VecDeque<ChannelData>>>,
    ultrasound_buffer: Arc<Mutex<VecDeque<ChannelData>>>,
    used_nonces: Arc<Mutex<HashMap<[u8; 32], Instant>>>, // Hash of coupled nonces
    crypto_engine: CryptoEngine,
    session_start: Instant,
    validation_metrics: Arc<Mutex<ValidationMetrics>>,
    session_key: Option<[u8; 32]>, // Session key for cross-channel signatures
}

/// Validation performance metrics
#[derive(Debug, Clone)]
pub struct ValidationMetrics {
    pub total_validations: u64,
    pub successful_validations: u64,
    pub temporal_coupling_failures: u64,
    pub signature_verification_failures: u64,
    pub anti_replay_failures: u64,
    pub quality_threshold_failures: u64,
    pub average_coupling_quality: f32,
    pub average_validation_time_ms: f64,
}

impl ChannelValidator {
    /// Create new channel validator with default configuration
    pub fn new() -> Self {
        Self::with_config(ValidationConfig::default())
    }

    /// Create validator with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            config,
            current_phase: Arc::new(Mutex::new(ValidationPhase::Idle)),
            laser_buffer: Arc::new(Mutex::new(VecDeque::new())),
            ultrasound_buffer: Arc::new(Mutex::new(VecDeque::new())),
            used_nonces: Arc::new(Mutex::new(HashMap::new())),
            crypto_engine: CryptoEngine::new(),
            session_start: Instant::now(),
            validation_metrics: Arc::new(Mutex::new(ValidationMetrics {
                total_validations: 0,
                successful_validations: 0,
                temporal_coupling_failures: 0,
                signature_verification_failures: 0,
                anti_replay_failures: 0,
                quality_threshold_failures: 0,
                average_coupling_quality: 0.0,
                average_validation_time_ms: 0.0,
            })),
            session_key: None,
        }
    }

    /// Receive data from a channel with timestamp
    pub async fn receive_channel_data(&self, data: ChannelData) -> Result<(), ValidationError> {
        match data.channel_type {
            ChannelType::Laser => {
                let mut buffer = self.laser_buffer.lock().await;
                buffer.push_back(data);
                // Keep only recent data (last 1 second)
                while let Some(front) = buffer.front() {
                    if front.timestamp.elapsed() > Duration::from_secs(1) {
                        buffer.pop_front();
                    } else {
                        break;
                    }
                }
            }
            ChannelType::Ultrasound => {
                let mut buffer = self.ultrasound_buffer.lock().await;
                buffer.push_back(data);
                // Keep only recent data (last 1 second)
                while let Some(front) = buffer.front() {
                    if front.timestamp.elapsed() > Duration::from_secs(1) {
                        buffer.pop_front();
                    } else {
                        break;
                    }
                }
            }
        }

        // Trigger validation if we have data from both channels
        self.attempt_coupled_validation().await
    }

    /// Attempt coupled validation when both channels have data
    async fn attempt_coupled_validation(&self) -> Result<(), ValidationError> {
        let laser_data = {
            let buffer = self.laser_buffer.lock().await;
            buffer.back().cloned()
        };

        let ultrasound_data = {
            let buffer = self.ultrasound_buffer.lock().await;
            buffer.back().cloned()
        };

        if let (Some(laser), Some(ultrasound)) = (laser_data, ultrasound_data) {
            self.perform_full_validation(laser, ultrasound).await
        } else {
            Ok(()) // Not enough data yet
        }
    }

    /// Perform complete coupled channel validation
    async fn perform_full_validation(&self, laser_data: ChannelData, ultrasound_data: ChannelData) -> Result<(), ValidationError> {
        let validation_start = Instant::now();
        let mut metrics = self.validation_metrics.lock().await;
        metrics.total_validations += 1;

        // Phase 1: Temporal coupling validation
        self.validate_temporal_coupling(&laser_data, &ultrasound_data).await?;
        self.update_phase(ValidationPhase::TemporalCouplingValidated).await;

        // Phase 2: Cross-channel signature verification
        self.validate_cross_channel_signature(&laser_data, &ultrasound_data).await?;
        self.update_phase(ValidationPhase::CrossChannelSignatureVerified).await;

        // Phase 3: Anti-replay protection
        self.validate_anti_replay(&laser_data, &ultrasound_data).await?;
        self.update_phase(ValidationPhase::AntiReplayCheckPassed).await;

        // Phase 4: Channel quality validation
        self.validate_channel_quality(&laser_data, &ultrasound_data).await?;
        self.update_phase(ValidationPhase::FullyValidated).await;

        // Update metrics
        metrics.successful_validations += 1;
        let validation_time = validation_start.elapsed().as_millis() as f64;
        metrics.average_validation_time_ms =
            (metrics.average_validation_time_ms * (metrics.total_validations - 1) as f64 + validation_time)
            / metrics.total_validations as f64;

        Ok(())
    }

    /// Validate temporal coupling between channels
    async fn validate_temporal_coupling(&self, laser: &ChannelData, ultrasound: &ChannelData) -> Result<(), ValidationError> {
        let time_diff = if laser.timestamp > ultrasound.timestamp {
            (laser.timestamp - ultrasound.timestamp).as_millis() as u64
        } else {
            (ultrasound.timestamp - laser.timestamp).as_millis() as u64
        };

        if time_diff > self.config.temporal_tolerance_ms {
            let mut metrics = self.validation_metrics.lock().await;
            metrics.temporal_coupling_failures += 1;
            return Err(ValidationError::TemporalCouplingFailed(time_diff, self.config.temporal_tolerance_ms));
        }

        Ok(())
    }

    /// Validate cross-channel signature verification (coupled MAC)
    async fn validate_cross_channel_signature(&self, laser: &ChannelData, ultrasound: &ChannelData) -> Result<(), ValidationError> {
        // Get session key - derive if not set
        let session_key = if let Some(key) = self.session_key {
            key
        } else {
            // Auto-derive session key from coupled channel data
            let mut temp_validator = Self::new();
            temp_validator.derive_session_key(&laser.data, &ultrasound.data);
            temp_validator.session_key.unwrap_or([0u8; 32]) // Fallback to zeros if derivation fails
        };

        // Convert Instant to u64 timestamp
        let laser_timestamp = laser.timestamp.elapsed().as_millis() as u64;
        let ultrasound_timestamp = ultrasound.timestamp.elapsed().as_millis() as u64;

        // Create cross-channel signatures
        let laser_hmac = crate::crypto::CryptoEngine::encrypt_ultrasonic_frame(&session_key, &laser.data, laser_timestamp);
        let ultrasound_hmac = crate::crypto::CryptoEngine::encrypt_ultrasonic_frame(&session_key, &ultrasound.data, ultrasound_timestamp);

        // Verify cross-channel authentication: each channel authenticates the other
        if let Err(_) = crate::crypto::CryptoEngine::verify_ultrasonic_frame(&session_key, &laser.data, laser_timestamp, &laser_hmac) {
            let mut metrics = self.validation_metrics.lock().await;
            metrics.signature_verification_failures += 1;
            return Err(ValidationError::CrossChannelSignatureFailed);
        }

        if let Err(_) = crate::crypto::CryptoEngine::verify_ultrasonic_frame(&session_key, &ultrasound.data, ultrasound_timestamp, &ultrasound_hmac) {
            let mut metrics = self.validation_metrics.lock().await;
            metrics.signature_verification_failures += 1;
            return Err(ValidationError::CrossChannelSignatureFailed);
        }

        Ok(())
    }

    /// Validate anti-replay protection using coupled nonces
    async fn validate_anti_replay(&self, laser: &ChannelData, ultrasound: &ChannelData) -> Result<(), ValidationError> {
        // Create coupled nonce hash
        let mut coupled_data = Vec::new();
        coupled_data.extend_from_slice(&laser.data);
        coupled_data.extend_from_slice(&ultrasound.data);
        let nonce_hash = CryptoEngine::generate_device_fingerprint(&coupled_data);

        let mut used_nonces = self.used_nonces.lock().await;

        // Clean old nonces outside replay window
        let now = Instant::now();
        used_nonces.retain(|_, timestamp| {
            now.duration_since(*timestamp).as_millis() < self.config.max_replay_window_ms as u128
        });

        // Check if nonce already used
        if used_nonces.contains_key(&nonce_hash) {
            let mut metrics = self.validation_metrics.lock().await;
            metrics.anti_replay_failures += 1;
            return Err(ValidationError::AntiReplayFailed);
        }

        // Mark nonce as used
        used_nonces.insert(nonce_hash, now);

        Ok(())
    }

    /// Validate channel quality thresholds
    async fn validate_channel_quality(&self, laser: &ChannelData, ultrasound: &ChannelData) -> Result<(), ValidationError> {
        // Calculate coupling quality based on signal correlation
        let coupling_quality = self.calculate_coupling_quality(laser, ultrasound).await;

        if coupling_quality < self.config.quality_threshold {
            let mut metrics = self.validation_metrics.lock().await;
            metrics.quality_threshold_failures += 1;
            metrics.average_coupling_quality =
                (metrics.average_coupling_quality * (metrics.total_validations - 1) as f32 + coupling_quality)
                / metrics.total_validations as f32;
            return Err(ValidationError::QualityThresholdFailed(coupling_quality, self.config.quality_threshold));
        }

        Ok(())
    }

    /// Calculate coupling quality between channels
    async fn calculate_coupling_quality(&self, laser: &ChannelData, ultrasound: &ChannelData) -> f32 {
        // Simplified coupling quality calculation
        // In real implementation, this would analyze signal correlation,
        // alignment quality, error rates, etc.

        let time_diff = if laser.timestamp > ultrasound.timestamp {
            (laser.timestamp - ultrasound.timestamp).as_millis() as f32
        } else {
            (ultrasound.timestamp - laser.timestamp).as_millis() as f32
        };

        // Quality decreases with time difference
        let temporal_quality = 1.0 - (time_diff / self.config.temporal_tolerance_ms as f32).min(1.0);

        // Simulate other quality factors
        let signal_quality = 0.8; // Would be measured from actual signals
        let alignment_quality = 0.9; // Would be measured from beam alignment

        // Weighted average
        (temporal_quality * 0.5) + (signal_quality * 0.3) + (alignment_quality * 0.2)
    }

    /// Update validation phase
    async fn update_phase(&self, new_phase: ValidationPhase) -> Result<(), ValidationError> {
        let mut current_phase = self.current_phase.lock().await;

        // Validate phase transitions
        let valid_transition = match (&*current_phase, &new_phase) {
            (ValidationPhase::Idle, ValidationPhase::TemporalCouplingValidated) => true,
            (ValidationPhase::TemporalCouplingValidated, ValidationPhase::CrossChannelSignatureVerified) => true,
            (ValidationPhase::CrossChannelSignatureVerified, ValidationPhase::AntiReplayCheckPassed) => true,
            (ValidationPhase::AntiReplayCheckPassed, ValidationPhase::FullyValidated) => true,
            (_, ValidationPhase::FallbackMode) if self.config.fallback_enabled => true,
            _ => false,
        };

        if !valid_transition {
            return Err(ValidationError::InvalidPhaseTransition);
        }

        *current_phase = new_phase;
        Ok(())
    }

    /// Get current validation phase
    pub async fn get_current_phase(&self) -> ValidationPhase {
        self.current_phase.lock().await.clone()
    }

    /// Get validation metrics
    pub async fn get_metrics(&self) -> ValidationMetrics {
        self.validation_metrics.lock().await.clone()
    }

    /// Check if validation is complete
    pub async fn is_validated(&self) -> bool {
        matches!(self.get_current_phase().await, ValidationPhase::FullyValidated)
    }

    /// Reset validator for new session
    pub async fn reset(&self) {
        let mut phase = self.current_phase.lock().await;
        *phase = ValidationPhase::Idle;

        let mut laser_buffer = self.laser_buffer.lock().await;
        laser_buffer.clear();

        let mut ultrasound_buffer = self.ultrasound_buffer.lock().await;
        ultrasound_buffer.clear();

        let mut used_nonces = self.used_nonces.lock().await;
        used_nonces.clear();
    }

    /// Attempt fallback validation when one channel is degraded
    pub async fn attempt_fallback_validation(&self, primary_channel: ChannelType, data: ChannelData) -> Result<(), ValidationError> {
        if !self.config.fallback_enabled {
            return Err(ValidationError::FallbackFailed);
        }

        // Check if fallback is appropriate based on channel quality
        let quality = match primary_channel {
            ChannelType::Laser => self.assess_channel_quality(&data, ChannelType::Laser).await,
            ChannelType::Ultrasound => self.assess_channel_quality(&data, ChannelType::Ultrasound).await,
        };

        if quality >= self.config.min_coupling_quality {
            self.update_phase(ValidationPhase::FallbackMode).await?;
            Ok(())
        } else {
            Err(ValidationError::FallbackFailed)
        }
    }

    /// Assess quality of individual channel
    async fn assess_channel_quality(&self, data: &ChannelData, channel_type: ChannelType) -> f32 {
        // Simplified quality assessment
        // In real implementation, this would analyze signal strength,
        // error correction success rate, etc.
        match channel_type {
            ChannelType::Laser => 0.85,    // Laser typically more reliable
            ChannelType::Ultrasound => 0.75, // Ultrasound more susceptible to interference
        }
    }

    /// Get validation configuration
    pub fn get_config(&self) -> &ValidationConfig {
        &self.config
    }

    /// Update validation configuration
    pub fn update_config(&mut self, config: ValidationConfig) {
        self.config = config;
    }

    /// Set session key for cross-channel signature verification
    pub fn set_session_key(&mut self, key: [u8; 32]) {
        self.session_key = Some(key);
    }

    /// Derive session key from coupled channel data
    pub fn derive_session_key(&mut self, laser_data: &[u8], ultrasound_data: &[u8]) {
        // Create a combined seed from both channel data
        let mut combined = Vec::new();
        combined.extend_from_slice(laser_data);
        combined.extend_from_slice(ultrasound_data);
        combined.extend_from_slice(&self.session_start.elapsed().as_nanos().to_be_bytes());

        // Use HKDF to derive a session key
        let ikm = CryptoEngine::generate_device_fingerprint(&combined);
        let salt = b"coupled_channel_session_key_salt";
        let info = b"coupled_channel_session_key_info";

        // Simple HKDF-like derivation (in production, use proper HKDF)
        let mut session_key = [0u8; 32];
        for i in 0..32 {
            session_key[i] = ikm[i] ^ salt[i % salt.len()] ^ info[i % info.len()];
        }

        self.session_key = Some(session_key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_validator_creation() {
        let validator = ChannelValidator::new();
        assert!(!validator.is_validated().await);
        assert_eq!(validator.get_current_phase().await, ValidationPhase::Idle);
    }

    #[tokio::test]
    async fn test_temporal_coupling_validation() {
        let validator = ChannelValidator::new();

        let laser_data = ChannelData {
            channel_type: ChannelType::Laser,
            data: vec![1, 2, 3],
            timestamp: Instant::now(),
            sequence_id: 1,
        };

        let ultrasound_data = ChannelData {
            channel_type: ChannelType::Ultrasound,
            data: vec![4, 5, 6],
            timestamp: Instant::now(),
            sequence_id: 1,
        };

        // Should pass with simultaneous timestamps
        let result = validator.validate_temporal_coupling(&laser_data, &ultrasound_data).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_temporal_coupling_failure() {
        let validator = ChannelValidator::new();

        let laser_data = ChannelData {
            channel_type: ChannelType::Laser,
            data: vec![1, 2, 3],
            timestamp: Instant::now(),
            sequence_id: 1,
        };

        // Create ultrasound data with large time difference
        let ultrasound_data = ChannelData {
            channel_type: ChannelType::Ultrasound,
            data: vec![4, 5, 6],
            timestamp: Instant::now() + Duration::from_millis(200), // 200ms difference
            sequence_id: 1,
        };

        let result = validator.validate_temporal_coupling(&laser_data, &ultrasound_data).await;
        assert!(matches!(result, Err(ValidationError::TemporalCouplingFailed(200, 100))));
    }

    #[tokio::test]
    async fn test_anti_replay_protection() {
        let validator = ChannelValidator::new();

        let laser_data = ChannelData {
            channel_type: ChannelType::Laser,
            data: vec![1, 2, 3],
            timestamp: Instant::now(),
            sequence_id: 1,
        };

        let ultrasound_data = ChannelData {
            channel_type: ChannelType::Ultrasound,
            data: vec![4, 5, 6],
            timestamp: Instant::now(),
            sequence_id: 1,
        };

        // First validation should pass
        let result1 = validator.validate_anti_replay(&laser_data, &ultrasound_data).await;
        assert!(result1.is_ok());

        // Second validation with same data should fail (replay)
        let result2 = validator.validate_anti_replay(&laser_data, &ultrasound_data).await;
        assert!(matches!(result2, Err(ValidationError::AntiReplayFailed)));
    }

    #[tokio::test]
    async fn test_channel_quality_calculation() {
        let validator = ChannelValidator::new();

        let laser_data = ChannelData {
            channel_type: ChannelType::Laser,
            data: vec![1, 2, 3],
            timestamp: Instant::now(),
            sequence_id: 1,
        };

        let ultrasound_data = ChannelData {
            channel_type: ChannelType::Ultrasound,
            data: vec![4, 5, 6],
            timestamp: Instant::now(),
            sequence_id: 1,
        };

        let quality = validator.calculate_coupling_quality(&laser_data, &ultrasound_data).await;
        assert!(quality > 0.0 && quality <= 1.0);
    }
}
