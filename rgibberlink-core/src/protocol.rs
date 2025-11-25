use crate::audio::AudioEngine;
use crate::crypto::CryptoEngine;
use crate::visual::{VisualEngine, VisualPayload};
use crate::ultrasonic_beam::{UltrasonicBeamEngine, UltrasonicBeamError};
use crate::laser::{LaserEngine, LaserError, LaserConfig, ReceptionConfig};
use crate::channel_validator::{ChannelValidator, ChannelData, ChannelType, ValidationError};
use crate::fallback::{FallbackManager, FallbackConfig, FallbackStatus, ChannelHealth, ChannelFailure};
use crate::performance_monitor::{PerformanceMonitor, PerformanceMetrics, PerformanceConfig, PerformancePreset, EnvironmentalFactors};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use rand::RngCore;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CommunicationMode {
    ShortRange,       // Original ultrasonic + QR
    LongRange,        // Laser + focused ultrasound
    NoisyEnvironment, // Multi-band ultrasonic harmonics + QR compensation
    Auto,             // Automatic mode selection
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ProtocolState {
    Idle,
    // Short-range states
    SendingNonce,
    WaitingForQr,
    SendingAck,
    Connected,
    // Long-range states
    LongRangeSync,
    LongRangeKeyExchange,
    LongRangeAuth,
    LongRangeConnected,
    // Security-enhanced states
    SecurityValidation,
    PermissionCheck,
    HumanApprovalRequired,
    RiskAssessment,
    CommandSafeguardsActive,
    SecureChannelEstablished,
    // Long-range with security
    LongRangeSecuritySync,
    LongRangeAuthWithValidation,
    LongRangeCommandSafeguards,
    LongRangeSecureChannel,
    // Fallback states
    FallbackToShortRange,
    Error(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Audio transmission failed: {0}")]
    AudioError(String),
    #[error("Visual generation failed: {0}")]
    VisualError(String),
    #[error("Crypto operation failed: {0}")]
    CryptoError(String),
    #[error("Protocol timeout")]
    Timeout,
    #[error("Invalid state transition")]
    InvalidState,
    #[error("Ultrasonic beam error: {0}")]
    UltrasonicBeamError(#[from] UltrasonicBeamError),
    #[error("Laser transmission error: {0}")]
    LaserError(#[from] LaserError),
    #[error("Coupled channel validation failed")]
    CoupledChannelValidationFailed,
    #[error("Channel validator error: {0}")]
    ChannelValidatorError(#[from] ValidationError),
    #[error("Long-range channel unavailable")]
    LongRangeChannelUnavailable,
    #[error("Fallback to short-range mode")]
    FallbackToShortRange,
}

pub struct ProtocolEngine {
    state: Arc<Mutex<ProtocolState>>,
    mode: CommunicationMode,
    crypto: CryptoEngine,
    audio: AudioEngine,
    visual: VisualEngine,
    ultrasonic_beam: Option<UltrasonicBeamEngine>,
    laser: Option<LaserEngine>,
    channel_validator: Option<ChannelValidator>,
    fallback_manager: Option<FallbackManager>,
    performance_monitor: Option<PerformanceMonitor>,
    session_id: [u8; 16],
    peer_public_key: Option<Vec<u8>>,
    shared_secret: Option<[u8; 32]>,
    // Long-range specific fields
    coupled_validation_required: bool,
    timeout_duration: Duration,
    retry_count: u32,
    max_retries: u32,
    last_activity: Instant,
    // Performance monitoring
    performance_enabled: bool,
    last_performance_check: Instant,
    performance_check_interval: Duration,
}

impl ProtocolEngine {
    pub fn new() -> Self {
        let mut session_id = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut session_id);

        // Initialize audio engine
        let mut audio_engine = AudioEngine::new();
        // Note: We can't call async initialize here, so we'll initialize on first use
        // For now, we'll assume it's initialized for testing
        audio_engine.force_initialize_for_testing();

        Self {
            state: Arc::new(Mutex::new(ProtocolState::Idle)),
            mode: CommunicationMode::Auto,
            crypto: CryptoEngine::new(),
            audio: audio_engine,
            visual: VisualEngine::new(),
            ultrasonic_beam: None,
            laser: None,
            channel_validator: None,
            fallback_manager: None,
            performance_monitor: None,
            session_id,
            peer_public_key: None,
            shared_secret: None,
            coupled_validation_required: true,
            timeout_duration: Duration::from_secs(30),
            retry_count: 0,
            max_retries: 3,
            last_activity: Instant::now(),
            performance_enabled: false,
            last_performance_check: Instant::now(),
            performance_check_interval: Duration::from_millis(500), // Check every 500ms
        }
    }

    /// Create protocol engine with specific communication mode
    pub fn with_mode(mode: CommunicationMode) -> Self {
        let mut engine = Self::new();
        engine.mode = mode;
        engine
    }

    /// Initialize long-range engines if needed
    pub async fn initialize_long_range(&mut self) -> Result<(), ProtocolError> {
        if self.mode == CommunicationMode::LongRange || self.mode == CommunicationMode::Auto {
            // Initialize ultrasonic beam engine
            let mut ultrasonic = UltrasonicBeamEngine::new();
            ultrasonic.initialize().await
                .map_err(|e| ProtocolError::UltrasonicBeamError(e))?;
            self.ultrasonic_beam = Some(ultrasonic);

            // Initialize laser engine
            let laser_config = LaserConfig::default();
            let rx_config = ReceptionConfig::default();
            let mut laser = LaserEngine::new(laser_config, rx_config);
            laser.initialize().await
                .map_err(|e| ProtocolError::LaserError(e))?;
            self.laser = Some(laser);

            // Initialize channel validator for coupled validation
            self.channel_validator = Some(ChannelValidator::new());
        }
        Ok(())
    }

    /// Initialize noisy environment engines (multi-band ultrasonic only)
    pub async fn initialize_noisy_environment(&mut self) -> Result<(), ProtocolError> {
        if self.mode == CommunicationMode::NoisyEnvironment || self.mode == CommunicationMode::Auto {
            // Initialize multi-band ultrasonic beam engine
            let beam_config = crate::ultrasonic_beam::BeamConfig::default(); // Multi-band config
            let mut ultrasonic = UltrasonicBeamEngine::with_config(beam_config)
                .map_err(|e| ProtocolError::UltrasonicBeamError(e))?;
            ultrasonic.initialize().await
                .map_err(|e| ProtocolError::UltrasonicBeamError(e))?;
            self.ultrasonic_beam = Some(ultrasonic);

            // No laser in noisy environment mode - use visual compensation
            // Initialize validator for visual-ultrasonic coupling
            self.channel_validator = Some(ChannelValidator::new());
        }
        Ok(())
    }

    /// Set communication mode
    pub async fn set_mode(&mut self, mode: CommunicationMode) -> Result<(), ProtocolError> {
        self.mode = mode;
        if self.mode == CommunicationMode::LongRange {
            self.initialize_long_range().await?;
        } else if self.mode == CommunicationMode::NoisyEnvironment {
            self.initialize_noisy_environment().await?;
        } else if self.mode == CommunicationMode::Auto {
            // Auto mode initializes both engines
            self.initialize_long_range().await?;
            self.initialize_noisy_environment().await?;
        }
        Ok(())
    }

    /// Get current communication mode
    pub fn get_mode(&self) -> &CommunicationMode {
        &self.mode
    }

    pub async fn initiate_handshake(&mut self) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        if !matches!(*state, ProtocolState::Idle) {
            return Err(ProtocolError::InvalidState);
        }

        *state = ProtocolState::SendingNonce;

        // Generate and send nonce via audio
        let nonce = CryptoEngine::generate_nonce();
        self.audio.send_data(&nonce).await.map_err(|e| ProtocolError::AudioError(e.to_string()))?;

        *state = ProtocolState::WaitingForQr;
        Ok(())
    }

    pub async fn receive_nonce(&self, nonce: &[u8]) -> Result<String, ProtocolError> {
        let mut state = self.state.lock().await;
        if !matches!(*state, ProtocolState::Idle) {
            return Err(ProtocolError::InvalidState);
        }

        *state = ProtocolState::WaitingForQr;

        // Generate QR payload
        let payload = VisualPayload {
            session_id: self.session_id,
            public_key: self.crypto.public_key().to_vec(),
            nonce: nonce.try_into().map_err(|_| ProtocolError::CryptoError("Invalid nonce length".to_string()))?,
            signature: vec![], // Simplified for prototype
        };

        let qr_svg = self.visual.encode_payload(&payload).map_err(|e| ProtocolError::VisualError(e.to_string()))?;
        Ok(qr_svg)
    }

    pub async fn process_qr_payload(&mut self, qr_data: &[u8]) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        if !matches!(*state, ProtocolState::WaitingForQr) {
            return Err(ProtocolError::InvalidState);
        }

        let payload = self.visual.decode_payload(qr_data).map_err(|e| ProtocolError::VisualError(e.to_string()))?;

        // Verify session ID matches
        if payload.session_id != self.session_id {
            return Err(ProtocolError::CryptoError("Session ID mismatch".to_string()));
        }

        // Derive shared secret first, then move the key
        let shared_secret = self.crypto.derive_shared_secret(&payload.public_key)
            .map_err(|e| ProtocolError::CryptoError(e.to_string()))?;

        self.peer_public_key = Some(payload.public_key);
        self.shared_secret = Some(shared_secret);

        *state = ProtocolState::SendingAck;

        // Send ACK via audio
        let ack_data = b"ACK";
        self.audio.send_data(ack_data).await.map_err(|e| ProtocolError::AudioError(e.to_string()))?;

        *state = ProtocolState::Connected;
        Ok(())
    }

    pub async fn receive_ack(&self) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        if !matches!(*state, ProtocolState::WaitingForQr) {
            return Err(ProtocolError::InvalidState);
        }

        *state = ProtocolState::Connected;
        Ok(())
    }

    pub async fn get_state(&self) -> ProtocolState {
        self.state.lock().await.clone()
    }

    pub fn get_shared_secret(&self) -> Option<&[u8; 32]> {
        self.shared_secret.as_ref()
    }

    /// Get session ID (for fallback manager)
    pub fn get_session_id(&self) -> &[u8; 16] {
        &self.session_id
    }

    /// Get peer public key (for fallback manager)
    pub fn get_peer_public_key(&self) -> Option<&Vec<u8>> {
        self.peer_public_key.as_ref()
    }

    /// Set session ID (for fallback restoration)
    pub fn set_session_id(&mut self, session_id: [u8; 16]) {
        self.session_id = session_id;
    }

    /// Set shared secret (for fallback restoration)
    pub fn set_shared_secret(&mut self, secret: Option<[u8; 32]>) {
        self.shared_secret = secret;
    }

    /// Set peer public key (for fallback restoration)
    pub fn set_peer_public_key(&mut self, key: Option<Vec<u8>>) {
        self.peer_public_key = key;
    }

    /// Set protocol state (for fallback restoration)
    pub async fn set_state(&self, new_state: ProtocolState) {
        *self.state.lock().await = new_state;
    }

    /// Set communication mode (for fallback restoration)
    pub fn set_communication_mode(&mut self, mode: CommunicationMode) {
        self.mode = mode;
    }

    /// Enable fallback management with custom configuration
    pub fn enable_fallback(&mut self, config: FallbackConfig) -> Result<(), ProtocolError> {
        let protocol_arc = Arc::new(Mutex::new(self.clone()));
        let mut fallback = FallbackManager::with_config(config, protocol_arc);

        // Initialize with available engines (take ownership for fallback)
        let laser_arc = self.laser.take().map(|l| Arc::new(Mutex::new(l)));
        let ultrasound_arc = self.ultrasonic_beam.take().map(|u| Arc::new(Mutex::new(u)));
        fallback.initialize_engines(laser_arc, ultrasound_arc);

        self.fallback_manager = Some(fallback);
        Ok(())
    }

    /// Enable fallback management with default configuration
    pub fn enable_fallback_default(&mut self) -> Result<(), ProtocolError> {
        self.enable_fallback(FallbackConfig::default())
    }

    /// Start fallback monitoring (must be called after enabling fallback)
    pub async fn start_fallback_monitoring(&self) -> Result<(), ProtocolError> {
        if let Some(fallback) = &self.fallback_manager {
            fallback.start().await
                .map_err(|e| ProtocolError::CryptoError(format!("Fallback start failed: {:?}", e)))?;
        }
        Ok(())
    }

    /// Get fallback status
    pub async fn get_fallback_status(&self) -> Option<FallbackStatus> {
        if let Some(fallback) = &self.fallback_manager {
            Some(fallback.get_fallback_status().await)
        } else {
            None
        }
    }

    /// Get channel health metrics
    pub async fn get_channel_health(&self) -> Option<ChannelHealth> {
        if let Some(fallback) = &self.fallback_manager {
            Some(fallback.get_channel_health().await)
        } else {
            None
        }
    }

    /// Manually trigger fallback (for testing or manual control)
    pub async fn manual_fallback(&self, reason: ChannelFailure) -> Result<(), ProtocolError> {
        if let Some(fallback) = &self.fallback_manager {
            fallback.manual_fallback(reason).await
                .map_err(|e| ProtocolError::CryptoError(format!("Manual fallback failed: {:?}", e)))?;
        }
        Ok(())
    }

    /// Check if fallback is currently active
    pub async fn is_fallback_active(&self) -> bool {
        if let Some(fallback) = &self.fallback_manager {
            fallback.is_fallback_active().await
        } else {
            false
        }
    }

    /// Initiate optimized long-range handshake (initiator side) - target <500ms
    pub async fn initiate_long_range_handshake(&mut self) -> Result<(), ProtocolError> {
        let handshake_start = Instant::now();
        let mut state = self.state.lock().await;
        if !matches!(*state, ProtocolState::Idle) {
            return Err(ProtocolError::InvalidState);
        }

        // Check if long-range engines are available
        if self.ultrasonic_beam.is_none() || self.laser.is_none() {
            return Err(ProtocolError::InvalidState);
        }

        *state = ProtocolState::LongRangeSync;
        self.last_activity = Instant::now();

        // OPTIMIZATION: Fast sequential sync with pre-computed data
        let _nonce = CryptoEngine::generate_nonce();
        let session_id = self.session_id;

        // Phase 1: Fast ultrasonic sync pulse (optimized for speed)
        if let Some(ultrasonic) = &self.ultrasonic_beam {
            // Use pre-optimized sync pattern for <50ms transmission
            ultrasonic.transmit_sync_pulse(&session_id).await
                .map_err(|e| ProtocolError::UltrasonicBeamError(e))?;
        }

        // Phase 1b: Immediate laser key transmission (parallel preparation)
        // Send public key immediately after sync for reduced round trips
        if let Some(laser) = &mut self.laser {
            let public_key = self.crypto.public_key();
            laser.transmit_data(public_key).await
                .map_err(|e| ProtocolError::LaserError(e))?;
        }

        *state = ProtocolState::LongRangeKeyExchange;

        // Log timing for optimization
        let sync_time = handshake_start.elapsed().as_millis() as f64;
        if self.performance_enabled {
            // Would log to performance monitor
            println!("Fast sync completed in {}ms", sync_time);
        }

        Ok(())
    }

    /// Create optimized sync packet for fast handshake
    fn create_fast_sync_packet(&self, nonce: &[u8], session_id: &[u8; 16]) -> Vec<u8> {
        // Compress sync data for faster transmission
        // Include: session_id (16B) + nonce (32B) + timestamp (8B) = 56B total
        let mut packet = Vec::with_capacity(64);
        packet.extend_from_slice(session_id);
        packet.extend_from_slice(nonce);
        packet.extend_from_slice(&self.last_activity.elapsed().as_millis().to_le_bytes());

        // Add checksum for error detection (4B)
        let checksum = crc32fast::hash(&packet);
        packet.extend_from_slice(&checksum.to_le_bytes());

        packet
    }

    /// Receive long-range sync pulse (receiver side)
    pub async fn receive_long_range_sync(&mut self, sync_pattern: &[u8]) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        if !matches!(*state, ProtocolState::Idle) {
            return Err(ProtocolError::InvalidState);
        }

        // Verify sync pattern matches session ID
        if sync_pattern != self.session_id {
            return Err(ProtocolError::CryptoError("Invalid sync pattern".to_string()));
        }

        *state = ProtocolState::LongRangeKeyExchange;
        self.last_activity = Instant::now();
        Ok(())
    }

    /// Send public key via laser (receiver side)
    pub async fn send_public_key_via_laser(&mut self) -> Result<(), ProtocolError> {
        let state = self.state.lock().await;
        if !matches!(*state, ProtocolState::LongRangeKeyExchange) {
            return Err(ProtocolError::InvalidState);
        }

        if let Some(laser) = &mut self.laser {
            let public_key = self.crypto.public_key();
            laser.transmit_data(public_key).await
                .map_err(|e| ProtocolError::LaserError(e))?;
        } else {
            return Err(ProtocolError::LongRangeChannelUnavailable);
        }

        Ok(())
    }

    /// Receive public key via laser (initiator side)
    pub async fn receive_public_key_via_laser(&mut self, timeout_ms: u64) -> Result<Vec<u8>, ProtocolError> {
        let state = self.state.lock().await;
        if !matches!(*state, ProtocolState::LongRangeKeyExchange) {
            return Err(ProtocolError::InvalidState);
        }

        if let Some(laser) = &mut self.laser {
            let public_key = laser.receive_data(timeout_ms).await
                .map_err(|e| ProtocolError::LaserError(e))?;
            Ok(public_key)
        } else {
            Err(ProtocolError::LongRangeChannelUnavailable)
        }
    }

    /// Perform coupled channel validation and send ACK
    pub async fn perform_coupled_validation(&mut self, laser_public_key: &[u8]) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        if !matches!(*state, ProtocolState::LongRangeKeyExchange) {
            return Err(ProtocolError::InvalidState);
        }

        // Store peer public key
        self.peer_public_key = Some(laser_public_key.to_vec());

        // Derive shared secret
        let shared_secret = self.crypto.derive_shared_secret(laser_public_key)
            .map_err(|e| ProtocolError::CryptoError(e.to_string()))?;
        self.shared_secret = Some(shared_secret);

        // Use ChannelValidator for coupled validation if available
        if let Some(validator) = &self.channel_validator {
            // Create channel data for laser reception
            let laser_data = ChannelData {
                channel_type: ChannelType::Laser,
                data: laser_public_key.to_vec(),
                timestamp: Instant::now(),
                sequence_id: 1, // Would be properly sequenced in real implementation
            };

            // Receive laser data into validator
            validator.receive_channel_data(laser_data).await
                .map_err(|_| ProtocolError::CoupledChannelValidationFailed)?;

            // Check if validation is complete
            if validator.is_validated().await {
                *state = ProtocolState::LongRangeAuth;
            } else {
                return Err(ProtocolError::CoupledChannelValidationFailed);
            }
        } else if self.coupled_validation_required {
            // Fallback to basic presence detection if no validator
            if let Some(ultrasonic) = &self.ultrasonic_beam {
                let presence_detected = ultrasonic.detect_presence().await
                    .map_err(|e| ProtocolError::UltrasonicBeamError(e))?;

                if !presence_detected {
                    return Err(ProtocolError::CoupledChannelValidationFailed);
                }
            }
            *state = ProtocolState::LongRangeAuth;
        }

        // Send ACK via ultrasonic beam (coupled with laser validation)
        if let Some(ultrasonic) = &self.ultrasonic_beam {
            let ack_data = b"LONG_RANGE_ACK";
            ultrasonic.transmit_control_data(ack_data, 1).await
                .map_err(|e| ProtocolError::UltrasonicBeamError(e))?;
        }

        *state = ProtocolState::LongRangeConnected;
        self.last_activity = Instant::now();
        Ok(())
    }

    /// Receive coupled ACK (receiver side)
    pub async fn receive_coupled_ack(&mut self, ack_data: &[u8], sequence_id: u64) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        if !matches!(*state, ProtocolState::LongRangeAuth) {
            return Err(ProtocolError::InvalidState);
        }

        // Use ChannelValidator if available
        if let Some(validator) = &self.channel_validator {
            // Receive ultrasonic ACK data
            self.receive_ultrasonic_data(ack_data, sequence_id).await?;

            // Check if validation is complete
            if validator.is_validated().await {
                *state = ProtocolState::LongRangeConnected;
                self.last_activity = Instant::now();
                Ok(())
            } else {
                Err(ProtocolError::CoupledChannelValidationFailed)
            }
        } else {
            // Fallback: basic ACK reception
            if let Some(_ultrasonic) = &self.ultrasonic_beam {
                // In real implementation, this would verify the ACK data
                *state = ProtocolState::LongRangeConnected;
                self.last_activity = Instant::now();
                Ok(())
            } else {
                Err(ProtocolError::LongRangeChannelUnavailable)
            }
        }
    }

    /// Check for timeout and handle retries/fallback
    pub async fn check_timeout_and_retry(&mut self) -> Result<(), ProtocolError> {
        let elapsed = self.last_activity.elapsed();

        if elapsed > self.timeout_duration {
            if self.retry_count < self.max_retries {
                // Retry the current operation
                self.retry_count += 1;
                // Reset state to retry
                let mut state = self.state.lock().await;
                *state = match *state {
                    ProtocolState::LongRangeSync => ProtocolState::LongRangeSync,
                    ProtocolState::LongRangeKeyExchange => ProtocolState::LongRangeKeyExchange,
                    ProtocolState::LongRangeAuth => ProtocolState::LongRangeAuth,
                    _ => ProtocolState::Idle,
                };
                self.last_activity = Instant::now();
                Ok(())
            } else {
                // Max retries exceeded, fallback to short-range
                let mut state = self.state.lock().await;
                *state = ProtocolState::FallbackToShortRange;
                Err(ProtocolError::FallbackToShortRange)
            }
        } else {
            Ok(())
        }
    }

    /// Receive ultrasonic data for coupled validation
    pub async fn receive_ultrasonic_data(&self, data: &[u8], sequence_id: u64) -> Result<(), ProtocolError> {
        if let Some(validator) = &self.channel_validator {
            let ultrasonic_data = ChannelData {
                channel_type: ChannelType::Ultrasound,
                data: data.to_vec(),
                timestamp: Instant::now(),
                sequence_id,
            };

            validator.receive_channel_data(ultrasonic_data).await?;
        }
        Ok(())
    }

    /// Get channel quality metrics
    pub async fn get_channel_quality(&mut self) -> Result<ChannelQuality, ProtocolError> {
        let mut quality = ChannelQuality {
            ultrasonic_signal_strength: 0.0,
            laser_alignment_status: false,
            overall_quality: 0.0,
        };

        if let Some(_ultrasonic) = &self.ultrasonic_beam {
            // In real implementation, get actual signal strength
            quality.ultrasonic_signal_strength = 0.8; // Mock value
        }

        if let Some(laser) = &self.laser {
            let alignment = laser.get_alignment_status().await;
            quality.laser_alignment_status = alignment.is_aligned;
        }

        quality.overall_quality = if quality.laser_alignment_status {
            (quality.ultrasonic_signal_strength + 1.0) / 2.0
        } else {
            quality.ultrasonic_signal_strength / 2.0
        };

        Ok(quality)
    }

    /// Enable performance monitoring and optimization
    pub fn enable_performance_monitoring(&mut self, _config: PerformanceConfig) -> Result<(), ProtocolError> {
        self.performance_monitor = Some(PerformanceMonitor::new(100));
        self.performance_enabled = true;
        // Note: In real implementation, apply the config to the monitor
        Ok(())
    }

    /// Disable performance monitoring
    pub fn disable_performance_monitoring(&mut self) {
        self.performance_monitor = None;
        self.performance_enabled = false;
    }

    /// Set performance preset
    pub async fn set_performance_preset(&mut self, preset: PerformancePreset) -> Result<(), ProtocolError> {
        if let Some(_monitor) = &self.performance_monitor {
            // Apply preset configuration
            // This would adjust laser power, modulation schemes, ECC strength, etc.
            match preset {
                PerformancePreset::SpeedOptimized => {
                    // Prioritize speed: higher data rates, minimal ECC
                    if let Some(_laser) = &self.laser {
                        // Would set high-speed configuration
                    }
                }
                PerformancePreset::ReliabilityOptimized => {
                    // Prioritize reliability: stronger ECC, robust modulation
                    if let Some(_laser) = &self.laser {
                        // Would set robust configuration
                    }
                }
                PerformancePreset::PowerOptimized => {
                    // Minimize power consumption
                    if let Some(_laser) = &self.laser {
                        // Would set low-power configuration
                    }
                }
                PerformancePreset::Balanced => {
                    // Good balance of all factors
                    // Default configuration
                }
                PerformancePreset::LongRangeOptimized => {
                    // Optimized for maximum range
                    if let Some(_laser) = &self.laser {
                        // Would set long-range optimized configuration
                    }
                }
                PerformancePreset::LowLatency => {
                    // Minimize handshake time
                    if let Some(_laser) = &self.laser {
                        // Would set low-latency configuration
                    }
                }
                PerformancePreset::HighBandwidth => {
                    // Maximize data throughput
                    if let Some(_laser) = &self.laser {
                        // Would set high-bandwidth configuration
                    }
                }
                PerformancePreset::Custom(_config) => {
                    // Apply custom configuration
                    // Would apply config settings
                }
            }
        }
        Ok(())
    }

    /// Perform real-time performance check and adjustment
    pub async fn perform_performance_check(&mut self) -> Result<(), ProtocolError> {
        if !self.performance_enabled || self.performance_monitor.is_none() {
            return Ok(());
        }

        let now = Instant::now();
        if now.duration_since(self.last_performance_check) < self.performance_check_interval {
            return Ok(());
        }

        self.last_performance_check = now;

        // Collect current performance metrics
        let metrics = self.collect_performance_metrics().await?;

        // Store metrics in monitor
        if let Some(monitor) = &self.performance_monitor {
            monitor.record_metrics(metrics.clone()).await;
        }

        // Perform automatic adjustments based on metrics
        self.perform_automatic_adjustments(metrics).await?;

        Ok(())
    }

    /// Collect current performance metrics
    async fn collect_performance_metrics(&mut self) -> Result<PerformanceMetrics, ProtocolError> {
        let channel_quality = self.get_channel_quality().await?;

        let (power_consumption, data_rate, modulation_scheme) = if let Some(laser) = &self.laser {
            let power = laser.get_current_power_consumption().await;
            let profile = laser.get_current_power_profile().await;
            let modulation = laser.select_optimal_modulation().await;
            (power, profile.data_rate_bps as f64, modulation)
        } else {
            (0.0, 1_000_000.0, crate::laser::ModulationScheme::Ook)
        };

        let range_meters = if let Some(laser) = &self.laser {
            // Get range from laser's monitoring status
            let (_is_adaptive, category) = laser.get_monitoring_status().await;
            if let Some(cat) = category {
                match cat {
                    crate::range_detector::RangeDetectorCategory::Close => 75.0,
                    crate::range_detector::RangeDetectorCategory::Medium => 125.0,
                    crate::range_detector::RangeDetectorCategory::Far => 150.0,
                    crate::range_detector::RangeDetectorCategory::Extreme => 190.0,
                }
            } else {
                100.0
            }
        } else {
            0.0
        };

        Ok(PerformanceMetrics {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            handshake_latency_ms: 250.0, // Would be measured from actual handshakes
            data_throughput_bps: data_rate,
            bit_error_rate: 0.001, // Would be measured from actual transmission
            packet_loss_rate: 0.0,
            power_consumption_mw: power_consumption as f64,
            range_meters,
            signal_strength: channel_quality.ultrasonic_signal_strength as f64,
            modulation_scheme,
            ecc_strength: 0.5, // Would be measured from ECC configuration
            environmental_conditions: EnvironmentalFactors::default(),
        })
    }

    /// Perform automatic performance adjustments
    async fn perform_automatic_adjustments(&mut self, metrics: PerformanceMetrics) -> Result<(), ProtocolError> {
        // Check if adjustments are needed based on metrics

        // Adjust power consumption if too high
        if metrics.power_consumption_mw > 100.0 {
            if let Some(laser) = &mut self.laser {
                // Reduce power consumption
                let battery_state = None; // Would get actual battery state
                laser.optimize_power_usage(battery_state).await
                    .map_err(|e| ProtocolError::LaserError(e))?;
            }
        }

        // Adjust modulation scheme if throughput is too low
        if metrics.data_throughput_bps < 500_000.0 {
            if let Some(laser) = &mut self.laser {
                laser.update_modulation_scheme().await
                    .map_err(|e| ProtocolError::LaserError(e))?;
            }
        }

        // Adjust ECC strength if error rate is too high
        if metrics.bit_error_rate > 0.01 {
            if let Some(laser) = &mut self.laser {
                laser.update_ecc_for_range().await
                    .map_err(|e| ProtocolError::LaserError(e))?;
            }
        }

        Ok(())
    }

    /// Get performance recommendations
    pub async fn get_performance_recommendations(&self) -> Vec<String> {
        if let Some(monitor) = &self.performance_monitor {
            monitor.get_recommendations().await
        } else {
            Vec::new()
        }
    }

    /// Update environmental conditions for performance optimization
    pub async fn update_environmental_conditions(&mut self, weather: crate::security::WeatherCondition, visibility_m: f32) -> Result<(), ProtocolError> {
        if let Some(laser) = &self.laser {
            laser.update_environmental_conditions(weather.clone(), visibility_m).await
                .map_err(|e| ProtocolError::LaserError(e))?;
        }

        // Update performance monitor with environmental data
        if let Some(monitor) = &self.performance_monitor {
            let conditions = EnvironmentalFactors {
                weather,
                temperature_celsius: 20.0, // Would get actual temperature
                humidity_percent: 50.0,
                visibility_meters: visibility_m,
                wind_speed_mps: 2.0,
            };
            monitor.update_environmental_factors(conditions).await;
        }

        Ok(())
    }

    /// Get current performance status
    pub async fn get_performance_status(&self) -> Option<PerformanceMetrics> {
        if let Some(monitor) = &self.performance_monitor {
            monitor.get_current_metrics().await
        } else {
            None
        }
    }

    /// Get mutable reference to audio engine (for hierarchical protocol)
    pub fn get_audio_engine_mut(&mut self) -> &mut AudioEngine {
        &mut self.audio
    }

    /// Get mutable reference to laser engine (for hierarchical protocol)
    pub fn get_laser_engine_mut(&mut self) -> Option<&mut LaserEngine> {
        self.laser.as_mut()
    }

    /// Get mutable reference to ultrasonic beam engine (for hierarchical protocol)
    pub fn get_ultrasonic_beam_engine_mut(&mut self) -> Option<&mut UltrasonicBeamEngine> {
        self.ultrasonic_beam.as_mut()
    }

    /// Enable hierarchical protocol mode (integrates with HierarchicalProtocolEngine)
    pub async fn enable_hierarchical_protocol(&mut self, hierarchical_engine: &mut crate::hierarchical::HierarchicalProtocolEngine) -> Result<(), ProtocolError> {
        hierarchical_engine.enable_hierarchy().await?;
        Ok(())
    }

    /// Process hierarchical protocol messages
    pub async fn process_hierarchical_message(&mut self, data: &[u8], hierarchical_engine: &mut crate::hierarchical::HierarchicalProtocolEngine) -> Result<(), ProtocolError> {
        hierarchical_engine.receive_hierarchical_message(data).await?;
        Ok(())
    }

    /// Broadcast rank presence for hierarchical protocol
    pub async fn broadcast_hierarchical_presence(&mut self, hierarchical_engine: &mut crate::hierarchical::HierarchicalProtocolEngine) -> Result<(), ProtocolError> {
        hierarchical_engine.broadcast_rank_presence().await?;
        Ok(())
    }

    /// Send hierarchical command
    pub async fn send_hierarchical_command(
        &mut self,
        target_rank: crate::hierarchical::MilitaryRank,
        command_type: crate::hierarchical::CommandType,
        payload: Vec<u8>,
        require_ack: bool,
        hierarchical_engine: &mut crate::hierarchical::HierarchicalProtocolEngine,
    ) -> Result<u32, ProtocolError> {
        hierarchical_engine.send_command(target_rank, command_type, payload, require_ack).await
    }

    /// Coordinate multi-device command (e.g., cart pushing)
    pub async fn coordinate_multi_device(
        &mut self,
        target_ranks: Vec<crate::hierarchical::MilitaryRank>,
        command: &str,
        hierarchical_engine: &mut crate::hierarchical::HierarchicalProtocolEngine,
    ) -> Result<(), ProtocolError> {
        hierarchical_engine.coordinate_multi_device(target_ranks, command).await
    }

    /// Get current hierarchical state
    pub async fn get_hierarchical_state(&self, hierarchical_engine: &crate::hierarchical::HierarchicalProtocolEngine) -> crate::hierarchical::HierarchicalState {
        hierarchical_engine.get_current_state().await
    }

    /// Get device rank
    pub fn get_device_rank<'a>(&self, hierarchical_engine: &'a crate::hierarchical::HierarchicalProtocolEngine) -> &'a crate::hierarchical::MilitaryRank {
        hierarchical_engine.get_rank()
    }

    /// Check if superior is present in hierarchy
    pub async fn is_superior_present(&self, hierarchical_engine: &crate::hierarchical::HierarchicalProtocolEngine) -> bool {
        hierarchical_engine.is_superior_present().await
    }

    /// Get highest rank currently present
    pub async fn get_highest_rank_present(&self, hierarchical_engine: &crate::hierarchical::HierarchicalProtocolEngine) -> Option<crate::hierarchical::MilitaryRank> {
        hierarchical_engine.get_highest_rank_present().await
    }

    pub async fn encrypt_message(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let state = self.state.lock().await;
        if !matches!(*state, ProtocolState::Connected | ProtocolState::LongRangeConnected) {
            return Err(ProtocolError::InvalidState);
        }

        let key = self.shared_secret.ok_or(ProtocolError::CryptoError("No shared secret".to_string()))?;
        CryptoEngine::encrypt_data(&key, data).map_err(|e| ProtocolError::CryptoError(e.to_string()))
    }

    pub async fn decrypt_message(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let state = self.state.lock().await;
        if !matches!(*state, ProtocolState::Connected | ProtocolState::LongRangeConnected) {
            return Err(ProtocolError::InvalidState);
        }

        let key = self.shared_secret.ok_or(ProtocolError::CryptoError("No shared secret".to_string()))?;
        CryptoEngine::decrypt_data(&key, encrypted_data).map_err(|e| ProtocolError::CryptoError(e.to_string()))
    }
}

/// Channel quality metrics
#[derive(Debug, Clone)]
pub struct ChannelQuality {
    pub ultrasonic_signal_strength: f32, // 0.0 to 1.0
    pub laser_alignment_status: bool,
    pub overall_quality: f32, // 0.0 to 1.0
}

impl Clone for ProtocolEngine {
    fn clone(&self) -> Self {
        // Create a new ProtocolEngine with the same configuration but fresh state
        let mut new_engine = Self::new();
        new_engine.mode = self.mode.clone();
        // Note: We don't copy engines or session state for simplicity
        // In a real implementation, you might want to implement proper cloning
        new_engine
    }
}
