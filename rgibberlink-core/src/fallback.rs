//! # Fallback Manager Module
//!
//! Implements automatic fallback to short-range mode when long-range channels fail.
//! Monitors channel health, detects failures, and manages graceful protocol switching
//! while preserving session state and cryptographic keys.

use crate::laser::{LaserEngine, LaserError, AlignmentStatus};
use crate::ultrasonic_beam::{UltrasonicBeamEngine, UltrasonicBeamError};
use crate::protocol::{ProtocolEngine, ProtocolState, ProtocolError, CommunicationMode};
use crate::crypto::CryptoEngine;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use std::collections::VecDeque;

/// Types of channel failures that can trigger fallback
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelFailure {
    LaserBlocked,
    LaserAlignmentLost,
    LaserHardwareFailure,
    UltrasoundObstructed,
    UltrasoundInterference,
    UltrasoundHardwareFailure,
    EnvironmentalConditions,
    RangeExceeded,
    HardwareTimeout,
}

/// Fallback operation modes
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackMode {
    Automatic,    // Automatic fallback on failure detection
    Manual,       // Manual fallback only
    Disabled,     // No fallback allowed
}

/// Fallback status for user notifications
#[derive(Debug, Clone)]
pub struct FallbackStatus {
    pub active: bool,
    pub current_mode: CommunicationMode,
    pub failure_reason: Option<ChannelFailure>,
    pub fallback_time: Option<Instant>,
    pub recovery_attempts: u32,
    pub last_recovery_attempt: Option<Instant>,
    pub session_snapshot: Option<SessionSnapshot>,
}

/// Channel health metrics
#[derive(Debug, Clone)]
pub struct ChannelHealth {
    pub laser_signal_strength: f32,
    pub laser_alignment_status: bool,
    pub ultrasound_signal_strength: f32,
    pub ultrasound_presence_detected: bool,
    pub overall_health_score: f32, // 0.0 to 1.0
    pub last_update: Instant,
}

/// Fallback configuration
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    pub mode: FallbackMode,
    pub health_check_interval_ms: u64,
    pub failure_threshold: f32, // Health score threshold for failure (0.0-1.0)
    pub recovery_retry_interval_ms: u64,
    pub max_recovery_attempts: u32,
    pub graceful_degradation_timeout_ms: u64,
    pub session_preservation_enabled: bool,
    pub user_notifications_enabled: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            mode: FallbackMode::Automatic,
            health_check_interval_ms: 1000, // 1 second
            failure_threshold: 0.3,         // 30% health triggers fallback
            recovery_retry_interval_ms: 5000, // 5 seconds
            max_recovery_attempts: 5,
            graceful_degradation_timeout_ms: 2000, // 2 seconds
            session_preservation_enabled: true,
            user_notifications_enabled: true,
        }
    }
}

/// Comprehensive fallback errors
#[derive(Debug, thiserror::Error)]
pub enum FallbackError {
    #[error("Fallback mode disabled")]
    FallbackDisabled,
    #[error("Channel health monitoring failed: {0}")]
    HealthMonitoringFailed(String),
    #[error("Protocol switching failed: {0}")]
    ProtocolSwitchFailed(String),
    #[error("Session preservation failed")]
    SessionPreservationFailed,
    #[error("Recovery attempt failed: {0}")]
    RecoveryFailed(String),
    #[error("Maximum recovery attempts exceeded")]
    MaxRecoveryAttemptsExceeded,
    #[error("Invalid fallback state transition")]
    InvalidStateTransition,
}

/// Session state snapshot for preservation during fallback
#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    pub session_id: [u8; 16],
    pub shared_secret: Option<[u8; 32]>,
    pub peer_public_key: Option<Vec<u8>>,
    pub protocol_state: ProtocolState,
    pub communication_mode: CommunicationMode,
    pub crypto_state: Vec<u8>, // Serialized crypto state
    pub timestamp: Instant,
}

/// Fallback manager for automatic channel switching
pub struct FallbackManager {
    config: FallbackConfig,
    laser_engine: Option<Arc<Mutex<LaserEngine>>>,
    ultrasound_engine: Option<Arc<Mutex<UltrasonicBeamEngine>>>,
    protocol_engine: Arc<Mutex<ProtocolEngine>>,
    current_health: Arc<Mutex<ChannelHealth>>,
    fallback_status: Arc<Mutex<FallbackStatus>>,
    session_snapshot: Arc<Mutex<Option<SessionSnapshot>>>,
    failure_history: Arc<Mutex<VecDeque<(ChannelFailure, Instant)>>>,
    recovery_task_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    health_monitor_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl FallbackManager {
    /// Create new fallback manager with default configuration
    pub fn new(protocol_engine: Arc<Mutex<ProtocolEngine>>) -> Self {
        Self::with_config(FallbackConfig::default(), protocol_engine)
    }

    /// Create fallback manager with custom configuration
    pub fn with_config(config: FallbackConfig, protocol_engine: Arc<Mutex<ProtocolEngine>>) -> Self {
        let now = Instant::now();

        Self {
            config,
            laser_engine: None,
            ultrasound_engine: None,
            protocol_engine,
            current_health: Arc::new(Mutex::new(ChannelHealth {
                laser_signal_strength: 1.0,
                laser_alignment_status: true,
                ultrasound_signal_strength: 1.0,
                ultrasound_presence_detected: true,
                overall_health_score: 1.0,
                last_update: now,
            })),
            fallback_status: Arc::new(Mutex::new(FallbackStatus {
                active: false,
                current_mode: CommunicationMode::Auto,
                failure_reason: None,
                fallback_time: None,
                recovery_attempts: 0,
                last_recovery_attempt: None,
                session_snapshot: None,
            })),
            session_snapshot: Arc::new(Mutex::new(None)),
            failure_history: Arc::new(Mutex::new(VecDeque::with_capacity(10))),
            recovery_task_handle: Arc::new(Mutex::new(None)),
            health_monitor_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize fallback manager with channel engines
    pub fn initialize_engines(
        &mut self,
        laser_engine: Option<Arc<Mutex<LaserEngine>>>,
        ultrasound_engine: Option<Arc<Mutex<UltrasonicBeamEngine>>>,
    ) {
        self.laser_engine = laser_engine;
        self.ultrasound_engine = ultrasound_engine;
    }

    /// Start automatic health monitoring and fallback management
    pub async fn start(&self) -> Result<(), FallbackError> {
        if self.config.mode == FallbackMode::Disabled {
            return Err(FallbackError::FallbackDisabled);
        }

        // Start health monitoring task
        self.start_health_monitoring().await?;

        // Start recovery monitoring if in fallback mode
        let status = self.fallback_status.lock().await;
        if status.active {
            drop(status);
            self.start_recovery_monitoring().await?;
        }

        Ok(())
    }

    /// Stop all monitoring and recovery tasks
    pub async fn stop(&self) -> Result<(), FallbackError> {
        // Stop health monitoring
        if let Some(handle) = self.health_monitor_handle.lock().await.take() {
            handle.abort();
        }

        // Stop recovery monitoring
        if let Some(handle) = self.recovery_task_handle.lock().await.take() {
            handle.abort();
        }

        Ok(())
    }

    /// Start continuous health monitoring
    async fn start_health_monitoring(&self) -> Result<(), FallbackError> {
        let health_arc = Arc::clone(&self.current_health);
        let fallback_status_arc = Arc::clone(&self.fallback_status);
        let config = self.config.clone();
        let laser_engine = self.laser_engine.clone();
        let ultrasound_engine = self.ultrasound_engine.clone();
        let protocol_engine = Arc::clone(&self.protocol_engine);
        let failure_history = Arc::clone(&self.failure_history);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(config.health_check_interval_ms));

            loop {
                interval.tick().await;

                // Assess channel health
                let health_result = Self::assess_channel_health(
                    &laser_engine,
                    &ultrasound_engine,
                    &protocol_engine,
                ).await;

                match health_result {
                    Ok(health) => {
                        *health_arc.lock().await = health.clone();

                        // Check if fallback is needed
                        if health.overall_health_score < config.failure_threshold {
                            let failure_reason = Self::determine_failure_reason(&health);
                            if let Some(reason) = failure_reason {
                                // Record failure
                                let mut history = failure_history.lock().await;
                                history.push_back((reason.clone(), Instant::now()));
                                if history.len() > 10 {
                                    history.pop_front();
                                }

                                // Trigger fallback if not already active
                                let mut status = fallback_status_arc.lock().await;
                                if !status.active && config.mode == FallbackMode::Automatic {
                                    drop(status);
                                    if let Err(e) = Self::trigger_fallback(
                                        &protocol_engine,
                                        reason,
                                        &config,
                                        &fallback_status_arc,
                                        &laser_engine,
                                        &ultrasound_engine,
                                    ).await {
                                        eprintln!("Fallback trigger failed: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Health assessment failed: {:?}", e);
                    }
                }
            }
        });

        *self.health_monitor_handle.lock().await = Some(handle);
        Ok(())
    }

    /// Assess current channel health
    async fn assess_channel_health(
        laser_engine: &Option<Arc<Mutex<LaserEngine>>>,
        ultrasound_engine: &Option<Arc<Mutex<UltrasonicBeamEngine>>>,
        protocol_engine: &Arc<Mutex<ProtocolEngine>>,
    ) -> Result<ChannelHealth, FallbackError> {
        let mut health = ChannelHealth {
            laser_signal_strength: 0.0,
            laser_alignment_status: false,
            ultrasound_signal_strength: 0.0,
            ultrasound_presence_detected: false,
            overall_health_score: 0.0,
            last_update: Instant::now(),
        };

        // Assess laser health using detailed diagnostics
        if let Some(laser) = laser_engine {
            let laser_lock = laser.lock().await;
            if laser_lock.is_active().await {
                let diagnostics = laser_lock.get_channel_diagnostics().await;

                health.laser_alignment_status = diagnostics.alignment_status.is_aligned;
                health.laser_signal_strength = diagnostics.alignment_status.signal_strength;

                // Reduce score based on detected failures
                let failure_penalty = diagnostics.detected_failures.len() as f32 * 0.2;
                health.laser_signal_strength = (health.laser_signal_strength - failure_penalty).max(0.0);

                // Additional penalties for specific failures
                for failure in &diagnostics.detected_failures {
                    match failure {
                        LaserError::AlignmentLost => {
                            health.laser_signal_strength *= 0.3; // Severe penalty for alignment loss
                        }
                        LaserError::SafetyViolation => {
                            health.laser_signal_strength *= 0.1; // Critical penalty for safety issues
                        }
                        LaserError::HardwareUnavailable => {
                            health.laser_signal_strength = 0.0; // Complete failure
                        }
                        _ => {
                            health.laser_signal_strength *= 0.8; // Moderate penalty for other issues
                        }
                    }
                }
            }
        }

        // Assess ultrasound health using detailed diagnostics
        if let Some(ultrasound) = ultrasound_engine {
            let ultrasound_lock = ultrasound.lock().await;
            if ultrasound_lock.is_active() {
                let diagnostics = ultrasound_lock.get_channel_diagnostics().await;

                health.ultrasound_presence_detected = diagnostics.presence_detected;
                health.ultrasound_signal_strength = if diagnostics.presence_detected { 0.8 } else { 0.1 };

                // Reduce score based on detected failures
                let failure_penalty = diagnostics.detected_failures.len() as f32 * 0.15;
                health.ultrasound_signal_strength = (health.ultrasound_signal_strength - failure_penalty).max(0.0);

                // Additional penalties for specific failures
                for failure in &diagnostics.detected_failures {
                    match failure {
                        UltrasonicBeamError::HardwareUnavailable => {
                            health.ultrasound_signal_strength = 0.0; // Complete failure
                        }
                        UltrasonicBeamError::PresenceDetectionError => {
                            health.ultrasound_signal_strength *= 0.2; // Severe penalty for detection failure
                        }
                        UltrasonicBeamError::RangeOutOfBounds(_) => {
                            health.ultrasound_signal_strength *= 0.5; // Moderate penalty for range issues
                        }
                        _ => {
                            health.ultrasound_signal_strength *= 0.9; // Minor penalty for other issues
                        }
                    }
                }
            }
        }

        // Calculate overall health score with dynamic weighting
        let laser_weight = if laser_engine.is_some() { 0.6 } else { 0.0 };
        let ultrasound_weight = if ultrasound_engine.is_some() { 0.4 } else { 0.0 };

        health.overall_health_score =
            health.laser_signal_strength * laser_weight +
            health.ultrasound_signal_strength * ultrasound_weight;

        // Environmental and protocol state factors
        let protocol_state_bonus = match protocol_engine.lock().await.get_state().await {
            crate::protocol::ProtocolState::LongRangeConnected => 0.1, // Bonus for stable connection
            crate::protocol::ProtocolState::Connected => 0.05,         // Smaller bonus for short-range
            _ => 0.0, // No bonus for unstable states
        };

        health.overall_health_score = (health.overall_health_score + protocol_state_bonus).min(1.0);

        Ok(health)
    }

    /// Determine the primary failure reason from health assessment
    fn determine_failure_reason(health: &ChannelHealth) -> Option<ChannelFailure> {
        // Prioritize failures by severity
        if health.laser_signal_strength < 0.3 && !health.laser_alignment_status {
            Some(ChannelFailure::LaserAlignmentLost)
        } else if health.laser_signal_strength < 0.2 {
            Some(ChannelFailure::LaserBlocked)
        } else if health.ultrasound_signal_strength < 0.3 && !health.ultrasound_presence_detected {
            Some(ChannelFailure::UltrasoundObstructed)
        } else if health.ultrasound_signal_strength < 0.2 {
            Some(ChannelFailure::UltrasoundInterference)
        } else if health.overall_health_score < 0.4 {
            Some(ChannelFailure::EnvironmentalConditions)
        } else {
            None
        }
    }

    /// Trigger fallback to short-range mode
    async fn trigger_fallback(
        protocol_engine: &Arc<Mutex<ProtocolEngine>>,
        failure_reason: ChannelFailure,
        config: &FallbackConfig,
        fallback_status: &Arc<Mutex<FallbackStatus>>,
        laser_engine: &Option<Arc<Mutex<LaserEngine>>>,
        ultrasound_engine: &Option<Arc<Mutex<UltrasonicBeamEngine>>>,
    ) -> Result<(), FallbackError> {
        // Preserve session state before fallback
        Self::preserve_session_state(protocol_engine, fallback_status).await?;

        // Switch protocol to short-range mode
        {
            let mut protocol = protocol_engine.lock().await;
            protocol.set_mode(CommunicationMode::ShortRange).await
                .map_err(|e| FallbackError::ProtocolSwitchFailed(e.to_string()))?;
        }

        // Update fallback status
        {
            let mut status = fallback_status.lock().await;
            status.active = true;
            status.current_mode = CommunicationMode::ShortRange;
            status.failure_reason = Some(failure_reason.clone());
            status.fallback_time = Some(Instant::now());
            status.recovery_attempts = 0;
        }

        // Send user notification if enabled
        if config.user_notifications_enabled {
            Self::send_fallback_notification(&failure_reason).await;
        }

        // Start recovery monitoring
        Self::start_recovery_monitoring_internal(protocol_engine, config, fallback_status, laser_engine, ultrasound_engine).await?;

        Ok(())
    }

    /// Preserve session state before fallback
    async fn preserve_session_state(
        protocol_engine: &Arc<Mutex<ProtocolEngine>>,
        fallback_status: &Arc<Mutex<FallbackStatus>>,
    ) -> Result<(), FallbackError> {
        let protocol = protocol_engine.lock().await;

        // Serialize crypto state for preservation
        let crypto_state = Self::serialize_crypto_state(&protocol).await;

        let snapshot = SessionSnapshot {
            session_id: *protocol.get_session_id(),
            shared_secret: protocol.get_shared_secret().copied(),
            peer_public_key: protocol.get_peer_public_key().cloned(),
            protocol_state: protocol.get_state().await,
            communication_mode: protocol.get_mode().clone(),
            crypto_state,
            timestamp: Instant::now(),
        };

        {
            let mut status = fallback_status.lock().await;
            status.session_snapshot = Some(snapshot);
        }
        Ok(())
    }

    /// Serialize crypto state for session preservation
    async fn serialize_crypto_state(protocol: &ProtocolEngine) -> Vec<u8> {
        use serde::Serialize;

        #[derive(Serialize)]
        struct CryptoStateSnapshot {
            session_id: [u8; 16],
            shared_secret: Option<[u8; 32]>,
            peer_public_key: Option<Vec<u8>>,
            protocol_state: crate::protocol::ProtocolState,
            communication_mode: crate::protocol::CommunicationMode,
        }

        let snapshot = CryptoStateSnapshot {
            session_id: *protocol.get_session_id(),
            shared_secret: protocol.get_shared_secret().copied(),
            peer_public_key: protocol.get_peer_public_key().cloned(),
            protocol_state: protocol.get_state().await,
            communication_mode: protocol.get_mode().clone(),
        };

        // Serialize to CBOR for compact binary format
        serde_cbor::to_vec(&snapshot).unwrap_or_default()
    }

    /// Restore session state after recovery
    async fn restore_session_state(
        protocol_engine: &Arc<Mutex<ProtocolEngine>>,
        snapshot: &SessionSnapshot,
    ) -> Result<(), FallbackError> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct CryptoStateSnapshot {
            session_id: [u8; 16],
            shared_secret: Option<[u8; 32]>,
            peer_public_key: Option<Vec<u8>>,
            protocol_state: crate::protocol::ProtocolState,
            communication_mode: crate::protocol::CommunicationMode,
        }

        // Deserialize crypto state
        if let Ok(state) = serde_cbor::from_slice::<CryptoStateSnapshot>(&snapshot.crypto_state) {
            let mut protocol = protocol_engine.lock().await;

            // Restore session parameters using setter methods
            protocol.set_session_id(state.session_id);
            protocol.set_shared_secret(state.shared_secret);
            protocol.set_peer_public_key(state.peer_public_key);
            protocol.set_state(state.protocol_state).await;
            protocol.set_communication_mode(state.communication_mode);
        }

        Ok(())
    }

    /// Send user notification about fallback event
    async fn send_fallback_notification(failure_reason: &ChannelFailure) {
        // In a real implementation, this would send notifications through
        // the application's notification system (Android notifications, UI updates, etc.)
        let message = match failure_reason {
            ChannelFailure::LaserBlocked => "Laser communication blocked - switched to short-range mode",
            ChannelFailure::LaserAlignmentLost => "Laser alignment lost - switched to short-range mode",
            ChannelFailure::UltrasoundObstructed => "Ultrasound signal obstructed - switched to short-range mode",
            ChannelFailure::EnvironmentalConditions => "Poor environmental conditions - switched to short-range mode",
            _ => "Communication channel failure - switched to short-range mode",
        };

        println!("FALLBACK NOTIFICATION: {}", message);
        // TODO: Integrate with actual notification system
    }

    /// Start recovery monitoring to attempt long-range restoration
    async fn start_recovery_monitoring(&self) -> Result<(), FallbackError> {
        let config = self.config.clone();
        let protocol_engine = Arc::clone(&self.protocol_engine);
        let fallback_status = Arc::clone(&self.fallback_status);
        let laser_engine = self.laser_engine.clone();
        let ultrasound_engine = self.ultrasound_engine.clone();

        let handle = tokio::spawn(async move {
            Self::start_recovery_monitoring_internal(&protocol_engine, &config, &fallback_status, &laser_engine, &ultrasound_engine).await.unwrap_or_else(|e| {
                eprintln!("Recovery monitoring failed to start: {:?}", e);
            });
        });

        *self.recovery_task_handle.lock().await = Some(handle);
        Ok(())
    }

    /// Internal recovery monitoring implementation
    async fn start_recovery_monitoring_internal(
        protocol_engine: &Arc<Mutex<ProtocolEngine>>,
        config: &FallbackConfig,
        fallback_status: &Arc<Mutex<FallbackStatus>>,
        laser_engine: &Option<Arc<Mutex<LaserEngine>>>,
        ultrasound_engine: &Option<Arc<Mutex<UltrasonicBeamEngine>>>,
    ) -> Result<(), FallbackError> {
        let mut interval = tokio::time::interval(Duration::from_millis(config.recovery_retry_interval_ms));

        loop {
            interval.tick().await;

            let mut status = fallback_status.lock().await;
            if !status.active || status.recovery_attempts >= config.max_recovery_attempts {
                break;
            }

            status.recovery_attempts += 1;
            status.last_recovery_attempt = Some(Instant::now());

            // Attempt to assess if long-range channels are now healthy
            let health_result = Self::assess_channel_health(
                laser_engine,
                ultrasound_engine,
                protocol_engine,
            ).await;

            if let Ok(health) = health_result {
                if health.overall_health_score >= 0.7 { // Recovery threshold
                    // Attempt to restore long-range mode
                    drop(status);
                    if let Err(e) = Self::attempt_recovery(protocol_engine, config, fallback_status).await {
                        eprintln!("Recovery attempt failed: {:?}", e);
                    } else {
                        break; // Recovery successful
                    }
                }
            }
        }

        Ok(())
    }

    /// Attempt to recover to long-range mode
    async fn attempt_recovery(
        protocol_engine: &Arc<Mutex<ProtocolEngine>>,
        config: &FallbackConfig,
        fallback_status: &Arc<Mutex<FallbackStatus>>,
    ) -> Result<(), FallbackError> {
        // Restore session state from snapshot if available
        {
            let status = fallback_status.lock().await;
            if let Some(snapshot) = &status.session_snapshot {
                Self::restore_session_state(protocol_engine, snapshot).await?;
            }
        }

        // Switch back to long-range mode
        {
            let mut protocol = protocol_engine.lock().await;
            protocol.set_mode(CommunicationMode::LongRange).await
                .map_err(|e| FallbackError::RecoveryFailed(e.to_string()))?;
        }

        // Update fallback status
        {
            let mut status = fallback_status.lock().await;
            status.active = false;
            status.current_mode = CommunicationMode::LongRange;
            status.failure_reason = None;
            status.fallback_time = None;
        }

        // Send recovery notification
        if config.user_notifications_enabled {
            println!("RECOVERY NOTIFICATION: Restored long-range communication");
        }

        Ok(())
    }

    /// Get current fallback status
    pub async fn get_fallback_status(&self) -> FallbackStatus {
        self.fallback_status.lock().await.clone()
    }

    /// Get current channel health
    pub async fn get_channel_health(&self) -> ChannelHealth {
        self.current_health.lock().await.clone()
    }

    /// Manually trigger fallback (for testing or manual control)
    pub async fn manual_fallback(&self, reason: ChannelFailure) -> Result<(), FallbackError> {
        if self.config.mode == FallbackMode::Disabled {
            return Err(FallbackError::FallbackDisabled);
        }

        Self::trigger_fallback(
            &self.protocol_engine,
            reason,
            &self.config,
            &self.fallback_status,
            &self.laser_engine,
            &self.ultrasound_engine,
        ).await
    }

    /// Get failure history
    pub async fn get_failure_history(&self) -> Vec<(ChannelFailure, Instant)> {
        self.failure_history.lock().await.iter().cloned().collect()
    }

    /// Update fallback configuration
    pub fn update_config(&mut self, config: FallbackConfig) {
        self.config = config;
    }

    /// Check if fallback is currently active
    pub async fn is_fallback_active(&self) -> bool {
        self.fallback_status.lock().await.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fallback_manager_creation() {
        let protocol_engine = Arc::new(Mutex::new(ProtocolEngine::new()));
        let manager = FallbackManager::new(protocol_engine);

        let status = manager.get_fallback_status().await;
        assert!(!status.active);
        assert_eq!(status.current_mode, CommunicationMode::Auto);
    }

    #[tokio::test]
    async fn test_health_assessment() {
        let protocol_engine = Arc::new(Mutex::new(ProtocolEngine::new()));

        let health = FallbackManager::assess_channel_health(
            &None,
            &None,
            &protocol_engine,
        ).await.unwrap();

        // With no engines, health should be poor
        assert!(health.overall_health_score < 0.5);
    }

    #[tokio::test]
    async fn test_failure_reason_detection() {
        let health = ChannelHealth {
            laser_signal_strength: 0.1,
            laser_alignment_status: false,
            ultrasound_signal_strength: 0.8,
            ultrasound_presence_detected: true,
            overall_health_score: 0.2,
            last_update: Instant::now(),
        };

        let reason = FallbackManager::determine_failure_reason(&health);
        assert_eq!(reason, Some(ChannelFailure::LaserAlignmentLost));
    }
}